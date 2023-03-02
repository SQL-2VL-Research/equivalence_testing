use std::{path::PathBuf, io::Write};

use equivalence_testing::{query_creation::{
    random_query_generator::{QueryGenerator},
    state_generators::{
        MarkovChainGenerator,
        state_choosers::{ProbabilisticStateChooser, StateChooser},
        dynamic_models::{DynamicModel, MarkovModel, AntiCallModel},
    },
}, equivalence_testing_function::{
    check_query, string_to_query
}};
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "basic")]
struct ProgramArgs {
    /// Path to graph source for a markov chain
    #[structopt(parse(from_os_str))]
    input: PathBuf,
    /// number of generated queries
    #[structopt(default_value = "20")]
    num_generate: usize,
    /// Use AntiCallModel for dynamic probabilities
    #[structopt(short, long)]
    anticall_model: bool,
}

fn run_generation<DynMod: DynamicModel, StC: StateChooser>(markov_generator: MarkovChainGenerator<StC>, num_generate: usize) {
    let mut generator = QueryGenerator::<DynMod, StC>::from_state_generator(markov_generator);

    let mut num_generated = 0;
    let mut num_equivalent = 0;
    for i in 0..num_generate {
        let query_ast = Box::new(generator.next().unwrap());
        let query_string = query_ast.to_string();
        // println!("Generated query: {query_string}");
        let parsed_ast = string_to_query(&query_string);
        if parsed_ast != query_ast {
            println!("AST mismatch! For query: {query_string}");
            let mut f_g = std::fs::File::create(format!("{i}-g")).unwrap();
            write!(f_g, "{:#?}", query_ast).unwrap();
            let mut f_p = std::fs::File::create(format!("{i}-p")).unwrap();
            write!(f_p, "{:#?}", parsed_ast).unwrap();
        }
        let equivalent = check_query(query_ast);
        // println!("Equivalent? {:#?}\n", equivalent);
        num_generated += 1;
        if equivalent {
            num_equivalent += 1;
        }
        if i % 1000 == 0 {
            print!("{}/{num_generate}      \r", i);
            std::io::stdout().flush().unwrap();
        }
    }
    println!("Equivalence: {} / {}", num_equivalent, num_generated);
}

fn select_model_and_run_generation<StC: StateChooser>(program_args: ProgramArgs) {
    let markov_generator = match MarkovChainGenerator::<StC>::parse_graph_from_file(
        &program_args.input
    ) {
        Ok(generator) => generator,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    if program_args.anticall_model {
        run_generation::<AntiCallModel, _>(markov_generator, program_args.num_generate);
    } else {
        run_generation::<MarkovModel, _>(markov_generator, program_args.num_generate);
    };
}

fn main() {
    let program_args = ProgramArgs::from_args();

    select_model_and_run_generation::<ProbabilisticStateChooser>(program_args);
}
