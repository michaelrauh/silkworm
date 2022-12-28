use anyhow::{Context, Ok};

pub trait Registry {
    type Database;
    type Location;
    type GlobalQueueLocation;
    type DataRoute: Clone;
    type JobReceipt;
    type LocalQueue: Clone;
    type Data;

    fn unique_string(&self) -> String;
    fn worker_name(&self) -> String;
    fn create_db(&self) -> Self::Database;
    fn db_location(
        &self,
        worker_name: String,
        random_string: String,
    ) -> Result<Self::Location, anyhow::Error>;
    fn write_db(&self, loc: &Self::Location, db: &Self::Database) -> Result<(), anyhow::Error>;
    fn delete_db(&self, loc: Self::Location) -> Result<(), anyhow::Error>;
    fn read_db(&self, loc: &Self::Location) -> Result<Self::Database, anyhow::Error>;
    fn read_queue(&self, loc: &Self::Location) -> Result<Self::LocalQueue, anyhow::Error>;
    fn create_global_queue(&self) -> Result<Self::GlobalQueueLocation, anyhow::Error>;
    fn consume_global(
        &self,
        queue: &mut Self::GlobalQueueLocation,
    ) -> Result<(Self::DataRoute, Self::JobReceipt), anyhow::Error>;
    fn produce_global(
        &self,
        data: Self::DataRoute,
        queue: &mut Self::GlobalQueueLocation,
        priority: usize,
    ) -> Result<Self::JobReceipt, anyhow::Error>;
    fn ack_global(
        &self,
        queue: &mut Self::GlobalQueueLocation,
        receipt: Self::JobReceipt,
    ) -> Result<(), anyhow::Error>;
    fn create_local_queue(&self) -> Self::LocalQueue;
    fn consume_local(&self, queue: &mut Self::LocalQueue) -> Option<Self::DataRoute>;
    fn produce_local(&self, queue: &mut Self::LocalQueue, loc: Self::DataRoute);
    fn produce_merge_event(
        &self,
        queue: &mut Self::GlobalQueueLocation,
        first_db_loc: Self::Location,
        first_queue_loc: Self::Location,
    ) -> Result<Self::JobReceipt, anyhow::Error>;
    fn consume_merge_event(
        &self,
        queue: &mut Self::GlobalQueueLocation,
    ) -> Result<
        Option<(
            Self::Location,
            Self::Location,
            Self::JobReceipt,
            Self::Location,
            Self::Location,
            Self::JobReceipt,
        )>,
        anyhow::Error,
    >;
    fn collapse_dbs(&self, db: &Self::Database, other: &Self::Database) -> Self::Database;
    fn queue_location(
        &self,
        worker_name: String,
        random_string: String,
    ) -> Result<Self::Location, anyhow::Error>;
    fn write_local_queue(
        &self,
        loc: &Self::Location,
        queue: &Self::LocalQueue,
    ) -> Result<(), anyhow::Error>;
    fn get_data_cycle(
        &self,
        route: Self::DataRoute,
    ) -> Box<dyn DataCycle<Database = Self::Database, DataRoute = Self::DataRoute, Data = Self::Data>>;
}

pub trait DataCycle {
    type Database;
    type DataRoute;
    type Data;

    fn stop_categorically(&self, db: &Self::Database) -> bool;
    fn get_data(&self, db: &Self::Database, route: &Self::DataRoute) -> Option<Self::Data>;
    fn stop_data(&self, data: &Self::Data, db: &Self::Database) -> bool;
    fn get_friends(&self, db: &Self::Database, route: &Self::DataRoute) -> Vec<Self::Data>;
    fn stop_friends(&self, friends: &Vec<Self::Data>) -> bool;
    fn search(&self, data: &Self::Data, friends: &Vec<Self::Data>) -> Vec<Self::Data>;
    fn save(&self, db: &mut Self::Database, new_data: Self::Data) -> Option<Self::DataRoute>;
    fn public(&self) -> bool;
}

pub fn run_worker(reg: impl Registry) -> Result<(), anyhow::Error> {
    let mut global_queue = reg.create_global_queue()?;

    if let Some(merge_event) = reg.consume_merge_event(&mut global_queue)? {
        let (
            first_db_loc,
            first_queue_loc,
            first_merge_rec,
            second_db_loc,
            second_queue_loc,
            second_merge_rec,
        ) = merge_event;

        let first_db = reg.read_db(&first_db_loc)?;
        let mut first_queue = reg.read_queue(&first_queue_loc)?;

        let second_db = reg.read_db(&second_db_loc)?;
        let mut second_queue = reg.read_queue(&second_queue_loc)?;

        let new_db_loc = reg.db_location(reg.worker_name(), reg.unique_string())?;
        let new_db = reg.collapse_dbs(&first_db, &second_db);

        let new_queue_loc = reg.queue_location(reg.worker_name(), reg.unique_string())?;
        let mut new_queue = reg.create_local_queue();

        while let Some(data_route) = reg.consume_local(&mut first_queue) {
            let cycle = reg.get_data_cycle(data_route.clone());
            let first_data_option = cycle.get_data(&first_db, &data_route);
            let second_data_option = cycle.get_data(&second_db, &data_route);

            if first_data_option.is_some() && second_data_option.is_some() {
                reg.produce_local(&mut new_queue, data_route);
                continue;
            }

            let first_friends = cycle.get_friends(&first_db, &data_route);
            let second_friends = cycle.get_friends(&second_db, &data_route);

            if !first_friends.is_empty() && !second_friends.is_empty() {
                reg.produce_local(&mut new_queue, data_route);
                continue;
            }

            if (first_data_option.is_some()
                && second_data_option.is_none()
                && !first_friends.is_empty()
                && second_friends.is_empty())
                || (first_data_option.is_none()
                    && second_data_option.is_some()
                    && first_friends.is_empty()
                    && !second_friends.is_empty())
            {
                reg.produce_local(&mut new_queue, data_route);
                continue;
            }

            // todo play the whole data cycle on first_queue
            reg.produce_local(&mut new_queue, data_route);
        }

        while let Some(data_route) = reg.consume_local(&mut second_queue) {
            let cycle = reg.get_data_cycle(data_route.clone());
            let first_data_option = cycle.get_data(&first_db, &data_route);
            let second_data_option = cycle.get_data(&second_db, &data_route);

            if first_data_option.is_some() && second_data_option.is_some() {
                reg.produce_local(&mut new_queue, data_route);
                continue;
            }

            let first_friends = cycle.get_friends(&first_db, &data_route);
            let second_friends = cycle.get_friends(&second_db, &data_route);

            if !first_friends.is_empty() && !second_friends.is_empty() {
                reg.produce_local(&mut new_queue, data_route);
                continue;
            }

            if (first_data_option.is_some()
                && second_data_option.is_none()
                && !first_friends.is_empty()
                && second_friends.is_empty())
                || (first_data_option.is_none()
                    && second_data_option.is_some()
                    && first_friends.is_empty()
                    && !second_friends.is_empty())
            {
                reg.produce_local(&mut new_queue, data_route);
                continue;
            }

            // todo play the whole data cycle on second_queue
            reg.produce_local(&mut new_queue, data_route);
        }

        reg.write_db(&new_db_loc, &new_db)?;
        reg.write_local_queue(&new_queue_loc, &new_queue)?;

        reg.produce_merge_event(&mut global_queue, new_db_loc, new_queue_loc)?;

        reg.ack_global(&mut global_queue, first_merge_rec)?;
        reg.ack_global(&mut global_queue, second_merge_rec)?;

        reg.delete_db(first_db_loc).expect("panic here if we can't delete the DB. Otherwise it will be too late to come back to this as we have already acked. No, you cant just not ack until after because then it will retry and the DB will be gone");
        reg.delete_db(second_db_loc).expect("panic here if we can't delete the DB. Otherwise it will be too late to come back to this as we have already acked. No, you cant just not ack until after because then it will retry and the DB will be gone");
        return Ok(());
    }

    let name = reg.worker_name();
    let random_string = reg.unique_string();
    let mut db = reg.create_db();
    let db_loc = reg.db_location(name.clone(), random_string)?;

    let (data_route, global_receipt) = reg.consume_global(&mut global_queue)?;

    let mut local_queue = reg.create_local_queue();
    let queue_to_write = local_queue.clone(); // make sure this is getting written back to
    reg.produce_local(&mut local_queue, data_route.clone());

    while let Some(local_route) = reg.consume_local(&mut local_queue) {
        // start datacycle
        let cycle = reg.get_data_cycle(local_route.clone());
        if cycle.stop_categorically(&db) {
            reg.ack_global(&mut global_queue, global_receipt)?;
            return Ok(());
        }

        let data = cycle
            .get_data(&db, &local_route)
            .context("attempting to get data")?;

        if cycle.stop_data(&data, &db) {
            reg.ack_global(&mut global_queue, global_receipt)?;
            return Ok(());
        }

        let friends = cycle.get_friends(&db, &local_route);

        if cycle.stop_friends(&friends) {
            reg.ack_global(&mut global_queue, global_receipt)?;
            return Ok(());
        }

        let results = cycle.search(&data, &friends);

        for result in results { // should this be a for?
            let res = cycle.save(&mut db, result); // is this the right time to save?

            if res.is_none() {
                continue;
            }

            let res = res.unwrap();

            if cycle.public() {
                reg.produce_global(res, &mut global_queue, 0)?; // concern: what if the data route is referring to a route that is not in the local db? global queue will probably need to expose the data instead of the route
            } else {
                reg.produce_local(&mut local_queue, res);
            }
        }
    }

    // end datacycle

    reg.write_db(&db_loc, &db)?;
    let local_queue_location = reg.queue_location(name, reg.unique_string())?;
    reg.write_local_queue(&local_queue_location, &queue_to_write)?;

    reg.produce_merge_event(&mut global_queue, db_loc, local_queue_location)?;
    reg.ack_global(&mut global_queue, global_receipt)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
