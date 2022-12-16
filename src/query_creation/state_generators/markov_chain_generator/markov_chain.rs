use std::path::PathBuf;

struct MarkovChain {
    parser: DotParser,
}

impl MarkovChain {
    fn from_file(graph_source_file_path: PathBuf) -> Self {
        MarkovChain {
            parser: DotParser::from_file(graph_source_file_path),
        }
    }
}

pub struct MarkovChainGenerator {
    markov_chain: MarkovChain
}

impl MarkovChainGenerator {
    pub fn from_file(graph_source_file_path: PathBuf) -> Self {
        MarkovChainGenerator {
            markov_chain: MarkovChain::from_file(graph_source_file_path)
        }
    }
}
