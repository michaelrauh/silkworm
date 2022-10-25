pub trait Registry {
    type Database;
    type Location;
    type GlobalQueueLocation;
    fn worker_name(&self) -> String;
    fn create_db(&self) -> Self::Database;
    fn db_location(&self, worker_name: String) -> Result<Self::Location, anyhow::Error>;
    fn write_db(&self, loc: Self::Location, db: Self::Database) -> Result<(), anyhow::Error>;
    fn create_global_queue(&self) -> Result<Self::GlobalQueueLocation, anyhow::Error>;
    fn consume_global(&self, queue: Self::GlobalQueueLocation) -> Result<Vec<u8>, anyhow::Error>;

    // read/write (to, from) (global, local) queue
}

pub trait DataCycle {
    type Database;
    // event name
    fn stop_categorically(&self, db: Self::Database) -> bool;
}

fn run_worker(reg: impl Registry, worker: impl DataCycle) {
    todo!()
}

fn life(reg: impl Registry) -> Result<(), anyhow::Error> {
    let name = reg.worker_name();
    let db = reg.create_db();
    let loc = reg.db_location(name)?;
    reg.write_db(loc, db)?;
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
