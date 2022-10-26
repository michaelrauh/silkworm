use beanstalkc::{Beanstalkc};
use serde::{Deserialize, Serialize};
use silkworm::{DataCycle, Registry};
use std::collections::HashMap;
use std::fs::File;
use std::process;
use std::time::Duration;
use std::{io::Write};

fn main() {
    println!("Hello from an example!");
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
struct Node {
    _label: String,
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
struct Edge {
    from: Node,
    to: Node,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Default)]
struct GraphData {
    nodes: HashMap<i64, Node>,
    edges: HashMap<i64, Edge>,
    input_files: HashMap<i64, String>,
    paths: HashMap<i64, Vec<Edge>>,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Default)]
struct DatabaseLocation {
    data_type: String,
    hash: i64
}

impl DataCycle for Node {
    type Database = GraphData;

    fn stop_categorically(&self, db: Self::Database) -> bool {
        !db.edges.is_empty()
    }
}

struct Holder {}

impl Registry for Holder {
    type Database = GraphData;
    type Location = File;
    type GlobalQueueLocation = Beanstalkc;
    type DataRoute = DatabaseLocation;
    type JobReceipt = u64;
    type LocalQueue = Vec<DatabaseLocation>;

    fn create_db(&self) -> Self::Database {
        GraphData::default()
    }

    fn worker_name(&self) -> String {
        process::id().to_string()
    }

    fn db_location(&self, worker_name: String) -> Result<Self::Location, anyhow::Error> {
        let res = File::create("database".to_owned() + &worker_name)?;
        Ok(res)
    }

    fn write_db(&self, mut loc: Self::Location, db: Self::Database) -> Result<(), anyhow::Error> {
        let serialized = bincode::serialize(&db)?;
        loc.write_all(&serialized)?;
        Ok(())
    }

    fn create_global_queue(&self) -> Result<Self::GlobalQueueLocation, anyhow::Error> {
        let conn = Beanstalkc::new()
            .connect()?;
        Ok(conn)
    }

    fn consume_global(&self, mut queue: Self::GlobalQueueLocation) -> Result<(Self::DataRoute, Self::JobReceipt), anyhow::Error> {
        queue.watch("jobs")?;

        let job = queue.reserve()?;
        let ans = bincode::deserialize(job.body())?;
        let reciept = job.id();

        Ok((ans, reciept))
    }

    fn produce_global(&self, data: Self::DataRoute, mut queue: Self::GlobalQueueLocation, priority: usize) -> Result<Self::JobReceipt, anyhow::Error> {
        let to_put = bincode::serialize(&data)?;
        let res = queue.put(&to_put, priority.try_into().unwrap(), Duration::from_secs(0), Duration::from_secs(10))?;

        Ok(res)
    }

    fn ack_global(&self, mut queue: Self::GlobalQueueLocation, receipt: Self::JobReceipt) -> Result<(), anyhow::Error> {
        let _res = queue.delete(receipt)?;
        Ok(())
    }

    fn nack_global(&self, mut queue: Self::GlobalQueueLocation, receipt: Self::JobReceipt) -> Result<(), anyhow::Error> {
        queue.bury_default(receipt)?;
        queue.kick_job(receipt)?;

        Ok(())
    }

    fn create_local_queue(&self) -> Self::LocalQueue {
        vec![]
    }

    fn consume_local(&self, mut queue: Self::LocalQueue) -> Self::DataRoute { // note there is no ack or nack for local as it consumes live and infallible
        queue.pop().expect("should this method be optional?") // there is an issue here: framework should assume nonempty at this stage so the types should too
    }

    fn produce_local(&self, mut queue: Self::LocalQueue, loc: Self::DataRoute) {
        queue.push(loc)
    }
    
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_tests_examples() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
