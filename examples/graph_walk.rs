use amiquip::{AmqpValue, Connection, FieldTable, QueueDeclareOptions};
use beanstalkc::{Beanstalkc, Job};
use serde::{Deserialize, Serialize};
use silkworm::{DataCycle, Registry};
use std::fs::File;
use std::process;
use std::{collections::HashSet, io::Write};

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
    nodes: HashSet<Node>,
    edges: HashSet<Edge>,
    input_files: HashSet<String>,
    paths: HashSet<Vec<Edge>>,
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
            .connect()
            .expect("connect to beanstalkd server failed");
        Ok(conn)
    }

    fn consume_global(&self, mut queue: Self::GlobalQueueLocation) -> Result<Vec<u8>, anyhow::Error> {
        queue.watch("jobs").unwrap();

        let job = queue.reserve().unwrap();

        Ok(job.body().to_vec())
        // add a delete call to lifecycle
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
