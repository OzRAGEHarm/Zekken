use std::env;
use std::fs;
use std::process;

mod lexer; // Assuming your lexer is in a module named `lexer`
mod parser; // Assuming your parser is in a module named `parser`
mod ast; // Assuming your AST is in a module named `ast`

fn main() {
    // Get the file name from command line arguments
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <filename>", args[0]);
        process::exit(1);
    }

    let filename = &args[1];

    // Read the file content
    let source_code = fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Error reading file {}: {}", filename, err);
        process::exit(1);
    });

    // Tokenize the source code
    let mut parser = parser::Parser::new();
    let ast = parser.produce_ast(source_code);

    // Print the AST for debugging
    println!("{:#?}", ast);
}