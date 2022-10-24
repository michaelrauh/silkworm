use std::{collections::HashSet, io::Write};
use std::fs::File;
use std::process;
use serde::{Deserialize, Serialize};
use silkworm::Registry;


fn main() {
    println!("Hello from an example!");
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
struct Node {
    _label: String
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
struct Edge {
    from: Node,
    to: Node
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Default)]
struct GraphData {
    nodes: HashSet<Node>,
    edges: HashSet<Edge>,
    input_files: HashSet<String>,
    paths: HashSet<Vec<Edge>>
}

struct Holder{}

impl Registry for Holder {
    
    type Database = GraphData;
    type Location = File;

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
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_tests_examples() {
        let result = 2 + 2;
        assert_eq!(result, 4);
    }
}
