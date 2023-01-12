use anyhow::{Context, Ok};
use beanstalkc::Beanstalkc;
use itertools::Itertools;
use random_string::generate;
use serde::{Deserialize, Serialize};
use silkworm::{DataCycle};
use std::collections::hash_map::DefaultHasher;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::hash::{Hash, Hasher};
use std::io::Read;
use std::io::Write;
use std::iter::once;
use std::time::Duration;
use std::{process, vec};

fn main() {
    println!("Hello from an example!");
}

struct GraphData {
    nodes: HashSet<Data>,
    edges: HashSet<Data>,
    input_files: HashSet<Data>,
    paths: HashSet<Data>,
}

#[derive(PartialEq, Serialize, Deserialize)]
enum Data {
    Node {
        label: String
    },
    Edge{
        from: Box<crate::Data>,
        to: Box<crate::Data>,
    },
    InputFile{
        contents: String,
    },
    GraphPath{
        edges: Vec<crate::Data>,
    },
}

impl DataCycle for Data {
    type Database = GraphData;

    type DataRoute = DatabaseLocation;

    fn stop_categorically(&self, db: &Self::Database) -> bool {
        todo!()
    }

    fn get_data(&self, db: &Self::Database, route: &Self::DataRoute) -> Option<Self> where Self: Sized {
        todo!()
    }

    fn stop_data(&self, data: &Self, db: &Self::Database) -> bool {
        match data {
            Data::Node(_) => todo!(),
            Data::Edge(_) => todo!(),
            Data::InputFile(_) => todo!(),
            Data::GraphPath(_) => todo!(),
        }
    }

    fn get_friends(&self, db: &Self::Database, route: &Self::DataRoute) -> Vec<Self> where Self: Sized {
        todo!()
    }

    fn stop_friends(&self, friends: &Vec<Self>) -> bool where Self: Sized {
        todo!()
    }

    fn search(&self, data: &Self, friends: &Vec<Self>) -> Vec<Self> where Self: Sized {
        todo!()
    }

    fn save(
        &self,
        db: &mut Self::Database,
        new_data: Vec<&Self>,
    ) -> Vec<Option<Self::DataRoute>> {
        todo!()
    }

    fn public(&self) -> bool {
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
