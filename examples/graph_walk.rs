use anyhow::Context;
use beanstalkc::{Beanstalkc};
use random_string::generate;
use serde::{Deserialize, Serialize};
use silkworm::{DataCycle, Registry};
use std::collections::HashMap;
use std::fs::File;
use std::io::Read;
use std::{process, vec};
use std::time::Duration;
use std::io::Write;
use itertools::Itertools;

fn main() {
    println!("Hello from an example!");
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Default, Clone)]
struct Node {
    _label: String,
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Default, Clone)]
struct InputFile {
    _label: String,
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Default, Clone)]
struct GraphPath {
    _label: String,
    edges: Vec<Edge>
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Default, Clone)]
struct Edge {
    from: Node,
    to: Node,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Default, Clone)]
struct GraphData {
    nodes: HashMap<i64, Node>, // todo pull in im crate or use an ECS
    edges: HashMap<i64, Edge>,
    input_files: HashMap<i64, InputFile>,
    paths: HashMap<i64, GraphPath>,
}

#[derive(PartialEq)]
enum Data {
    Node(Node), Edge(Edge), InputFile(InputFile), GraphPath(GraphPath)
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Default, Clone)]
struct DatabaseLocation {
    data_type: String,
    hash: i64
}

impl DataCycle for Node {
    type Database = GraphData;
    type DataRoute = DatabaseLocation;
    type Data = Data;

    fn stop_categorically(&self, db: Self::Database) -> bool {
        db.edges.is_empty()
    }

    fn get_data(&self, db: &Self::Database, route: &Self::DataRoute) -> Option<Self::Data> {
        let node = db.nodes.get(&route.hash)?;
        Some(Data::Node(node.to_owned()))
    }

    fn get_friends(&self, db: &Self::Database, route: &Self::DataRoute) -> Vec<Self::Data> { // todo make this less cloney
        let data = self.get_data(db, route).expect("Do not get friends of nonexistent data");
        let edges = db.edges.iter().filter(|(_, inner_edge)| Data::Node(inner_edge.from.clone()) == data || Data::Node(inner_edge.to.clone()) == data).map(|(_, edge)| Data::Edge(edge.clone()));
        let paths = db.paths.iter().filter(|(_, graph_path)| graph_path.edges.iter().any(|inner_edge| Data::Node(inner_edge.from.clone()) == data || Data::Node(inner_edge.to.clone()) == data)).map(|(_, graph_path)| Data::GraphPath(graph_path.clone()));
        edges.chain(paths).collect_vec()
    }
}

impl DataCycle for Edge {
    type Database = GraphData;
    type DataRoute = DatabaseLocation;
    type Data = Data;

    fn stop_categorically(&self, db: Self::Database) -> bool {
        db.nodes.is_empty()
    }

    fn get_data(&self, db: &Self::Database, route: &Self::DataRoute) -> Option<Self::Data> {
        let edge = db.edges.get(&route.hash)?;
        Some(Data::Edge(edge.to_owned()))
    }

    fn get_friends(&self, db: &Self::Database, route: &Self::DataRoute) -> Vec<Self::Data> {
        let data = self.get_data(db, route).expect("Do not get friends of nonexistent data");
        if let Data::Edge(edge) = data {
            let nodes = db.nodes.iter().filter(|(_, node)| node.clone().clone() == edge.from || node.clone().clone() == edge.to).map(|(_, node)| Data::Node(node.clone()));
            let paths = db.paths.iter().filter(|(_, graph_path)| graph_path.edges.iter().any(|inner_edge| inner_edge.clone() == edge)).map(|(_, graph_path)| Data::GraphPath(graph_path.clone()));
            nodes.chain(paths).collect_vec()
        } else {
            panic!("edge handler is handling a non edge")
        }
        
    }
}

impl DataCycle for InputFile {
    type Database = GraphData;
    type DataRoute = DatabaseLocation;
    type Data = Data;

    fn stop_categorically(&self, _db: Self::Database) -> bool {
        false
    }

    fn get_data(&self, db: &Self::Database, route: &Self::DataRoute) -> Option<Self::Data> {
        let input_file = db.input_files.get(&route.hash)?;
        Some(Data::InputFile(input_file.to_owned()))
    }

    fn get_friends(&self, db: &Self::Database, route: &Self::DataRoute) -> Vec<Self::Data> {
        vec![]
    }
}

impl DataCycle for GraphPath {
    type Database = GraphData;
    type DataRoute = DatabaseLocation;
    type Data = Data;

    fn stop_categorically(&self, db: Self::Database) -> bool {
        db.nodes.is_empty() || db.edges.is_empty()
    }

    fn get_data(&self, db: &Self::Database, route: &Self::DataRoute) -> Option<Self::Data> {
        let graph_path = db.paths.get(&route.hash)?;
        Some(Data::GraphPath(graph_path.to_owned()))
    }

    fn get_friends(&self, db: &Self::Database, route: &Self::DataRoute) -> Vec<Self::Data> {
        let data = self.get_data(db, route).expect("Do not get friends of nonexistent data");

        if let Data::GraphPath(graph_path) = data {
        let nodes = db.nodes.iter().filter(|(_, node)| graph_path.edges.iter().any(|edge| edge.from == node.clone().clone() || edge.to == node.clone().clone() )).map(|(_, node)| Data::Node(node.clone()));
        let edges = db.edges.iter().filter(|(_, edge)| graph_path.edges.iter().any(|inner_edge| inner_edge == edge.clone())).map(|(_, edge)| Data::Edge(edge.clone()));
        nodes.chain(edges).collect_vec()
        } else {
            panic!("path handler is handling a non edge")
        }
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
    type Data = Data;

    fn create_db(&self) -> Self::Database {
        GraphData::default()
    }

    fn worker_name(&self) -> String {
        process::id().to_string()
    }

    fn db_location(&self, worker_name: String, random_string: String) -> Result<Self::Location, anyhow::Error> {
        let filename = "database".to_owned() + &worker_name + &random_string;

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

    fn produce_local(&self, mut queue: &mut Vec<DatabaseLocation>, loc: Self::DataRoute) {
        queue.push(loc)
    }


    fn queue_location(&self, worker_name: String, unique_string: String) -> Result<Self::Location, anyhow::Error> {
        let filename = "queue".to_owned() + &worker_name + &unique_string;

        Ok(filename)
    }

    fn write_local_queue(&self, loc: &Self::Location, queue: &Self::LocalQueue) -> Result<(), anyhow::Error> {
        let serialized = bincode::serialize(&queue)?;

        let mut f = File::create(loc)?;
        f.write_all(&serialized)?;
        Ok(())
    }

    fn write_db(&self, loc: &Self::Location, db: &Self::Database) -> Result<(), anyhow::Error> {
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

    fn collapse_dbs(&self, db: &Self::Database, other: &Self::Database) -> Self::Database { // todo make this less cloney
        let nodes = db.nodes.iter().chain(other.nodes.iter()).map(|(x, y)| (x.clone(), y.clone())).collect();
        let edges = db.edges.iter().chain(other.edges.iter()).map(|(x, y)| (x.clone(), y.clone())).collect();
        let input_files = db.input_files.iter().chain(other.input_files.iter()).map(|(x, y)| (x.clone(), y.clone())).collect();
        let paths = db.paths.iter().chain(other.paths.iter()).map(|(x, y)| (x.clone(), y.clone())).collect();

        GraphData{ nodes, edges, input_files, paths }
    }

    fn ack_global(&self, queue: &mut Self::GlobalQueueLocation, receipt: Self::JobReceipt) -> Result<(), anyhow::Error> {
        let _res = queue.delete(receipt)?;
        Ok(())
    }

    fn get_data_cycle(&self, route: Self::DataRoute) -> Box<dyn DataCycle<Database = Self::Database, DataRoute = Self::DataRoute, Data = Self::Data>> {
        if route.data_type == "nodes" {
            return Box::new(Node::default())
        }
        if route.data_type == "edges" {
            return Box::new(Edge::default())
        }
        if route.data_type == "input_files" {
            return Box::new(InputFile::default())
        }
        else { // route.data_type == "paths" {
            Box::new(GraphPath::default())
        }
    }

    fn unique_string(&self) -> String {
        let charset = "abcdefghijklmnopqrstuvwxyz";
        generate(10, charset)
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
