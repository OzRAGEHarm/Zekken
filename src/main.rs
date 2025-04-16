use std::env;
use std::fs;
use std::process;
use std::io::Write;

mod ast;
mod lexer;
mod parser;
mod environment;
mod eval;
mod errors;
mod libraries;

use parser::Parser;
use eval::statement::evaluate_statement;
use environment::{Environment, Value};
use ast::Stmt;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <filename>", args[0]);
        process::exit(1);
    }

    let filename = &args[1];
    std::env::set_var("ZEKKEN_CURRENT_FILE", filename);
    let source_code = fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Error reading file {}: {}", filename, err);
        process::exit(1)
    });

    let mut parser = Parser::new();
    let ast = parser.produce_ast(source_code);

    //println!("{:#?}", ast);

    /*
    let source = env::var("ZEKKEN_SOURCE_LINES").unwrap_or_else(|_| "<unknown>".to_string());
    let tokens = env::var("ZEKKEN_TOKENS").unwrap_or_else(|_| "<unknown>".to_string());

    println!("Source code: \n{}", source);
    println!("Tokens: {}", tokens);

    */
    
    let mut env = Environment::new();

    let file_path = std::path::Path::new(filename);
    let current_dir = file_path.parent()
        .unwrap_or_else(|| std::path::Path::new(""))
        .to_string_lossy()
        .to_string();

    env.declare("ZEKKEN_CURRENT_DIR".to_string(), Value::String(current_dir), false);

    match evaluate_statement(&Stmt::Program(ast), &mut env) {
        Ok(result) => {
            std::io::stdout().flush().unwrap();
            match result {
                Some(Value::Void) => (),
                Some(value) => println!("{}", value),
                None => ()
            }
            process::exit(0)
        },
        Err(error) => {
            eprintln!("{}", error);
            process::exit(1);
        }
    }
}