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
    fn write_db(&self, loc: Self::Location, db: Self::Database) -> Result<(), anyhow::Error>;
    fn read_db(&self, loc: Self::Location) -> Result<Self::Database, anyhow::Error>;
    fn read_queue(&self, loc: Self::Location) -> Result<Self::Database, anyhow::Error>;
    fn create_global_queue(&self) -> Result<Self::GlobalQueueLocation, anyhow::Error>;
    fn consume_global(&self, queue: Self::GlobalQueueLocation) -> Result<(Self::DataRoute, Self::JobReceipt), anyhow::Error>;
    fn produce_global(&self, data: Self::DataRoute, queue: Self::GlobalQueueLocation, priority: usize) -> Result<Self::JobReceipt, anyhow::Error>;
    fn ack_global(&self, queue: Self::GlobalQueueLocation, receipt: Self::JobReceipt) -> Result<(), anyhow::Error>;
    fn create_local_queue(&self) -> Self::LocalQueue;
    fn consume_local(&self, queue: Self::LocalQueue) -> Option<Self::DataRoute>;
    fn produce_local(&self, queue: Self::LocalQueue, loc: Self::DataRoute);
    fn produce_merge_event(&self, queue: Self::GlobalQueueLocation, db_loc: Self::Location, queue_loc: Self::Location) -> Result<Self::JobReceipt, anyhow::Error>;
    fn consume_merge_event(&self, queue: Self::GlobalQueueLocation) -> Result<Option<(Self::Location, Self::Location, Self::JobReceipt)>, anyhow::Error>;
    fn collapse_dbs(&self, db: Self::Database, other: Self::Database);
    fn queue_location(&self, worker_name: String) -> Result<Self::Location, anyhow::Error>;
    fn write_local_queue(&self, loc: Self::Location, queue: Self::LocalQueue) -> Result<(), anyhow::Error>;
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

    let local_queue = reg.create_local_queue();
    let queue_to_write = local_queue.clone();
    reg.produce_local(local_queue, data_route);

    // find the right datacycle and run it for the data route
    // let (db_update, global_events) = cycle_data(db, worker, local_queue);
    
    let global_events = todo!();
    let db_update = todo!();
    reg.produce_global(global_events, global_queue, 0)?; // this hard coded priority is an issue. Priority will have to be bundled with the event if it is to stay

    // end datacycle
    

    reg.write_db(loc, db_update)?;
    let local_queue_location = reg.queue_location(name)?;
    reg.write_local_queue(local_queue_location, queue_to_write)?;
    reg.ack_global(global_queue, receipt)?;

    let merge_queue = reg.create_global_queue()?;
    let merge_event = reg.consume_merge_event(merge_queue)?;

    if merge_event.is_some() { // move merge event handling to the top and abort early. Have it merge two written things in stead of one written and one not.
        let (other_db_loc, other_queue_loc, merge_rec) = merge_event.unwrap();
        
        let other_db = reg.read_db(other_db_loc)?;
        let other_queue = reg.read_queue(other_queue_loc)?;

        reg.collapse_dbs(db_update, other_db);
        // replay all events in both queues and use provenance to determine merge tactic

        reg.write_db(loc, db_update)?;
        reg.produce_merge_event(global_queue, loc, local_queue_location)?;
        reg.ack_global(merge_queue, merge_rec)?;
    }

    reg.produce_merge_event(global_queue, loc, local_queue_location)?;
    
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
