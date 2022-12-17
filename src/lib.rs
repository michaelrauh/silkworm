pub trait Registry {
    type Database;
    type Location;
    type GlobalQueueLocation;
    type DataRoute;
    type JobReceipt;
    type LocalQueue: Clone;

    fn worker_name(&self) -> String;
    fn create_db(&self) -> Self::Database;
    fn db_location(&self, worker_name: String) -> Result<Self::Location, anyhow::Error>;
    fn write_db(&self, loc: &Self::Location, db: &mut Self::Database) -> Result<(), anyhow::Error>;
    fn read_db(&self, loc: &Self::Location) -> Result<Self::Database, anyhow::Error>;
    fn read_queue(&self, loc: &Self::Location) -> Result<Self::LocalQueue, anyhow::Error>;
    fn create_global_queue(&self) -> Result<Self::GlobalQueueLocation, anyhow::Error>;
    fn consume_global(&self, queue: Self::GlobalQueueLocation) -> Result<(Self::DataRoute, Self::JobReceipt), anyhow::Error>;
    fn produce_global(&self, data: Self::DataRoute, queue: Self::GlobalQueueLocation, priority: usize) -> Result<Self::JobReceipt, anyhow::Error>;
    fn ack_global(&self, queue: &mut Self::GlobalQueueLocation, receipt: Self::JobReceipt) -> Result<(), anyhow::Error>;
    fn create_local_queue(&self) -> Self::LocalQueue;
    fn consume_local(&self, queue: &mut Self::LocalQueue) -> Option<Self::DataRoute>;
    fn produce_local(&self, queue: Self::LocalQueue, loc: Self::DataRoute);
    fn produce_merge_event(&self, queue: &mut Self::GlobalQueueLocation, first_db_loc: Self::Location, first_queue_loc: Self::Location) -> Result<Self::JobReceipt, anyhow::Error>;
    fn consume_merge_event(&self, queue: &mut Self::GlobalQueueLocation) -> Result<Option<(Self::Location, Self::Location, Self::JobReceipt, Self::Location, Self::Location, Self::JobReceipt)>, anyhow::Error>;
    fn collapse_dbs(&self, db: &mut Self::Database, other: Self::Database);
    fn queue_location(&self, worker_name: String) -> Result<Self::Location, anyhow::Error>;
    fn write_local_queue(&self, loc: Self::Location, queue: Self::LocalQueue) -> Result<(), anyhow::Error>;
    fn get_data_cycle(&self, route: Self::DataRoute) -> Box<dyn DataCycle<Database = Self::Database>>;
    // add a reorder local queue and a reorder frequency or trigger
    // add some concept of queue cleanup or event collapse
}

pub trait DataCycle {
    type Database;
    fn stop_categorically(&self, db: Self::Database) -> bool;
}

fn run_worker(reg: impl Registry, worker: impl DataCycle) -> Result<(), anyhow::Error> {

    let mut merge_queue = reg.create_global_queue()?;
    let mut global_queue = reg.create_global_queue()?;

    if let Some(merge_event) =  reg.consume_merge_event(&mut merge_queue)? {
        let (first_db_loc, first_queue_loc, first_merge_rec, second_db_loc, second_queue_loc, second_merge_rec) = merge_event;
        
        let mut first_db = reg.read_db(&first_db_loc)?;
        let mut first_queue = reg.read_queue(&first_queue_loc)?;

        let second_db = reg.read_db(&second_db_loc)?;
        let second_queue = reg.read_queue(&second_queue_loc)?;

        reg.collapse_dbs(&mut first_db, second_db);


        while let Some(data_route) = reg.consume_local(&mut first_queue) {
            let cycle = reg.get_data_cycle(data_route);
            // get friends off of cycle for that data route
            // if data is on both sides, stop.
            // if friends are on both sides, stop.
            // if data and friends are all on the same side, stop
            // else play the whole data cycle
        }


        // replay first queue
        // replay second queue and add queue events to first queue

        reg.write_db(&first_db_loc, &mut first_db)?;
        // safely delete other db
        // call compact queue to reorder or delete queue items
        // write first queue
        reg.produce_merge_event(&mut global_queue, first_db_loc, first_queue_loc)?;
        reg.ack_global(&mut merge_queue, first_merge_rec)?;
        reg.ack_global(&mut merge_queue, second_merge_rec)?;

        return Ok(())
    }

    let name = reg.worker_name();
    let db = reg.create_db();
    let loc = reg.db_location(name)?;
    

    let (data_route, receipt) = reg.consume_global(global_queue)?; // consume batch size events and repeat batch count times until timeout

    let local_queue = reg.create_local_queue();
    let queue_to_write = local_queue.clone();
    reg.produce_local(local_queue, data_route);

    // find the right datacycle and run it for the data route
    // let (db_update, global_events) = cycle_data(db, worker, local_queue);
    
    let global_events = todo!();
    let db_update = todo!();
    reg.produce_global(global_events, global_queue, 0)?; // this hard coded priority is an issue. Priority will have to be bundled with the event if it is to stay

    // end datacycle
    

    reg.write_db(&loc, db_update)?;
    let local_queue_location = reg.queue_location(name)?;
    reg.write_local_queue(local_queue_location, queue_to_write)?;
    

    reg.produce_merge_event(&mut global_queue, loc, local_queue_location)?;
    reg.ack_global(&mut global_queue, receipt)?;
    
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
