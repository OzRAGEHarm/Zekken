use crate::ast::*;
use crate::environment::{Environment, Value, FunctionValue};
use crate::parser::Parser;
use super::expression::evaluate_expression;
use crate::errors::ZekkenError;
use crate::libraries::load_library;
use crate::lexer::DataType;
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

// Helper to get a string name for Value type
fn value_type_name(val: &Value) -> &'static str {
    match val {
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Boolean(_) => "bool",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Function(_) => "function",
        Value::NativeFunction(_) => "native function",
        Value::Void => "void",
        _ => "unknown",
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
    // Process imports first
    for import in &program.imports {
        if let Content::Statement(stmt) = &*import {
            match **stmt {
                Stmt::Include(ref include) => evaluate_include(include, env)?,
                Stmt::Use(ref use_stmt) => evaluate_use(use_stmt, env)?,
                _ => return Err(ZekkenError::syntax(
                    "Invalid import statement",
                    0,
                     0,
                    None,
                    None,
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
                    return Err(ZekkenError::type_error(
                        &format!("Type mismatch in variable declaration '{}'", decl.ident),
                        &format!("{:?}", decl.type_),
                        value_type_name(&val),
                        decl.location.line,
                        decl.location.column
                    ));
                }
                val
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
    Ok(None)
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
            .map_err(|e| ZekkenError::type_error(
                &format!("Failed to evaluate property '{}': {}", property.key, e),
                "object",
                "property evaluation failed",
                obj.location.line,
                obj.location.column
            ))?;
        keys.push(property.key.clone());
        object_map.insert(property.key.clone(), value);
    }
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
        _ => Err(ZekkenError::type_error(
            "If statement condition must evaluate to a boolean",
            "bool",
            value_type_name(&test_result),
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
                    _ => return Err(ZekkenError::runtime(
                        "Expected expression in for loop initialization",
                        for_stmt.location.line,
                        for_stmt.location.column,
                        Some("for |x| in array { ... }"),
                    )),
                },
                None => return Err(ZekkenError::runtime(
                    "For loop initialization requires a value",
                    for_stmt.location.line,
                    for_stmt.location.column,
                    None,
                )),
            };
            match collection_value {
                Value::Object(ref map) => evaluate_for_object(map, var_decl, &for_stmt.body, env),
                Value::Array(arr) => evaluate_for_array(arr, var_decl, &for_stmt.body, env),
                _ => Err(ZekkenError::type_error(
                    "For loop must iterate over an object or array",
                    "object or array",
                    value_type_name(&collection_value),
                    for_stmt.location.line,
                    for_stmt.location.column
                ))
            }
        } else {
            Err(ZekkenError::runtime(
                "For loop requires a variable declaration",
                for_stmt.location.line,
                for_stmt.location.column,
                None
            ))
        }
    } else {
        Err(ZekkenError::runtime(
            "For loop requires an initialization",
            for_stmt.location.line,
            for_stmt.location.column,
            None
        ))
    }
}

// Handle while statements
fn evaluate_while_statement(while_stmt: &WhileStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let mut result = None;
    loop {
        let test_result = evaluate_expression(&while_stmt.test, env)?;
        match test_result {
            Value::Boolean(true) => {
                for content in &while_stmt.body {
                    match &**content {
                        Content::Statement(stmt) => {
                            result = evaluate_statement(stmt, env)?;
                        }
                        Content::Expression(expr) => {
                            result = Some(evaluate_expression(expr, env)?);
                        }
                    }
                }
            }
            Value::Boolean(false) => break,
            _ => {
                return Err(ZekkenError::type_error(
                    "While loop condition must evaluate to a boolean",
                    "bool",
                    value_type_name(&test_result),
                    while_stmt.location.line,
                    while_stmt.location.column
                ))
            }
        }
    }
    Ok(result)
}

// Handle try-catch statements
fn evaluate_try_catch(try_catch: &TryCatchStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match evaluate_block_content(&try_catch.try_block, env) {
        Ok(value) => Ok(value),
        Err(error) => {
            if let Some(catch_block) = &try_catch.catch_block {
                let mut catch_env = Environment::new_with_parent(env.clone());
                let error_str = error.to_string();
                catch_env.declare("e".to_string(), Value::String(error_str), false);
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
    if let Some(methods) = &use_stmt.methods {
        let method_values = methods.iter().map(|m| Value::String(m.clone())).collect();
        env.declare("__IMPORT_METHODS__".to_string(), Value::Array(method_values), true);
    }

    match load_library(&use_stmt.module, env) {
        Ok(_) => Ok(None),
        Err(e) => Err(ZekkenError::runtime(
            &format!("Failed to load library '{}': {}", use_stmt.module, e),
            use_stmt.location.line,
            use_stmt.location.column,
            None,
        )),
    }
}

// Handle include statements for including external files
fn evaluate_include(include: &IncludeStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let current_dir = env.lookup("ZEKKEN_CURRENT_DIR")
        .and_then(|v| if let Value::String(s) = v { Some(s) } else { None })
        .unwrap_or_default();

    let file_path = if include.file_path.contains("../") || include.file_path.starts_with("./") {
        let mut path = std::path::PathBuf::from(current_dir);
        path.push(&include.file_path);
        std::fs::canonicalize(path)
            .map_err(|e| ZekkenError::runtime(
                &format!("Invalid include path: {}", e),
                include.location.line,
                include.location.column,
                None,
            ))?
            .to_string_lossy()
            .to_string()
    } else {
        let mut path = std::path::PathBuf::from(current_dir);
        path.push(&include.file_path);
        path.to_string_lossy().to_string()
    };
    
    let file_contents = std::fs::read_to_string(&file_path)
        .map_err(|e| ZekkenError::runtime(
            &format!("Failed to include file '{}': {}", file_path, e),
            include.location.line,
            include.location.column,
            None,
        ))?;

    // Save previous file context
    let prev_file = std::env::var("ZEKKEN_CURRENT_FILE").unwrap_or_else(|_| "<unknown>".to_string());
    // Set current file context to included file
    std::env::set_var("ZEKKEN_CURRENT_FILE", &file_path);

    let mut parser = Parser::new();
    let included_ast = parser.produce_ast(file_contents);

    // Create a new child environment with current env as parent
    let mut child_env = Environment::new_with_parent(env.clone());

    // Evaluate included AST in child environment
    let result = evaluate_statement(&Stmt::Program(included_ast), &mut child_env);

    // Restore previous file context
    std::env::set_var("ZEKKEN_CURRENT_FILE", prev_file);

    result?;

    // Copy exported methods or all variables from child_env to current env
    match &include.methods {
        Some(methods) => {
            for method in methods {
                if let Some(value) = child_env.lookup(method) {
                    env.declare(method.clone(), value, false);
                } else {
                    return Err(ZekkenError::runtime(
                        &format!("Method '{}' not found in included file", method),
                        include.location.line,
                        include.location.column,
                        None,
                    ));
                }
            }
        }
        None => {
            for (name, value) in &child_env.variables {
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
            return Err(ZekkenError::runtime(
                &format!("Cannot export undefined value '{}'", name),
                exports.location.line,
                exports.location.column,
                None,
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
        return Err(ZekkenError::syntax(
            "Object iteration requires two identifiers (key, value)",
            var_decl.location.line,
            var_decl.location.column,
            None,
            None,
        ));
    }
    let keys = if let Some(Value::Array(keys)) = map.get("__keys__") {
        keys
    } else {
        return Err(ZekkenError::type_error(
            "Object missing key order",
            "array",
            "missing",
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
        return Err(ZekkenError::syntax(
            "Array iteration requires one identifier",
            var_decl.location.line,
            var_decl.location.column,
            None,
            None,
        ));
    }

    for value in arr.iter() {
        env.declare(idents[0].clone(), value.clone(), false);
        evaluate_block_content(body, env)?;
    }
    Ok(None)
}