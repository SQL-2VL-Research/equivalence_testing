use std::path::PathBuf;

use equivalence_testing::query_creation::{
    random_query_generator::{QueryGenerator},
    state_generators::MarkovChainGenerator,
};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct ProgramArgs {
    /// Path to graph source for a markov chain
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    /// number of generated queries
    #[structopt(default_value = "10")]
    num_generate: usize,
}

fn main() {
    let program_args = ProgramArgs::from_args();
    let markov_generator = match MarkovChainGenerator::parse_graph_from_file(program_args.input) {
        Ok(generator) => generator,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    let mut generator = QueryGenerator::from_state_generator(markov_generator);

    for _ in 0..program_args.num_generate {
        let query = generator.next().unwrap();
        println!("Generated query: {:#?}", query.to_string());
    }
}
