use std::io::Write;

use equivalence_testing::{query_creation::{
    random_query_generator::{QueryGenerator, QueryGeneratorConfig},
    state_generators::{
        MarkovChainGenerator,
        state_choosers::{ProbabilisticStateChooser, StateChooser},
        dynamic_models::{DynamicModel, MarkovModel, AntiCallModel},
    },
}, equivalence_testing_function::{
    check_query, string_to_query
}, config::{Config, ProgramArgs, MainConfig}};

use structopt::StructOpt;

fn run_generation<DynMod: DynamicModel, StC: StateChooser>(markov_generator: MarkovChainGenerator<StC>, generator_config: QueryGeneratorConfig, main_config: MainConfig) {
    let mut generator = QueryGenerator::<DynMod, StC>::from_state_generator_and_schema(markov_generator, generator_config);

    let mut num_equivalent = 0;
    for i in 0..main_config.num_generate {
        let query_ast = Box::new(generator.next().unwrap());
        if main_config.assert_parcing_equivalence {
            let query_string = query_ast.to_string();
            let parsed_ast = string_to_query(&query_string);
            if parsed_ast != query_ast {
                println!("AST mismatch! For query: {query_string}");
                let mut f_g = std::fs::File::create(format!("{i}-g")).unwrap();
                write!(f_g, "{:#?}", query_ast).unwrap();
                let mut f_p = std::fs::File::create(format!("{i}-p")).unwrap();
                write!(f_p, "{:#?}", parsed_ast).unwrap();
            }
        }
        if main_config.count_equivalence {
            num_equivalent += check_query(query_ast) as usize;
        }
        if i % 100 == 0 {
            print!("{}/{}      \r", i, main_config.num_generate);
            std::io::stdout().flush().unwrap();
        }
    }
    if main_config.count_equivalence {
        println!("Equivalence: {} / {}", num_equivalent, main_config.num_generate);
    }
}

fn select_model_and_run_generation<StC: StateChooser>(config: Config) {
    let markov_generator = match MarkovChainGenerator::<StC>::with_config(config.chain_config) {
        Ok(generator) => generator,
        Err(err) => {
            println!("{}", err);
            return;
        }
    };

    if config.generator_config.dynamic_model_name == "anticall" {
        run_generation::<AntiCallModel, _>(markov_generator, config.generator_config, config.main_config);
    } else {
        run_generation::<MarkovModel, _>(markov_generator, config.generator_config, config.main_config);
    };
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let program_args = ProgramArgs::from_args();
    let mut config = Config::read_config(&program_args.config_path)?;
    config.update_from_args(&program_args);
    select_model_and_run_generation::<ProbabilisticStateChooser>(config);
    Ok(())
}
