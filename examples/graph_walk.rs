use std::{collections::HashSet, io::Write};
use std::fs::File;
use std::process;
use serde::{Deserialize, Serialize};
use silkworm::Registry;


fn main() {
    println!("Hello from an example!");
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize)]
struct Node {
    _label: String
}

type Nodes = HashSet<Node>;
type Edges = HashSet<(Node, Node)>;
type NodesAndEdges = (Nodes, Edges);

struct Holder {
}

impl Registry for Holder {
    
    type Database = NodesAndEdges;
    type Location = File;

    fn create_db(&self) -> Self::Database {
        (Nodes::default(), Edges::default())
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
