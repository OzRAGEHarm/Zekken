use crate::ast::*;
use crate::environment::{Environment, Value, FunctionValue};
use crate::parser::Parser;
use super::expression::evaluate_expression;
use crate::errors::{ZekkenError, RuntimeErrorType, Location, runtime_error, type_error};
use crate::libraries::load_library;
use crate::lexer::DataType;
use std::env;
use std::collections::HashMap;

// Check if the value type matches the expected type
fn check_value_type(value: &Value, expected: &DataType) -> bool {
    match (value, expected) {
        (Value::Int(_), DataType::Int) => true,
        (Value::Float(_), DataType::Float) => true,
        (Value::String(_), DataType::String) => true,
        (Value::Boolean(_), DataType::Bool) => true,
        (Value::Array(_), DataType::Array) => true,
        (Value::Object(_), DataType::Object) => true,
        (Value::Function(_), DataType::Fn) => true,
        _ => false,
    }
}

// Evaluate a statement and return the result
pub fn evaluate_statement(stmt: &Stmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match stmt {
        Stmt::Program(program) => evaluate_program(program, env),
        Stmt::VarDecl(var_decl) => evaluate_var_declaration(var_decl, env),
        Stmt::FuncDecl(func_decl) => evaluate_function_declaration(func_decl, env),
        Stmt::ObjectDecl(obj_decl) => evaluate_object_declaration(obj_decl, env),
        Stmt::IfStmt(if_stmt) => evaluate_if_statement(if_stmt, env),
        Stmt::ForStmt(for_stmt) => evaluate_for_statement(for_stmt, env),
        Stmt::WhileStmt(while_stmt) => evaluate_while_statement(while_stmt, env),
        Stmt::TryCatchStmt(try_catch) => evaluate_try_catch(try_catch, env),
        Stmt::BlockStmt(block) => evaluate_block(block, env),
        Stmt::Return(ret) => evaluate_return(ret, env),
        Stmt::Lambda(lambda) => evaluate_lambda(lambda, env),
        Stmt::Use(use_stmt) => evaluate_use(use_stmt, env),
        Stmt::Include(include) => evaluate_include(include, env),
        Stmt::Export(exports) => evaluate_export(exports, env),
    }
}

// Evaluate the entire program
fn evaluate_program(program: &Program, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    //let current_file = env::var("ZEKKEN_CURRENT_FILE").unwrap_or_default();
    
    // Process imports first
    for import in &program.imports {
        if let Content::Statement(stmt) = &*import {
            match **stmt {
                Stmt::Include(ref include) => evaluate_include(include, env)?,
                Stmt::Use(ref use_stmt) => evaluate_use(use_stmt, env)?,
                _ => return Err(runtime_error(
                    "Invalid import statement",
                    RuntimeErrorType::SyntaxError,
                    0,
                    0
                ))
            };
        }
    }

    // Process main content
    let mut last_value = None;
    for content in &program.content {
        match &**content {
            Content::Statement(stmt) => {
                last_value = evaluate_statement(stmt, env)?;
            },
            Content::Expression(expr) => {
                last_value = Some(evaluate_expression(expr, env)?);
            }
        }
    }

    Ok(last_value)
}

// Handle variable declarations
fn evaluate_var_declaration(decl: &VarDecl, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let value = match &decl.value {
        Some(content) => match content {
            Content::Expression(expr) => {
                let val = evaluate_expression(expr, env)?;
                if !check_value_type(&val, &decl.type_) {
                    return Err(type_error(
                        &format!("Type mismatch in variable declaration '{}'", decl.ident),
                        &format!("{:?}", decl.type_),
                        &format!("{:?}", val),
                        Location {
                            filename: env::var("ZEKKEN_CURRENT_FILE").unwrap_or_default(),
                            line: decl.location.line,
                            column: decl.location.column,
                            line_content: env::var("ZEKKEN_SOURCE_LINES")
                                .unwrap_or_default()
                                .lines()
                                .nth(decl.location.line - 1)
                                .unwrap_or("")
                                .to_string(),
                        }
                    ));
                }
                val // Return the value directly instead of Some(val)
            },
            Content::Statement(stmt) => {
                if let Some(val) = evaluate_statement(stmt, env)? {
                    val
                } else {
                    Value::Void
                }
            },
        },
        None => Value::Void,
    };

    env.declare(decl.ident.clone(), value.clone(), decl.constant);
    Ok(Some(value))
}

// Handle function declarations
fn evaluate_function_declaration(func: &FuncDecl, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let function_value = FunctionValue {
        params: func.params.clone(),
        body: func.body.clone(),
    };

    env.declare(func.ident.clone(), Value::Function(function_value), false);
    Ok(None)
}

// Handle object declarations
fn evaluate_object_declaration(obj: &ObjectDecl, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let mut object_map = HashMap::new();
    let mut keys = Vec::new();
    for property in &obj.properties {
        let value = evaluate_expression(&property.value, env)
            .map_err(|e| runtime_error(
                &format!("Failed to evaluate property '{}': {}", property.key, e),
                RuntimeErrorType::TypeError,
                obj.location.line,
                obj.location.column
            ))?;
        keys.push(property.key.clone());
        object_map.insert(property.key.clone(), value);
    }
    // Store key order as a hidden property
    object_map.insert("__keys__".to_string(), Value::Array(keys.iter().map(|k| Value::String(k.clone())).collect()));
    env.declare(obj.ident.clone(), Value::Object(object_map), false);
    Ok(None)
}

// Handle if statements
fn evaluate_if_statement(if_stmt: &IfStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let test_result = evaluate_expression(&if_stmt.test, env)?;
    
    match test_result {
        Value::Boolean(true) => evaluate_block_content(&if_stmt.body, env),
        Value::Boolean(false) => {
            if let Some(alt) = &if_stmt.alt {
                evaluate_block_content(alt, env)
            } else {
                Ok(None)
            }
        }
        _ => Err(runtime_error(
            "If statement condition must evaluate to a boolean",
            RuntimeErrorType::TypeError,
            if_stmt.location.line,
            if_stmt.location.column
        ))
    }
}

// Handle for statements
fn evaluate_for_statement(for_stmt: &ForStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    if let Some(ref init) = for_stmt.init {
        if let Stmt::VarDecl(var_decl) = &**init {
            let collection_value = match &var_decl.value {
                Some(content) => match content {
                    Content::Expression(expr) => evaluate_expression(expr, env)?,
                    _ => return Err(runtime_error(
                        "Expected expression in for loop initialization",
                        RuntimeErrorType::SyntaxError,
                        for_stmt.location.line,
                        for_stmt.location.column
                    )),
                },
                None => return Err(runtime_error(
                    "For loop initialization requires a value",
                    RuntimeErrorType::SyntaxError,
                    for_stmt.location.line,
                    for_stmt.location.column
                )),
            };
            
            match collection_value {
                Value::Object(ref map) => evaluate_for_object(map, var_decl, &for_stmt.body, env),
                Value::Array(arr) => evaluate_for_array(arr, var_decl, &for_stmt.body, env),
                _ => Err(runtime_error(
                    "For loop must iterate over an object or array",
                    RuntimeErrorType::TypeError,
                    for_stmt.location.line,
                    for_stmt.location.column
                ))
            }
        } else {
            Err(runtime_error(
                "For loop requires a variable declaration",
                RuntimeErrorType::SyntaxError,
                for_stmt.location.line,
                for_stmt.location.column
            ))
        }
    } else {
        Err(runtime_error(
            "For loop requires an initialization",
            RuntimeErrorType::SyntaxError,
            for_stmt.location.line,
            for_stmt.location.column
        ))
    }
}

// Handle while statements
fn evaluate_while_statement(while_stmt: &WhileStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    loop {
        match evaluate_expression(&while_stmt.test, env)? {
            Value::Boolean(false) => break,
            Value::Boolean(true) => {
                let result = evaluate_block_content(&while_stmt.body, env)?;
                // If body returns a value, update the variable used in condition
                if let Some(value) = result {
                    // Extract variable name from test expression
                    if let Expr::Binary(ref binary) = *while_stmt.test {
                        if let Expr::Identifier(ref ident) = *binary.left {
                            env.assign(&ident.name, value).map_err(|e| runtime_error(
                                &format!("Failed to assign to variable '{}': {}", ident.name, e),
                                RuntimeErrorType::ReferenceError,
                                while_stmt.location.line,
                                while_stmt.location.column
                            ))?;
                        }
                    }
                }
            }
            _ => return Err(runtime_error(
                "While loop condition must evaluate to a boolean",
                RuntimeErrorType::TypeError,
                while_stmt.location.line,
                while_stmt.location.column
            )),
        }
    }
    Ok(None)
}

// Handle try-catch statements
fn evaluate_try_catch(try_catch: &TryCatchStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match evaluate_block_content(&try_catch.try_block, env) {
        Ok(value) => Ok(value),
        Err(error) => {
            if let Some(catch_block) = &try_catch.catch_block {
                let mut catch_env = Environment::new_with_parent(env.clone());
                // Convert error to string value that can be used in catch block
                let error_str = match error {
                    ZekkenError::RuntimeError { message, .. } => message,
                    ZekkenError::TypeError { message, .. } => message,
                    ZekkenError::ReferenceError { message, .. } => message,
                    _ => error.to_string()
                };
                
                // Make error available as 'e' variable in catch block
                catch_env.declare("e".to_string(), Value::String(error_str), false);
                
                // Evaluate catch block with error value
                evaluate_block_content(catch_block, &mut catch_env)
            } else {
                Err(error)
            }
        }
    }
}

// Handle code blocks
fn evaluate_block(block: &BlockStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    evaluate_block_content(&block.body, env)
}

// Handle code block content
fn evaluate_block_content(content: &Vec<Box<Content>>, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let mut result = None;
    for stmt in content {
        match **stmt {
            Content::Statement(ref stmt) => {
                result = evaluate_statement(stmt, env)?;
            }
            Content::Expression(ref expr) => {
                result = Some(evaluate_expression(expr, env)?);
            }
        }
    }
    Ok(result)
}

// Handle return values in functions
fn evaluate_return(ret: &ReturnStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match &ret.value {
        Some(content) => match &**content {
            Content::Expression(expr) => {
                let value = evaluate_expression(expr, env)?;
                Ok(Some(value))
            },
            Content::Statement(stmt) => evaluate_statement(stmt, env),
        },
        None => Ok(Some(Value::Void)),
    }
}

// Handle lambda expressions
fn evaluate_lambda(lambda: &LambdaDecl, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let function_value = FunctionValue {
        params: lambda.params.clone(),
        body: lambda.body.clone(),
    };

    env.declare(lambda.ident.clone(), Value::Function(function_value), lambda.constant);
    Ok(None)
}

// Handle use statements for importing libraries
fn evaluate_use(use_stmt: &UseStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    // Set up __IMPORT_METHODS__ for selective import
    if let Some(methods) = &use_stmt.methods {
        let method_values = methods.iter().map(|m| Value::String(m.clone())).collect();
        env.declare("__IMPORT_METHODS__".to_string(), Value::Array(method_values), true);
    }

    // Load the library/module
    match load_library(&use_stmt.module, env) {
        Ok(_) => Ok(None),
        Err(e) => Err(runtime_error(
            &format!("Failed to load library '{}': {}", use_stmt.module, e),
            RuntimeErrorType::ReferenceError,
            use_stmt.location.line,
            use_stmt.location.column,
        )),
    }
}

// Handle include statements for including external files
fn evaluate_include(include: &IncludeStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let current_dir = env.lookup("ZEKKEN_CURRENT_DIR")
        .and_then(|v| if let Value::String(s) = v { Some(s) } else { None })
        .unwrap_or_default();

    let file_path = if include.file_path.contains("../") || include.file_path.starts_with("./") {
        // This is a relative path, combine it with current directory
        let mut path = std::path::PathBuf::from(current_dir);
        // canonicalize() will resolve .. and . in paths
        path.push(&include.file_path);
        std::fs::canonicalize(path)
            .map_err(|e| runtime_error(
                &format!("Invalid include path: {}", e),
                RuntimeErrorType::ReferenceError,
                include.location.line,
                include.location.column
            ))?
            .to_string_lossy()
            .to_string()
    } else {
        // Treat as a path relative to current directory
        let mut path = std::path::PathBuf::from(current_dir);
        path.push(&include.file_path);
        path.to_string_lossy().to_string()
    };
    
    let file_contents = std::fs::read_to_string(&file_path)
        .map_err(|e| runtime_error(
            &format!("Failed to include file '{}': {}", file_path, e),
            RuntimeErrorType::ReferenceError,
            include.location.line,
            include.location.column
        ))?;

    let mut parser = Parser::new();
    let included_ast = parser.produce_ast(file_contents);

    let mut temp_env = Environment::new();
    evaluate_statement(&Stmt::Program(included_ast), &mut temp_env)?;

    match &include.methods {
        Some(methods) => {
            for method in methods {
                if let Some(value) = temp_env.lookup(method) {
                    env.declare(method.clone(), value, false);
                } else {
                    return Err(runtime_error(
                        &format!("Method '{}' not found in included file", method),
                        RuntimeErrorType::ReferenceError,
                        include.location.line,
                        include.location.column
                    ));
                }
            }
        }
        None => {
            for (name, value) in &temp_env.variables {
                env.declare(name.clone(), value.clone(), false);
            }
        }
    }
    Ok(None)
}

// Handle export statements
fn evaluate_export(exports: &ExportStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    for name in &exports.exports {
        if let Some(value) = env.lookup(name) {
            env.declare(name.clone(), value, false);
        } else {
            return Err(runtime_error(
                &format!("Cannot export undefined value '{}'", name),
                RuntimeErrorType::ReferenceError,
                exports.location.line,
                exports.location.column
            ));
        }
    }
    Ok(None)
}

// Handle for loop iterations over objects
fn evaluate_for_object(
    map: &HashMap<String, Value>,
    var_decl: &VarDecl,
    body: &Vec<Box<Content>>,
    env: &mut Environment
) -> Result<Option<Value>, ZekkenError> {
    let idents: Vec<String> = var_decl.ident.split(", ").map(|s| s.to_string()).collect();
    if idents.len() != 2 {
        return Err(runtime_error(
            "Object iteration requires two identifiers (key, value)",
            RuntimeErrorType::SyntaxError,
            var_decl.location.line,
            var_decl.location.column,
        ));
    }
    // Use the stored key order
    let keys = if let Some(Value::Array(keys)) = map.get("__keys__") {
        keys
    } else {
        return Err(runtime_error(
            "Object missing key order",
            RuntimeErrorType::TypeError,
            var_decl.location.line,
            var_decl.location.column,
        ));
    };

    for key_val in keys {
        if let Value::String(ref key) = key_val {
            if let Some(value) = map.get(key) {
                env.declare(idents[0].clone(), Value::String(key.clone()), false);
                env.declare(idents[1].clone(), value.clone(), false);
                evaluate_block_content(body, env)?;
            }
        }
    }
    Ok(None)
}

// Handle for loop iterations over arrays
fn evaluate_for_array(
    arr: Vec<Value>,
    var_decl: &VarDecl,
    body: &Vec<Box<Content>>,
    env: &mut Environment
) -> Result<Option<Value>, ZekkenError> {
    let idents: Vec<String> = var_decl.ident.split(", ").map(|s| s.to_string()).collect();
    if idents.len() != 1 {
        return Err(runtime_error(
            "Array iteration requires one identifier",
            RuntimeErrorType::SyntaxError,
            var_decl.location.line,
            var_decl.location.column
        ));
    }

    for value in arr.iter() {
        env.declare(idents[0].clone(), value.clone(), false);
        evaluate_block_content(body, env)?;
    }
    Ok(None)
}