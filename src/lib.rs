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
    fn create_global_queue(&self) -> Result<Self::GlobalQueueLocation, anyhow::Error>;
    fn consume_global(&self, queue: Self::GlobalQueueLocation) -> Result<(Self::DataRoute, Self::JobReceipt), anyhow::Error>;
    fn produce_global(&self, data: Self::DataRoute, queue: Self::GlobalQueueLocation, priority: usize) -> Result<Self::JobReceipt, anyhow::Error>;
    fn ack_global(&self, queue: Self::GlobalQueueLocation, receipt: Self::JobReceipt) -> Result<(), anyhow::Error>;
    fn nack_global(&self, queue: Self::GlobalQueueLocation, receipt: Self::JobReceipt) -> Result<(), anyhow::Error>;
    fn create_local_queue(&self) -> Self::LocalQueue;
    fn consume_local(&self, queue: Self::LocalQueue) -> Self::DataRoute;
    fn produce_local(&self, queue: Self::LocalQueue, loc: Self::DataRoute);
}

pub trait DataCycle {
    type Database;
    fn stop_categorically(&self, db: Self::Database) -> bool;
}

fn run_worker(reg: impl Registry, worker: impl DataCycle) -> Result<(), anyhow::Error> { // question this returning result
    // work out empty queue (waiting to start) scenario


    let name = reg.worker_name();
    let db = reg.create_db();
    let loc = reg.db_location(name)?;
    let global_queue = reg.create_global_queue()?;

    let (batch, receipt) = reg.consume_global(global_queue)?; // consume batch size events and repeat batch count times

    // cycle data
    // let (db_update, global_events) = cycle_data(db, worker, batch);
    //
    let db_update = todo!();
    let global_events = todo!();

    reg.write_db(loc, db_update)?;
    reg.ack_global(global_queue, receipt)?;
    // reg.nack_global(global_queue, receipt)?; // add this in somewhere
    // produce a merge event on the global queue
    
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
