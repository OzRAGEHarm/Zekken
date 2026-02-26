use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::process;

mod ast;
mod lexer;
mod parser;
mod environment;
mod eval;
mod errors;
mod libraries;

use parser::Parser as ZkParser;
use eval::statement::evaluate_statement;
use environment::{Environment, Value};
use ast::Stmt;
use errors::{extract_exit_code, push_error, print_and_clear_errors};

/// Zekken Language CLI
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new Zekken project
    Init {
        /// Use default values
        #[arg(short, long)]
        default: bool,
    },

    /// Run a Zekken script file
    Run {
        /// The script file to run
        file: String,
        /// Skip pre-execution lint pass for performance benchmarking
        #[arg(long)]
        no_lint: bool,
    },

    /// Start a Zekken REPL
    Repl,
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Init { default } => {
            let (name, version, entry_point, author, description) = if *default {
                ("zekken_project".to_string(), "0.0.1".to_string(), "main.zk".to_string(), "".to_string(), "A Zekken Package".to_string())
            } else {
                let mut input = String::new();
                print!("Project name: ");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut input).unwrap();
                let name = input.trim().to_string();
                input.clear();

                print!("Version (default 0.0.1): ");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut input).unwrap();
                let version = if input.trim().is_empty() { "0.0.1".to_string() } else { input.trim().to_string() };
                input.clear();

                print!("Entry Point (default main.zk): ");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut input).unwrap();
                let entry_point = if input.trim().is_empty() { "main.zk".to_string() } else { input.trim().to_string() };
                input.clear();

                print!("Author: ");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut input).unwrap();
                let author = input.trim().to_string();
                input.clear();

                print!("Description: ");
                io::stdout().flush().unwrap();
                io::stdin().read_line(&mut input).unwrap();
                let description = input.trim().to_string();

                (name, version, entry_point, author, description)
            };

            // This is the modified section.
            let manifest = format!(
"[package]

name = \"{}\"
version = \"{}\"
entry_point = \"{}\"
author = \"{}\"
description = \"{}\"

[dependencies]
",
                name, version, entry_point, author, description
            );

            // This is the modified file creation.
            fs::write("Zekken.toml", manifest).expect("Failed to write package manifest file.");
            fs::write(&entry_point, "@println => |\"Hello World!\"|\n").expect("Failed to create entry point file.");
            println!("Initialized new Zekken project.");
        }
        Commands::Run { file, no_lint } => {
            std::env::set_var("ZEKKEN_CURRENT_FILE", file);
            if *no_lint {
                std::env::set_var("ZEKKEN_NO_LINT", "1");
            } else {
                std::env::remove_var("ZEKKEN_NO_LINT");
            }
            let source_code = fs::read_to_string(file).unwrap_or_else(|err| {
                eprintln!("Error reading file {}: {}", file, err);
                process::exit(1)
            });

            let mut parser = ZkParser::new();
            let ast = parser.produce_ast(source_code);

            // Push all syntax errors to the global error list
            for error in &parser.errors {
                push_error(error.clone());
            }

            // Stop immediately when parsing failed.
            if !parser.errors.is_empty() {
                let _ = print_and_clear_errors();
                process::exit(1);
            }

            let mut env = Environment::new();

            let file_path = std::path::Path::new(file);
            let current_dir = file_path.parent()
                .unwrap_or_else(|| std::path::Path::new(""))
                .to_string_lossy()
                .to_string();

            env.declare("ZEKKEN_CURRENT_DIR".to_string(), Value::String(current_dir), false);

            // Evaluate and push all runtime/type/reference errors to the global error list
            let result = match evaluate_statement(&Stmt::Program(ast), &mut env) {
                Ok(val) => Some(val),
                Err(e) => {
                    if let Some(code) = extract_exit_code(&e.message) {
                        process::exit(code);
                    }

                    // Internal errors are control markers; user-facing errors are already collected.
                    if e.kind != crate::errors::ErrorKind::Internal {
                        push_error(e);
                    }
                    None
                }
            };

            // Print all errors (syntax, runtime, etc.) and exit if any
            if print_and_clear_errors() {
                std::process::exit(1);
            }

            // Only print result if there were no errors at all
            io::stdout().flush().unwrap();
            match result.flatten() {
                Some(Value::Void) => (),
                Some(value) => println!("{}", value),
                None => ()
            }
            process::exit(0);
        }
        Commands::Repl => {
            // Enable REPL-friendly error formatting
            *errors::REPL_MODE.lock().unwrap() = true;
            println!("Zekken REPL (type 'exit' or Ctrl+C to quit)");
            let mut env = Environment::new();
            loop {
                print!("> ");
                io::stdout().flush().unwrap();
                let mut input = String::new();
                if io::stdin().read_line(&mut input).is_err() {
                    break;
                }
                let line = input.trim();
                if line == "exit" || line == "quit" {
                    break;
                }
                if line.is_empty() {
                    continue;
                }
                let mut parser = ZkParser::new();
                let ast = parser.produce_ast(line.to_string());
                for error in &parser.errors {
                    println!("{}", error); // Will use REPL-friendly format
                }
                if !parser.errors.is_empty() {
                    continue;
                }
                match evaluate_statement(&Stmt::Program(ast), &mut env) {
                    Ok(Some(Value::Void)) | Ok(None) => {}
                    Ok(Some(val)) => println!("{}", val),
                    Err(e) => {
                        if let Some(_code) = extract_exit_code(&e.message) {
                            break;
                        }
                        println!("{}", e)
                    }, // Will use REPL-friendly format
                }
            }
            // Disable REPL mode after exiting
            *errors::REPL_MODE.lock().unwrap() = false;
        }
    }
}
