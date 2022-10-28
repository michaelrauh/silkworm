pub trait Registry {
    type Database;
    type Location;
    type GlobalQueueLocation;
    type DataRoute;
    type JobReceipt;
    type LocalQueue;

    fn worker_name(&self) -> String;
    fn create_db(&self) -> Self::Database;
    fn db_location(&self, worker_name: String) -> Result<Self::Location, anyhow::Error>;
    fn write_db(&self, loc: Self::Location, db: Self::Database) -> Result<(), anyhow::Error>;
    fn read_db(&self, loc: Self::Location) -> Result<Self::Database, anyhow::Error>;
    fn create_global_queue(&self) -> Result<Self::GlobalQueueLocation, anyhow::Error>;
    fn consume_global(&self, queue: Self::GlobalQueueLocation) -> Result<(Self::DataRoute, Self::JobReceipt), anyhow::Error>;
    fn produce_global(&self, data: Self::DataRoute, queue: Self::GlobalQueueLocation, priority: usize) -> Result<Self::JobReceipt, anyhow::Error>;
    fn ack_global(&self, queue: Self::GlobalQueueLocation, receipt: Self::JobReceipt) -> Result<(), anyhow::Error>;
    fn create_local_queue(&self) -> Self::LocalQueue;
    fn consume_local(&self, queue: Self::LocalQueue) -> Option<Self::DataRoute>;
    fn produce_local(&self, queue: Self::LocalQueue, loc: Self::DataRoute);
    fn produce_merge_event(&self, queue: Self::GlobalQueueLocation, loc: Self::Location) -> Result<Self::JobReceipt, anyhow::Error>;
    fn consume_merge_event(&self, queue: Self::GlobalQueueLocation) -> Result<Option<(Self::Location, Self::JobReceipt)>, anyhow::Error>;
    fn collapse_dbs(&self, db: Self::Database, other: Self::Database);
    // add a reorder local queue and a reorder frequency or trigger
}

pub trait DataCycle {
    type Database;
    fn stop_categorically(&self, db: Self::Database) -> bool;
}

fn run_worker(reg: impl Registry, worker: impl DataCycle) -> Result<(), anyhow::Error> {
    let name = reg.worker_name();
    let db = reg.create_db();
    let loc = reg.db_location(name)?;
    let global_queue = reg.create_global_queue()?;
    

    let (data_route, receipt) = reg.consume_global(global_queue)?; // consume batch size events and repeat batch count times until timeout

    // find the right datacycle and run it for the data route
    // let (db_update, global_events) = cycle_data(db, worker, data_route);
    let global_events = todo!();
    let db_update = todo!();
    reg.produce_global(global_events, global_queue, 0)?; // this hard coded priority is an issue. Priority will have to be bundled with the event if it is to stay

    // end datacycle
    

    reg.write_db(loc, db_update)?;
    reg.ack_global(global_queue, receipt)?;

    let merge_queue = reg.create_global_queue()?;
    let merge_event = reg.consume_merge_event(merge_queue)?;

    if merge_event.is_some() {
        let (other_loc, merge_rec) = merge_event.unwrap();
        
        let other_db = reg.read_db(other_loc)?;

        reg.collapse_dbs(db_update, other_db);
        // roll through each data point from current and do the merge algorithm
        // issue here is separation of events from data in the database
        // is it necessary to capture all events to replay them? 

        reg.write_db(loc, db_update)?;
        reg.produce_merge_event(global_queue, loc)?;
        reg.ack_global(merge_queue, merge_rec)?;
    }

    reg.produce_merge_event(global_queue, loc)?;
    
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
