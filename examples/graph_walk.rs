use anyhow::Context;
use beanstalkc::{Beanstalkc};
use serde::{Deserialize, Serialize};
use silkworm::{DataCycle, Registry};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
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

#[derive(PartialEq, Eq, Serialize, Deserialize, Default, Clone)]
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
    type Location = String;
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
        let filename = "database".to_owned() + &worker_name;

        Ok(filename)
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
        queue.use_tube("jobs")?;
        let res = queue.put(&to_put, priority.try_into().unwrap(), Duration::from_secs(0), Duration::from_secs(10))?;

        Ok(res)
    }

    fn create_local_queue(&self) -> Self::LocalQueue {
        vec![]
    }

    fn consume_local(&self, queue: &mut Self::LocalQueue) -> Option<Self::DataRoute> {
        queue.pop()
    }

    fn produce_local(&self, mut queue: Self::LocalQueue, loc: Self::DataRoute) {
        queue.push(loc)
    }


    fn queue_location(&self, worker_name: String) -> Result<Self::Location, anyhow::Error> {
        let filename = "queue".to_owned() + &worker_name;

        Ok(filename)
    }

    fn write_local_queue(&self, loc: Self::Location, queue: Self::LocalQueue) -> Result<(), anyhow::Error> {
        let serialized = bincode::serialize(&queue)?;

        let mut f = File::create(loc)?;
        f.write_all(&serialized)?;
        Ok(())
    }

    fn write_db(&self, loc: &Self::Location, db: &mut Self::Database) -> Result<(), anyhow::Error> {
            let serialized = bincode::serialize(&db)?;
    
            let mut f = File::create(loc)?;
            f.write_all(&serialized)?;
            Ok(())
    }
    
    fn read_queue(&self, loc: &Self::Location) -> Result<Self::LocalQueue, anyhow::Error> {
        let mut f = File::create(loc)?;
        let mut buf = vec![];
        f.read_to_end(&mut buf)?;
        let deserialized = bincode::deserialize(&buf)?;

        Ok(deserialized)
    }

    fn read_db(&self, loc: &Self::Location) -> Result<Self::Database, anyhow::Error> {
        let mut f = File::create(loc)?;
        let mut buf = vec![];
        f.read_to_end(&mut buf)?;
        let deserialized = bincode::deserialize(&buf)?;

        Ok(deserialized)
    }

    fn consume_merge_event(&self, queue: &mut Self::GlobalQueueLocation) -> Result<Option<(Self::Location, Self::Location, Self::JobReceipt, Self::Location, Self::Location, Self::JobReceipt)>, anyhow::Error> {
        let binding = queue.stats_tube("merges")?;
        let count = binding.get("current-jobs-ready").context("cannot get current jobs ready")?;
        if count == "0" || count == "1" {
            return Ok(None)
        }

        queue.watch("merges")?;

        let first_job = queue.reserve()?;
        let (first_db, first_queue) = bincode::deserialize(first_job.body())?;
        let first_receipt = first_job.id();

        let second_job = queue.reserve()?;
        let (second_db, second_queue) = bincode::deserialize(second_job.body())?;
        let second_receipt = second_job.id();

        let tup = (first_db, first_queue, first_receipt, second_db, second_queue, second_receipt);
        let ans = Some(tup);
        Ok(ans)
    }

    fn produce_merge_event(&self, queue: &mut Self::GlobalQueueLocation, first_db_loc: Self::Location, first_queue_loc: Self::Location) -> Result<Self::JobReceipt, anyhow::Error> {
        let to_put = bincode::serialize(&(first_db_loc, first_queue_loc))?;
            queue.use_tube("merges")?;
            let res = queue.put(&to_put, 0, Duration::from_secs(0), Duration::from_secs(100))?;
    
            Ok(res)
    }

    fn collapse_dbs(&self, db: &mut Self::Database, other: Self::Database) {
        db.nodes.extend(other.nodes.into_iter());
            db.edges.extend(other.edges.into_iter());
            db.input_files.extend(other.input_files.into_iter());
            db.paths.extend(other.paths.into_iter());
    }

    fn ack_global(&self, queue: &mut Self::GlobalQueueLocation, receipt: Self::JobReceipt) -> Result<(), anyhow::Error> {
        let _res = queue.delete(receipt)?;
        Ok(())
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
