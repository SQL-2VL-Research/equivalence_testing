/// A small command-line app to run the parser.
/// Run with `cargo run --example eq_test`
use std::fs;

use sqlparser::ast::Query;
use sqlparser::dialect::{PostgreSqlDialect, Dialect};
use sqlparser::parser::Parser;

const ARGS_ERR_MSG: &str = r#"No arguments provided!
Usage:
    cargo run --example cli FILENAME1.sql FILENAME2.sql
"#;

/// read file contents, ignore BOM bytes if present
fn read_file_contents(filename: String) -> String {
    let contents = fs::read_to_string(&filename)
        .unwrap_or_else(|_| panic!("Unable to read file {}", &filename));
    if contents.chars().next().unwrap() as u64 != 0xfeff {
        contents
    } else {
        let mut chars = contents.chars();
        chars.next();
        chars.collect()
    }
}

fn file_to_query(filename: String) -> Box<Query> {
    let contents = read_file_contents(filename);

    // parse result using the dialect
    let dialect: Box<dyn Dialect> = Box::new(PostgreSqlDialect {});
    let statements = Parser::parse_sql(&*dialect, contents.as_str()).unwrap();
    match statements.into_iter().next().expect("No single query in sql file") {
        sqlparser::ast::Statement::Query(query) => query,
        _ => panic!("Query present in file is not a SELECT query.")
    }
}

fn main() {
    let query1 = file_to_query(std::env::args().nth(1).expect(ARGS_ERR_MSG));
    let query2 = file_to_query(std::env::args().nth(2).expect(ARGS_ERR_MSG));

    println!("First query: {}", query1.to_string());
    println!("Second query: {}", query2.to_string());
}