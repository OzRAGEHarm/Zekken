mod ast;
mod parser;
mod environment;
mod eval;
mod errors;
mod lexer;
mod libraries;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn run_zekken(input: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    let mut parser = parser::Parser::new();
    let ast = parser.produce_ast(input.to_string());
    let mut env = environment::Environment::new();
    let mut output = String::new();

    for error in &parser.errors {
        output.push_str(&format!("{}\n", error.to_repl_string()));
    }
    if !parser.errors.is_empty() {
        return output;
    }
    match eval::statement::evaluate_statement(&ast::Stmt::Program(ast), &mut env) {
        Ok(Some(val)) if !matches!(val, environment::Value::Void) => {
            output.push_str(&format!("{}\n", val));
        }
        Err(e) => output.push_str(&format!("{}\n", e.to_repl_string())),
        _ => {}
    }
    output
}
