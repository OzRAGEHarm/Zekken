use clap::{Parser, Subcommand};
use std::fs;
use std::io::{self, Write};
use std::process;

mod ast;
mod lexer;
mod parser;
mod environment;
mod bytecode;
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
    /// Run a Zekken script file
    Run {
        /// The script file to run
        file: String,
        /// Run using the register bytecode VM in src/bytecode
        #[arg(long)]
        vm: bool,
        /// Extra script arguments forwarded to the running Zekken program
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        script_args: Vec<String>,
    },

    /// Start a Zekken REPL
    Repl,

    /// Debug helpers (lexer/AST dumps)
    Debug {
        #[command(subcommand)]
        command: DebugCommands,
    },
}

#[derive(Subcommand)]
enum DebugCommands {
    /// Print the lexer token stream for a source file
    Tokens {
        /// The script file to tokenize
        file: String,
    },

    /// Parse and print the AST for a source file
    Ast {
        /// The script file to parse
        file: String,
    },
}

fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Run { file, vm, script_args: _ } => {
            std::env::set_var("ZEKKEN_CURRENT_FILE", file);
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
            let result = match if *vm {
                bytecode::execute_program(&ast, &mut env)
            } else {
                evaluate_statement(&Stmt::Program(ast), &mut env)
            } {
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

            // `zekken run` does not implicitly print the last expression value.
            // Use `@println` for output. (REPL remains expression-result oriented.)
            let _ = result; // keep evaluation for side effects
            io::stdout().flush().unwrap();
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
        Commands::Debug { command } => match command {
            DebugCommands::Tokens { file } => {
                std::env::set_var("ZEKKEN_CURRENT_FILE", file);
                let source_code = fs::read_to_string(file).unwrap_or_else(|err| {
                    eprintln!("Error reading file {}: {}", file, err);
                    process::exit(1)
                });

                let tokens = lexer::tokenize(source_code);
                for (i, t) in tokens.iter().enumerate() {
                    println!(
                        "{:04}  line={:<4} col={:<4}  kind={:?}  value={:?}",
                        i, t.line, t.column, t.kind, t.value
                    );
                }
                process::exit(0);
            }
            DebugCommands::Ast { file } => {
                std::env::set_var("ZEKKEN_CURRENT_FILE", file);
                let source_code = fs::read_to_string(file).unwrap_or_else(|err| {
                    eprintln!("Error reading file {}: {}", file, err);
                    process::exit(1)
                });

                let mut parser = ZkParser::new();
                let ast = parser.produce_ast(source_code);

                for error in &parser.errors {
                    push_error(error.clone());
                }
                if print_and_clear_errors() {
                    process::exit(1);
                }

                println!("{:#?}", ast);
                process::exit(0);
            }
        },
    }
}
