use crate::environment::{Environment, Value, FunctionValue};
use crate::ast::{*};
use crate::lexer::{*};
use std::collections::HashMap;

pub fn register(env: &mut Environment) -> Result<(), String> {
    let mut math_obj = HashMap::new();

    // Constants
    math_obj.insert("PI".to_string(), Value::Float(std::f64::consts::PI));
    math_obj.insert("E".to_string(), Value::Float(std::f64::consts::E));
    math_obj.insert("I".to_string(), Value::Complex { real: 0.0, imag: 1.0 });

    // Helper function to create a parameter
    fn create_param(name: &str, type_: DataType) -> Param {
        Param {
            ident: name.to_string(),
            type_,
            location: Location { line: 0, column: 0 }
        }
    }

    // Basic Math Functions
    math_obj.insert("sqrt".to_string(), Value::Function(FunctionValue {
        params: vec![create_param("x", DataType::Float)],
        body: vec![Box::new(Content::Expression(Box::new(Expr::FloatLit({
            FloatLit {
                value: 0.0,  // Placeholder - actual computation happens at runtime
                location: Location { line: 0, column: 0 }
            }
        }))))]
    }));

    math_obj.insert("pow".to_string(), Value::Function(FunctionValue {
        params: vec![
            create_param("base", DataType::Float),
            create_param("exp", DataType::Float)
        ],
        body: vec![Box::new(Content::Expression(Box::new(Expr::FloatLit({
            FloatLit {
                value: 0.0,
                location: Location { line: 0, column: 0 }
            }
        }))))]
    }));

    math_obj.insert("abs".to_string(), Value::Function(FunctionValue {
        params: vec![create_param("x", DataType::Float)],
        body: vec![Box::new(Content::Expression(Box::new(Expr::FloatLit({
            FloatLit {
                value: 0.0,
                location: Location { line: 0, column: 0 }
            }
        }))))]
    }));

    // Trigonometric Functions
    math_obj.insert("sin".to_string(), Value::Function(FunctionValue {
        params: vec![create_param("x", DataType::Float)],
        body: vec![Box::new(Content::Expression(Box::new(Expr::FloatLit({
            FloatLit {
                value: 0.0,
                location: Location { line: 0, column: 0 }
            }
        }))))]
    }));

    math_obj.insert("cos".to_string(), Value::Function(FunctionValue {
        params: vec![create_param("x", DataType::Float)],
        body: vec![Box::new(Content::Expression(Box::new(Expr::FloatLit({
            FloatLit {
                value: 0.0,
                location: Location { line: 0, column: 0 }
            }
        }))))]
    }));

    math_obj.insert("tan".to_string(), Value::Function(FunctionValue {
        params: vec![create_param("x", DataType::Float)],
        body: vec![Box::new(Content::Expression(Box::new(Expr::FloatLit({
            FloatLit {
                value: 0.0,
                location: Location { line: 0, column: 0 }
            }
        }))))]
    }));

    // Vector Operations
    math_obj.insert("vector".to_string(), Value::Function(FunctionValue {
        params: vec![create_param("values", DataType::Array)],
        body: vec![Box::new(Content::Expression(Box::new(Expr::ArrayLit({
            ArrayLit {
                elements: vec![],
                location: Location { line: 0, column: 0 }
            }
        }))))]
    }));

    math_obj.insert("dot".to_string(), Value::Function(FunctionValue {
        params: vec![
            create_param("v1", DataType::Array),
            create_param("v2", DataType::Array)
        ],
        body: vec![Box::new(Content::Expression(Box::new(Expr::FloatLit({
            FloatLit {
                value: 0.0,
                location: Location { line: 0, column: 0 }
            }
        }))))]
    }));

    // Matrix Operations 
    math_obj.insert("matrix".to_string(), Value::Function(FunctionValue {
        params: vec![create_param("rows", DataType::Array)],
        body: vec![Box::new(Content::Expression(Box::new(Expr::ArrayLit({
            ArrayLit {
                elements: vec![],
                location: Location { line: 0, column: 0 }
            }
        }))))]
    }));

    // Register either full module or specific imports
    if let Some(Value::Array(methods)) = env.lookup("__IMPORT_METHODS__") {
        // Specific imports
        for method in methods {
            if let Value::String(name) = method {
                if let Some(value) = math_obj.get(&name) {
                    env.declare(name, value.clone(), true);
                } else {
                    return Err(format!("Math module error: '{}' not found", name));
                }
            }
        }
    } else {
        // Full module import
        env.declare("math".to_string(), Value::Object(math_obj), true);
    }

    Ok(())
}