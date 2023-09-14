use std::path::PathBuf;
use std::collections::{HashMap, BTreeMap};
use crate::query_creation::state_generators::subgraph_type::SubgraphType;
use sqlparser::ast::{
    Ident, ObjectName, 
};
use rand::distributions::WeightedIndex;
use rand::thread_rng;
use rand::distributions::Distribution;
use std::fs;

pub struct AggregateFunctionDistribution {
   pub map : HashMap<Vec<SubgraphType>, HashMap<Vec<SubgraphType>, BTreeMap<String, f64>>>,
}

impl AggregateFunctionDistribution {
    pub fn parse_file( _file_path: PathBuf) -> AggregateFunctionDistribution {
        let content : HashMap<String, HashMap<String, HashMap<String, f64>>>;
        let data = fs::read_to_string(_file_path).expect("Unable to read file");
        content = serde_json::from_str(&data).unwrap();
        let mut filled_map : HashMap<Vec<SubgraphType>, HashMap<Vec<SubgraphType>, BTreeMap<String, f64>>> = HashMap::new();
        for (key, _) in &content {
            match key.as_str() {
                arm @ ("[numeric, numeric]" | "[numeric]" | "[text]" | "[Val3]") => {
                    for (inner_key, _) in &content[arm] {
                        match inner_key.as_str() {
                            inner_arm @ ("[numeric]" | "[text]" | "[Val3]") => {
                                let outer : Vec<SubgraphType> = match key.as_str() {
                                    "[numeric, numeric]" => vec![SubgraphType::Numeric, SubgraphType::Numeric],
                                    "[numeric]" => vec![SubgraphType::Numeric],
                                    "[text]" => vec![SubgraphType::Text],
                                    "[Val3]" => vec![SubgraphType::Val3],    
                                    any => panic!("found unknown domain type {} while filling aggregate functions structure.", key),
                                };
                                let inner : Vec<SubgraphType> = match inner_key.as_str() {
                                    "[numeric]" => vec![SubgraphType::Numeric],
                                    "[text]" => vec![SubgraphType::Text],
                                    "[Val3]" => vec![SubgraphType::Val3],    
                                    any => panic!("found unknown return type {} while filling aggregate functions structure.", key),
                                };
                                let mut tree : BTreeMap<String, f64> = BTreeMap::new();
                                for (func, weight) in &content[arm][inner_arm] {
                                    tree.insert(String::from(func), *weight);
                                }
                                let mut return_type_map : HashMap<Vec<SubgraphType>, BTreeMap<String, f64>> = HashMap::new();
                                return_type_map.insert(inner, tree);
                                filled_map.insert(outer, return_type_map);
                            },
                            any => panic!("found unknown return type {} while parsing json file.", key),
                        }
                    }
                },
                any => panic!("found unknown domain type {} while parsing json file.", key),
            }
        }
        AggregateFunctionDistribution {
            map : filled_map,
        }
    }  

    pub fn get_fun_name (&mut self, arg_domain: Vec<SubgraphType>, arg_return: Vec<SubgraphType>) -> sqlparser::ast::ObjectName {
        let dist = WeightedIndex::new(self.map[&arg_domain][&arg_return].iter().map(|item| item.1)).unwrap();
        let mut rng = thread_rng();
        let selected_name = self.map[&arg_domain][&arg_return].keys().nth(dist.sample(&mut rng)).unwrap();
        ObjectName(vec![Ident {
            value: selected_name.to_string(),
            quote_style: (None),
        }])
    }
}