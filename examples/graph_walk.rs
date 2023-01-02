use anyhow::{Context, Ok};
use beanstalkc::Beanstalkc;
use itertools::Itertools;
use random_string::generate;
use serde::{Deserialize, Serialize};
use silkworm::{run_worker, DataCycle, Registry};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::io::Write;
use std::iter::once;
use std::time::Duration;
use std::{process, vec};

fn main() {
    println!("Hello from an example!");
    let holder = Holder {};

    run_worker(holder).unwrap();
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
    edges: Vec<Edge>,
}

#[derive(PartialEq, Eq, Hash, Serialize, Deserialize, Default, Clone)]
struct Edge {
    from: Node,
    to: Node,
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Default, Clone)]
struct GraphData {
    nodes: HashMap<u64, Node>,
    edges: HashMap<u64, Edge>,
    input_files: HashMap<u64, InputFile>,
    paths: HashMap<u64, GraphPath>,
}

#[derive(PartialEq, Serialize, Deserialize)]
enum Data {
    Node(Node),
    Edge(Edge),
    InputFile(InputFile),
    GraphPath(GraphPath),
}

#[derive(PartialEq, Eq, Serialize, Deserialize, Default, Clone)]
struct DatabaseLocation {
    data_type: String,
    hash: u64,
}

impl DataCycle for Node {
    type Database = GraphData;
    type DataRoute = DatabaseLocation;
    type Data = Data;

    fn stop_categorically(&self, db: &Self::Database) -> bool {
        db.edges.is_empty()
    }

    fn get_data(&self, db: &Self::Database, route: &Self::DataRoute) -> Option<Self::Data> {
        db.nodes
            .get(&route.hash)
            .map(|node| Data::Node(node.to_owned()))
    }

    fn get_friends(&self, db: &Self::Database, route: &Self::DataRoute) -> Vec<Self::Data> {
        // paths are inhabited and so do not need to be checked
        // edges may not be inhabited, so the non-missing node must be checked to be an edge eligible for path inclusion

        // beginning
        // missing node on the left - ox : xxxxxxx
        // missing node on the right - xo : xxxxxx
        // end
        // missing node on the left - xxxxx : ox
        // missing node on the right - xxxxx : xo
        let data = self
            .get_data(db, route)
            .expect("Do not get friends of nonexistent data");

        let missing_node;
        if let Data::Node(node) = data {
            missing_node = node;
        } else {
            panic!("route in data cycle must point to a node")
        }

        let missing_node_on_left_edges = db
            .edges
            .iter()
            .filter(|(_, inner_edge)| {
                missing_node == inner_edge.from && db.nodes.values().contains(&inner_edge.to)
            })
            .map(|(_, inner_edge)| inner_edge)
            .cloned();
        let missing_node_on_right_edges = db
            .edges
            .iter()
            .filter(|(_, inner_edge)| {
                missing_node == inner_edge.to && db.nodes.values().contains(&inner_edge.from)
            })
            .map(|(_, inner_edge)| inner_edge)
            .cloned();

        let packaged_beginning_missing_node_on_left_paths = db
            .paths
            .iter()
            .filter(|(_, graph_path)| {
                missing_node_on_left_edges.clone().any(|inner_edge| {
                    inner_edge.to
                        == graph_path
                            .edges
                            .first()
                            .expect("dont have empty paths")
                            .from
                })
            })
            .map(|(_, inner_graph_path)| inner_graph_path)
            .cloned()
            .map(|inner_graph_path| Data::GraphPath(inner_graph_path));

        let packaged_beginning_missing_node_on_right_paths = db
            .paths
            .iter()
            .filter(|(_, graph_path)| {
                missing_node
                    == graph_path
                        .edges
                        .first()
                        .expect("dont have empty paths")
                        .from
            })
            .map(|(_, inner_graph_path)| inner_graph_path)
            .cloned()
            .map(|inner_graph_path| Data::GraphPath(inner_graph_path));

        let packaged_ending_missing_node_on_left_paths = db
            .paths
            .iter()
            .filter(|(_, graph_path)| {
                missing_node == graph_path.edges.last().expect("dont have empty paths").to
            })
            .map(|(_, inner_graph_path)| inner_graph_path)
            .cloned()
            .map(|inner_graph_path| Data::GraphPath(inner_graph_path));

        let packaged_ending_missing_node_on_right_paths = db
            .paths
            .iter()
            .filter(|(_, graph_path)| {
                missing_node_on_right_edges.clone().any(|inner_edge| {
                    inner_edge.from == graph_path.edges.last().expect("dont have empty paths").to
                })
            })
            .map(|(_, inner_graph_path)| inner_graph_path)
            .cloned()
            .map(|inner_graph_path| Data::GraphPath(inner_graph_path));

        let packaged_missing_node_on_left_edges = missing_node_on_left_edges
            .clone()
            .map(|edge| Data::Edge(edge));
        let packaged_missing_node_on_right_edges = missing_node_on_right_edges
            .clone()
            .map(|edge| Data::Edge(edge));

        packaged_missing_node_on_left_edges
            .chain(packaged_missing_node_on_right_edges)
            .chain(packaged_beginning_missing_node_on_left_paths)
            .chain(packaged_beginning_missing_node_on_right_paths)
            .chain(packaged_ending_missing_node_on_left_paths)
            .chain(packaged_ending_missing_node_on_right_paths)
            .collect_vec()
    }

    fn stop_data(&self, _data: &Self::Data, _db: &Self::Database) -> bool {
        false
    }

    fn stop_friends(&self, friends: &Vec<Self::Data>) -> bool {
        friends.is_empty()
    }

    fn search(&self, data: &Self::Data, friends: &Vec<Self::Data>) -> Vec<Self::Data> {
        // look for paths connected to this node. Collect into new paths. Make sure the new paths only use occupied nodes

        // two edges
        // missing node in the middle of two edges - xo : ox
        // beginning
        // missing node on the left - ox : xxxxxxx
        // missing node on the right - xo : xxxxxx
        // end
        // missing node on the left - xxxxx : ox
        // missing node on the right - xxxxx : xo

        let missing_node;
        if let Data::Node(node) = data {
            missing_node = node;
        } else {
            panic!("route in data cycle must point to a node")
        }

        let mut paths: Vec<&GraphPath> = vec![];
        let mut left_edges: Vec<&Edge> = vec![];
        let mut right_edges: Vec<&Edge> = vec![];
        for friend in friends {
            match friend {
                Data::Node(_) => panic!("nodes are not friends with nodes"),
                Data::Edge(edge) => {
                    if &edge.from == missing_node {
                        left_edges.push(edge);
                    } else {
                        right_edges.push(edge);
                    }
                }
                Data::InputFile(_) => panic!("nodes are not friends with input files"),
                Data::GraphPath(path) => paths.push(path),
            }
        }

        let new_minimal_path = right_edges
            .iter()
            .cartesian_product(left_edges.clone())
            .map(|(re, le)| {
                Data::GraphPath(GraphPath {
                    edges: vec![re.to_owned().to_owned(), le.to_owned()],
                })
            });

        let beginning = left_edges
            .iter()
            .chain(&right_edges)
            .cartesian_product(paths.clone())
            .filter(|(inner_edge, inner_path)| {
                inner_edge.to == inner_path.edges.first().expect("nonempty path").from
            })
            .map(|(inner_edge, inner_path)| {
                Data::GraphPath(GraphPath {
                    edges: once(inner_edge.to_owned())
                        .chain(inner_path.edges.iter())
                        .cloned()
                        .collect_vec(),
                })
            });

        let end = left_edges
            .iter()
            .chain(&right_edges)
            .cartesian_product(paths)
            .filter(|(inner_edge, inner_path)| {
                inner_edge.from == inner_path.edges.last().expect("nonempty path").from
            })
            .map(|(inner_edge, inner_path)| {
                Data::GraphPath(GraphPath {
                    edges: inner_path
                        .edges
                        .iter()
                        .chain(once(inner_edge.to_owned()))
                        .cloned()
                        .collect_vec(),
                })
            });

        new_minimal_path.chain(beginning).chain(end).collect_vec()
    }

    fn save(
        &self,
        db: &mut Self::Database,
        new_data: Vec<&Data>,
    ) -> Vec<std::option::Option<DatabaseLocation>> {
        new_data
            .into_iter()
            .map(|datum| {
                let data;
                if let Data::GraphPath(path) = datum {
                    data = path;
                } else {
                    panic!("node should only be saving paths")
                }

                let mut hasher = DefaultHasher::new();
                data.hash(&mut hasher);
                let hash = hasher.finish();

                let existed = db.paths.insert(hash, data.to_owned());

                match existed {
                    Some(_) => None,
                    None => Some(DatabaseLocation {
                        data_type: "paths".to_string(),
                        hash,
                    }),
                }
            })
            .collect_vec()
    }

    fn public(&self) -> bool {
        false
    }
}

impl DataCycle for Edge {
    type Database = GraphData;
    type DataRoute = DatabaseLocation;
    type Data = Data;

    fn stop_categorically(&self, db: &Self::Database) -> bool {
        db.nodes.is_empty()
    }

    fn get_data(&self, db: &Self::Database, route: &Self::DataRoute) -> Option<Self::Data> {
        db.edges
            .get(&route.hash)
            .map(|edge| Data::Edge(edge.to_owned()))
    }

    fn get_friends(&self, db: &Self::Database, route: &Self::DataRoute) -> Vec<Self::Data> {
        // All edges must be inhabited to be friends
        // edge : edge
        // to left
        // to right

        // edge : path
        // path : edge

        let data = self
            .get_data(db, route)
            .expect("Do not get friends of nonexistent data");

        let missing_edge;
        if let Data::Edge(edge) = data {
            missing_edge = edge;
        } else {
            panic!("route in data cycle must point to an edge")
        };

        let left_node = missing_edge.from;
        let right_node = missing_edge.to;

        let edges_to_left = db
            .edges
            .iter()
            .filter(|(_, inner_edge)| inner_edge.to == left_node)
            .filter(|(_, inner_edge)| db.nodes.values().contains(&inner_edge.from))
            .map(|(_, inner_edge)| inner_edge)
            .cloned()
            .map(|inner_edge| Data::Edge(inner_edge));

        let edges_to_right = db
            .edges
            .iter()
            .filter(|(_, inner_edge)| inner_edge.from == right_node)
            .filter(|(_, inner_edge)| db.nodes.values().contains(&inner_edge.to))
            .map(|(_, inner_edge)| inner_edge)
            .cloned()
            .map(|inner_edge| Data::Edge(inner_edge));

        let paths_to_right = db
            .paths
            .iter()
            .filter(|(_, inner_path)| {
                inner_path.edges.first().expect("nonempty path").from == right_node
            })
            .map(|(_, inner_path)| inner_path)
            .cloned()
            .map(|inner_path| Data::GraphPath(inner_path));

        let paths_to_left = db
            .paths
            .iter()
            .filter(|(_, inner_path)| {
                inner_path.edges.last().expect("nonempty path").to == left_node
            })
            .map(|(_, inner_path)| inner_path)
            .cloned()
            .map(|inner_path| Data::GraphPath(inner_path));

        edges_to_left
            .chain(edges_to_right)
            .chain(paths_to_right)
            .chain(paths_to_left)
            .collect_vec()
    }

    fn stop_data(&self, data: &Self::Data, db: &Self::Database) -> bool {
        let missing_edge;
        if let Data::Edge(edge) = data {
            missing_edge = edge;
        } else {
            panic!("route in data cycle must point to an edge")
        };

        !(db.nodes.values().contains(&missing_edge.from)
            && db.nodes.values().contains(&missing_edge.to))
    }

    fn stop_friends(&self, friends: &Vec<Self::Data>) -> bool {
        friends.is_empty()
    }

    fn search(&self, data: &Self::Data, friends: &Vec<Self::Data>) -> Vec<Self::Data> {
        // edge : edge
        // to left
        // to right

        // edge : path
        // path : edge
        let missing_edge;
        if let Data::Edge(edge) = data {
            missing_edge = edge;
        } else {
            panic!("route in data cycle must point to an edge")
        };

        let right_node = missing_edge.to.clone();

        let mut left_paths: Vec<&GraphPath> = vec![];
        let mut right_paths: Vec<&GraphPath> = vec![];
        let mut left_edges: Vec<&Edge> = vec![];
        let mut right_edges: Vec<&Edge> = vec![];
        for friend in friends {
            match friend {
                Data::Node(_) => panic!("edges are not friends with nodes"),
                Data::Edge(edge) => {
                    if edge.from == right_node {
                        left_edges.push(edge);
                    } else {
                        // if edge.to == left node
                        right_edges.push(edge);
                    }
                }
                Data::InputFile(_) => panic!("edges are not friends with input files"),
                Data::GraphPath(path) => {
                    if path.edges.first().expect("nonempty").from == right_node {
                        left_paths.push(path);
                    } else {
                        // if path.last.to == left node
                        right_paths.push(path);
                    }
                }
            }
        }

        let edge_friend_then_missing = right_edges.iter().map(|inner_edge| {
            Data::GraphPath(GraphPath {
                edges: vec![inner_edge.to_owned().to_owned(), missing_edge.to_owned()],
            })
        });
        let missing_then_edge_friend = left_edges.iter().map(|inner_edge| {
            Data::GraphPath(GraphPath {
                edges: vec![missing_edge.to_owned(), inner_edge.to_owned().to_owned()],
            })
        });

        let path_friend_then_missing = right_paths.iter().map(|inner_path| {
            Data::GraphPath(GraphPath {
                edges: inner_path
                    .edges
                    .iter()
                    .cloned()
                    .chain(once(missing_edge.clone()))
                    .collect_vec(),
            })
        });
        let missing_then_path_friend = left_paths.iter().map(|inner_path| {
            Data::GraphPath(GraphPath {
                edges: once(missing_edge.clone())
                    .chain(inner_path.edges.iter().cloned())
                    .collect_vec(),
            })
        });

        edge_friend_then_missing
            .chain(missing_then_edge_friend)
            .chain(path_friend_then_missing)
            .chain(missing_then_path_friend)
            .collect_vec()
    }

    fn save(
        &self,
        db: &mut Self::Database,
        new_data: Vec<&Data>,
    ) -> Vec<std::option::Option<DatabaseLocation>> {
        new_data
            .into_iter()
            .map(|datum| {
                let data;
                if let Data::GraphPath(path) = datum {
                    data = path;
                } else {
                    panic!("edge should only be saving paths")
                }

                let mut hasher = DefaultHasher::new();
                data.hash(&mut hasher);
                let hash = hasher.finish();

                let existed = db.paths.insert(hash, data.to_owned());

                match existed {
                    Some(_) => None,
                    None => Some(DatabaseLocation {
                        data_type: "paths".to_string(),
                        hash,
                    }),
                }
            })
            .collect_vec()
    }

    fn public(&self) -> bool {
        false
    }
}

impl DataCycle for InputFile {
    type Database = GraphData;
    type DataRoute = DatabaseLocation;
    type Data = Data;

    fn stop_categorically(&self, _db: &Self::Database) -> bool {
        false
    }

    fn get_data(&self, db: &Self::Database, route: &Self::DataRoute) -> Option<Self::Data> {
        db.input_files
            .get(&route.hash)
            .map(|input_file| Data::InputFile(input_file.to_owned()))
    }

    fn get_friends(&self, _db: &Self::Database, _route: &Self::DataRoute) -> Vec<Self::Data> {
        vec![]
    }

    fn stop_data(&self, data: &Self::Data, db: &Self::Database) -> bool {
        todo!()
    }

    fn stop_friends(&self, friends: &Vec<Self::Data>) -> bool {
        todo!()
    }

    fn search(&self, data: &Self::Data, friends: &Vec<Self::Data>) -> Vec<Self::Data> {
        todo!()
    }

    fn save(
        &self,
        db: &mut Self::Database,
        new_data: Vec<&Data>,
    ) -> Vec<std::option::Option<DatabaseLocation>> {
        todo!()
    }

    fn public(&self) -> bool {
        todo!()
    }
}

impl DataCycle for GraphPath {
    type Database = GraphData;
    type DataRoute = DatabaseLocation;
    type Data = Data;

    fn stop_categorically(&self, db: &Self::Database) -> bool {
        db.nodes.is_empty() || db.edges.is_empty()
    }

    fn get_data(&self, db: &Self::Database, route: &Self::DataRoute) -> Option<Self::Data> {
        db.paths
            .get(&route.hash)
            .map(|graph_path| Data::GraphPath(graph_path.to_owned()))
    }

    fn get_friends(&self, db: &Self::Database, route: &Self::DataRoute) -> Vec<Self::Data> {
        // todo make sure this reflects only occupied nodes and edges. That is, edges that connect occupied nodes.
        let data = self
            .get_data(db, route)
            .expect("Do not get friends of nonexistent data");

        if let Data::GraphPath(graph_path) = data {
            let nodes = db
                .nodes
                .iter()
                .filter(|(_, node)| {
                    graph_path.edges.iter().any(|edge| {
                        edge.from == node.clone().clone() || edge.to == node.clone().clone()
                    })
                })
                .map(|(_, node)| Data::Node(node.clone()));
            let edges = db
                .edges
                .iter()
                .filter(|(_, edge)| {
                    graph_path
                        .edges
                        .iter()
                        .any(|inner_edge| inner_edge == edge.clone())
                })
                .map(|(_, edge)| Data::Edge(edge.clone()));
            nodes.chain(edges).collect_vec()
        } else {
            panic!("path handler is handling a non edge")
        }
    }

    fn stop_data(&self, data: &Self::Data, db: &Self::Database) -> bool {
        todo!()
    }

    fn stop_friends(&self, friends: &Vec<Self::Data>) -> bool {
        todo!()
    }

    fn search(&self, data: &Self::Data, friends: &Vec<Self::Data>) -> Vec<Self::Data> {
        todo!()
    }

    fn save(
        &self,
        db: &mut Self::Database,
        new_data: Vec<&Data>,
    ) -> Vec<std::option::Option<DatabaseLocation>> {
        todo!()
    }

    fn public(&self) -> bool {
        todo!()
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

    fn db_location(
        &self,
        worker_name: String,
        random_string: String,
    ) -> Result<Self::Location, anyhow::Error> {
        let filename = "database".to_owned() + &worker_name + &random_string;

        Ok(filename)
    }

    fn create_global_queue(&self) -> Result<Self::GlobalQueueLocation, anyhow::Error> {
        let conn = Beanstalkc::new().connect()?;
        Ok(conn)
    }

    fn consume_global(
        &self,
        queue: &mut Self::GlobalQueueLocation,
    ) -> Result<(Self::Data, Self::JobReceipt), anyhow::Error> {
        queue.watch("jobs")?;

        let job = queue.reserve()?;
        let ans = bincode::deserialize(job.body())?;
        let reciept = job.id();

        Ok((ans, reciept))
    }

    fn produce_global(
        &self,
        data: Self::Data,
        queue: &mut Self::GlobalQueueLocation,
        priority: usize,
    ) -> Result<Self::JobReceipt, anyhow::Error> {
        let to_put = bincode::serialize(&data)?;
        queue.use_tube("jobs")?;
        let res = queue.put(
            &to_put,
            priority.try_into().unwrap(),
            Duration::from_secs(0),
            Duration::from_secs(10),
        )?;

        Ok(res)
    }

    fn create_local_queue(&self) -> Self::LocalQueue {
        vec![]
    }

    fn consume_local(&self, queue: &mut Self::LocalQueue) -> Option<Self::DataRoute> {
        queue.pop()
    }

    fn produce_local(&self, queue: &mut Vec<DatabaseLocation>, loc: Self::DataRoute) {
        queue.push(loc)
    }

    fn queue_location(
        &self,
        worker_name: String,
        unique_string: String,
    ) -> Result<Self::Location, anyhow::Error> {
        let filename = "queue".to_owned() + &worker_name + &unique_string;

        Ok(filename)
    }

    fn write_local_queue(
        &self,
        loc: &Self::Location,
        queue: &Self::LocalQueue,
    ) -> Result<(), anyhow::Error> {
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

    fn consume_merge_event(
        &self,
        queue: &mut Self::GlobalQueueLocation,
    ) -> Result<
        Option<(
            Self::Location,
            Self::Location,
            Self::JobReceipt,
            Self::Location,
            Self::Location,
            Self::JobReceipt,
        )>,
        anyhow::Error,
    > {
        let binding = queue.stats_tube("merges")?;
        let count = binding
            .get("current-jobs-ready")
            .context("cannot get current jobs ready")?;
        if count == "0" || count == "1" {
            return Ok(None);
        }

        queue.watch("merges")?;

        let first_job = queue.reserve()?;
        let (first_db, first_queue) = bincode::deserialize(first_job.body())?;
        let first_receipt = first_job.id();

        let second_job = queue.reserve()?;
        let (second_db, second_queue) = bincode::deserialize(second_job.body())?;
        let second_receipt = second_job.id();

        let tup = (
            first_db,
            first_queue,
            first_receipt,
            second_db,
            second_queue,
            second_receipt,
        );
        let ans = Some(tup);
        Ok(ans)
    }

    fn produce_merge_event(
        &self,
        queue: &mut Self::GlobalQueueLocation,
        first_db_loc: Self::Location,
        first_queue_loc: Self::Location,
    ) -> Result<Self::JobReceipt, anyhow::Error> {
        let to_put = bincode::serialize(&(first_db_loc, first_queue_loc))?;
        queue.use_tube("merges")?;
        let res = queue.put(&to_put, 0, Duration::from_secs(0), Duration::from_secs(100))?;

        Ok(res)
    }

    fn collapse_dbs(&self, db: &Self::Database, other: &Self::Database) -> Self::Database {
        // todo make this less cloney
        let nodes = db
            .nodes
            .iter()
            .chain(other.nodes.iter())
            .map(|(x, y)| (x.clone(), y.clone()))
            .collect();
        let edges = db
            .edges
            .iter()
            .chain(other.edges.iter())
            .map(|(x, y)| (x.clone(), y.clone()))
            .collect();
        let input_files = db
            .input_files
            .iter()
            .chain(other.input_files.iter())
            .map(|(x, y)| (x.clone(), y.clone()))
            .collect();
        let paths = db
            .paths
            .iter()
            .chain(other.paths.iter())
            .map(|(x, y)| (x.clone(), y.clone()))
            .collect();

        GraphData {
            nodes,
            edges,
            input_files,
            paths,
        }
    }

    fn ack_global(
        &self,
        queue: &mut Self::GlobalQueueLocation,
        receipt: Self::JobReceipt,
    ) -> Result<(), anyhow::Error> {
        let _res = queue.delete(receipt)?;
        Ok(())
    }

    fn get_data_cycle(
        &self,
        route: Self::DataRoute,
    ) -> Box<dyn DataCycle<Database = Self::Database, DataRoute = Self::DataRoute, Data = Self::Data>>
    {
        if route.data_type == "nodes" {
            return Box::new(Node::default());
        }
        if route.data_type == "edges" {
            return Box::new(Edge::default());
        }
        if route.data_type == "input_files" {
            return Box::new(InputFile::default());
        } else {
            // route.data_type == "paths" {
            Box::new(GraphPath::default())
        }
    }

    fn unique_string(&self) -> String {
        let charset = "abcdefghijklmnopqrstuvwxyz";
        generate(10, charset)
    }

    fn delete_db(&self, loc: Self::Location) -> Result<(), anyhow::Error> {
        fs::remove_file(loc)?;
        Ok(())
    }

    fn cycle_by_data(
        &self,
        data: &Self::Data,
    ) -> Box<dyn DataCycle<Database = Self::Database, DataRoute = Self::DataRoute, Data = Self::Data>>
    {
        todo!()
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
