mod ast;
mod parser;
mod environment;
mod bytecode;
mod errors;
mod lexer;
mod libraries;
mod eval;
mod diagnostics;

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
            env.declare_ref_typed(
                "println",
                Value::NativeFunction(Arc::new(|args: Vec<Value>| -> Result<Value, String> {
                    let mut buf = WASM_OUTPUT.lock().unwrap();
                    let line = environment::format_print_values(&args);
                    buf.push_str(&line);
                    buf.push('\n');
                    Ok(Value::Void)
                })),
                crate::lexer::DataType::Fn,
                true,
            );
        }
    }

    let report = diagnostics::run_program_collecting(
        &ast,
        &parser.errors,
        &mut env,
        diagnostics::ExecutionMode::Bytecode,
    );
    let mut output = String::new();
    for error in &report.errors {
        output.push_str(&format!("{}\n", error));
    }
    if report.errors.is_empty() {
        if let Some(val) = report.value {
            if !matches!(val, environment::Value::Void) {
                output.push_str(&format!("{}\n", val));
            }
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        output.push_str(&WASM_OUTPUT.lock().unwrap());
    }

    output
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn clear_errors() {
    crate::errors::clear_collected_errors();
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Stmt;
    use crate::environment::{Environment, Value};
    use crate::lexer::DataType;
    use std::sync::{Arc, Mutex};

    fn parse(source: &str) -> ast::Program {
        let mut parser = parser::Parser::new();
        let program = parser.produce_ast(source.to_string());
        assert!(parser.errors.is_empty(), "parser errors: {:#?}", parser.errors);
        program
    }

    fn execute(source: &str, use_vm: bool, env: &mut Environment) {
        let program = parse(source);
        let result = if use_vm {
            bytecode::execute_program(&program, env)
        } else {
            eval::statement::evaluate_statement(&Stmt::Program(program), env)
        };
        assert!(result.is_ok(), "execution failed: {result:#?}");
    }

    #[test]
    fn recursive_functions_keep_declaration_scope_builtins() {
        let source = r#"
func countdown |n: int| {
    if n > 0 {
        @println => |n|
        countdown => |n - 1|
    }
}

countdown => |3|
"#;

        for use_vm in [false, true] {
            let output = Arc::new(Mutex::new(Vec::new()));
            let sink = Arc::clone(&output);
            let mut env = Environment::new();
            env.declare_ref_typed(
                "println",
                Value::NativeFunction(Arc::new(move |args| {
                    if let [Value::Int(value)] = args.as_slice() {
                        sink.lock().unwrap().push(*value);
                        Ok(Value::Void)
                    } else {
                        Err("expected one integer".to_string())
                    }
                })),
                DataType::Fn,
                true,
            );

            execute(source, use_vm, &mut env);
            assert_eq!(*output.lock().unwrap(), vec![3, 2, 1]);
        }
    }

    #[test]
    fn optimized_identifier_loops_continue_to_explicit_return() {
        let source = r#"
func collect |n: int| {
    let out: arr = [];
    let i: int = 0;
    while i < n {
        out += [i]
        i += 1
    }
    return out;
}

let result: arr = collect => |3|;
"#;

        for use_vm in [false, true] {
            let mut env = Environment::new();
            execute(source, use_vm, &mut env);

            let values = match env.lookup_ref("result") {
                Some(Value::Array(values)) => values,
                other => panic!("expected array result, got {other:#?}"),
            };
            assert!(matches!(values.as_slice(), [Value::Int(0), Value::Int(1), Value::Int(2)]));
        }
    }

    #[test]
    fn diagnostics_collect_and_order_all_error_categories() {
        let source = r#"
let broken: int = 1
let missing: int = absent => ||;
let wrong: string = 2;
@fail => ||
"#;

        for mode in [
            diagnostics::ExecutionMode::TreeWalk,
            diagnostics::ExecutionMode::Bytecode,
        ] {
            let mut parser = parser::Parser::new();
            let program = parser.produce_ast(source.to_string());
            let mut env = Environment::new();
            env.declare_ref_typed(
                "fail",
                Value::NativeFunction(Arc::new(|_| Err("deliberate failure".to_string()))),
                DataType::Fn,
                true,
            );

            let report = diagnostics::run_program_collecting(
                &program,
                &parser.errors,
                &mut env,
                mode,
            );
            let kinds: Vec<errors::ErrorKind> =
                report.errors.iter().map(|error| error.kind.clone()).collect();
            assert_eq!(
                kinds,
                vec![
                    errors::ErrorKind::Syntax,
                    errors::ErrorKind::Reference,
                    errors::ErrorKind::Type,
                    errors::ErrorKind::Runtime,
                ]
            );

            let mut with_internal = report.errors;
            with_internal.push(errors::ZekkenError::internal("deliberate internal failure"));
            errors::sort_and_dedup_errors(&mut with_internal);
            assert_eq!(
                with_internal.last().map(|error| &error.kind),
                Some(&errors::ErrorKind::Internal)
            );
        }
    }

    #[test]
    fn rejects_semicolon_after_expression_statement_and_recovers() {
        let source = "@println => |\"Hello, World!\"|;\n@println => |\"Still parsed\"|\n";
        let mut parser = parser::Parser::new();
        let program = parser.produce_ast(source.to_string());

        assert_eq!(parser.errors.len(), 1, "parser errors: {:#?}", parser.errors);
        let error = &parser.errors[0];
        assert_eq!(error.kind, errors::ErrorKind::Syntax);
        assert_eq!(error.message, "Unexpected ';' after expression");
        assert_eq!(error.context.line, 1);
        assert_eq!(error.context.column, source.find(';').unwrap() + 1);
        assert_eq!(program.content.len(), 2, "parser did not recover after semicolon");
    }

    #[test]
    fn distinguishes_empty_call_pipes_from_logical_or() {
        let source = r#"
let left: bool = true;
let right: bool = false;
let compared: bool = queue.is_empty => || == false;
let combined: bool = left || right;
if left || right {
    left = false
}
while left || right {
    left = false
    right = false
}
for |item| in left || right {
    item
}
let length: int = data.length => ||;
@println => |combined|
"#;

        let mut parser = parser::Parser::new();
        let program = parser.produce_ast(source.to_string());

        assert!(parser.errors.is_empty(), "parser errors: {:#?}", parser.errors);
        assert_eq!(program.content.len(), 9);
    }

    #[test]
    fn diagnostics_deduplicate_only_exact_errors() {
        let duplicate = errors::ZekkenError::internal("duplicate");
        let mut different_details = duplicate.clone();
        different_details.extra = Some("different details".to_string());
        let mut collected = vec![duplicate.clone(), different_details, duplicate];

        errors::sort_and_dedup_errors(&mut collected);

        assert_eq!(collected.len(), 2);
        assert!(collected.iter().any(|error| error.extra.is_none()));
        assert!(collected.iter().any(|error| error.extra.is_some()));
    }
}
