mod ast;
mod parser;
mod environment;
mod eval;
mod errors;
mod lexer;
mod libraries;

use wasm_bindgen::prelude::*;

#[cfg(target_arch = "wasm32")]
use std::sync::Mutex;

#[cfg(target_arch = "wasm32")]
lazy_static::lazy_static! {
    static ref WASM_OUTPUT: Mutex<String> = Mutex::new(String::new());
}

#[wasm_bindgen]
pub fn run_zekken(input: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();

    // Store source lines globally for error context in WASM
    #[cfg(target_arch = "wasm32")]
    {
        crate::errors::set_wasm_source_lines(input, "main.zk");
    }

    let mut parser = parser::Parser::new();
    let ast = parser.produce_ast(input.to_string());
    let mut env = environment::Environment::new();

    #[cfg(target_arch = "wasm32")]
    {
        WASM_OUTPUT.lock().unwrap().clear();
        {
            use environment::Value;
            use std::sync::Arc;
            env.variables.insert(
                "println".to_string(),
                Value::NativeFunction(Arc::new(|args: Vec<Value>| -> Result<Value, String> {
                    let mut buf = WASM_OUTPUT.lock().unwrap();
                    for (i, val) in args.iter().enumerate() {
                        if i > 0 {
                            buf.push_str(" ");
                        }
                        buf.push_str(&val.to_string());
                    }
                    buf.push('\n');
                    Ok(Value::Void)
                })),
            );
        }
    }

    let mut output = String::new();

    for error in &parser.errors {
        output.push_str(&format!("{}\n", error));
    }
    if !parser.errors.is_empty() {
        return output;
    }
    match eval::statement::evaluate_statement(&ast::Stmt::Program(ast), &mut env) {
        Ok(Some(val)) if !matches!(val, environment::Value::Void) => {
            output.push_str(&format!("{}\n", val));
        }
        Err(e) => output.push_str(&format!("{}\n", e)), // Use Display, not to_repl_string
        _ => {}
    }

    #[cfg(target_arch = "wasm32")]
    {
        output.push_str(&WASM_OUTPUT.lock().unwrap());
    }

    output
}
