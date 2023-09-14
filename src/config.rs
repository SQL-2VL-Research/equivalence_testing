use std::path::PathBuf;

use structopt::StructOpt;
use toml::Value;

use crate::query_creation::{random_query_generator::QueryGeneratorConfig, state_generators::markov_chain_generator::StateGeneratorConfig};

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
pub struct ProgramArgs {
    /// config file
    #[structopt(parse(from_os_str), default_value = "config.toml")]
    pub config_path: PathBuf,
    /// Number of queries to generate (if provided, otherwise sourced from config)
    #[structopt(short = "n", long = "num_generate")]
    pub num_generate: Option<usize>,
    /// Print generated queries, overriding config file settings
    #[structopt(short = "p", long = "print")]
    pub print: bool,
}

pub trait TomlReadable {
    fn from_toml(toml_config: &Value) -> Self;
}

pub struct MainConfig {
    pub num_generate: usize,
    pub count_equivalence: bool,
    pub measure_generation_time: bool,
    pub assert_parcing_equivalence: bool,
    pub assert_runs_on_schema: bool,
    pub print_progress: bool,
}

impl TomlReadable for MainConfig {
    fn from_toml(toml_config: &toml::Value) -> Self {
        let section = &toml_config["main"];
        Self {
            num_generate: section["num_generate"].as_integer().unwrap() as usize,
            count_equivalence: section["count_equivalence"].as_bool().unwrap(),
            measure_generation_time: section["measure_generation_time"].as_bool().unwrap(),
            assert_parcing_equivalence: section["assert_parcing_equivalence"].as_bool().unwrap(),
            assert_runs_on_schema: section["assert_runs_on_schema"].as_bool().unwrap(),
            print_progress: section["print_progress"].as_bool().unwrap(),
        }
    }
}

pub struct Config {
    pub main_config: MainConfig,
    pub generator_config: QueryGeneratorConfig,
    pub chain_config: StateGeneratorConfig,
}

fn read_toml_config(config_path: &PathBuf) -> Result<Value, Box<dyn std::error::Error>> {
    let config_source = std::fs::read_to_string(config_path)?;
    Ok(toml::from_str(&config_source)?)
}

impl Config {
    pub fn read_config(config_path: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let toml_config = read_toml_config(config_path)?;
        Ok(Self {
            main_config: MainConfig::from_toml(&toml_config),
            generator_config: QueryGeneratorConfig::from_toml(&toml_config),
            chain_config: StateGeneratorConfig::from_toml(&toml_config),
        })
    }

    pub fn update_from_args(&mut self, program_args: &ProgramArgs) {
        if let Some(num_generate) = program_args.num_generate {
            self.main_config.num_generate = num_generate;
        }
        if program_args.print {
            self.generator_config.print_queries = true;
        }
    }
}
