use crate::ast::*;
use crate::environment::{Environment, Value, FunctionValue};
use crate::parser::Parser;
use super::expression::evaluate_expression;
use crate::errors::{ZekkenError, ErrorKind, push_error};
use crate::libraries::load_library;
use crate::lexer::DataType;
use std::collections::HashMap;
use std::path::Path;
// use std::process;
use super::lint::{lint_statement, lint_expression, lint_include, lint_use};

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

// Helper function to create a dummy value based on type
fn create_dummy_value(data_type: &DataType) -> Value {
    match data_type {
        DataType::String => Value::String(String::new()),
        DataType::Int => Value::Int(0),
        DataType::Float => Value::Float(0.0),
        DataType::Bool => Value::Boolean(false),
        DataType::Array => Value::Array(vec![]),
        DataType::Object => Value::Object(HashMap::new()),
        DataType::Fn => Value::Function(FunctionValue { 
            params: vec![], 
            body: vec![] 
        }),
        _ => Value::Void,
    }
}

// Helper function to process a statement for declarations
fn process_statement_scope(stmt: &Stmt, env: &mut Environment) {
    match stmt {
        Stmt::Lambda(lambda) => {
            // Register the lambda function in the environment during the first pass
            let function_value = FunctionValue {
                params: lambda.params.clone(),
                body: lambda.body.clone(),
            };
            env.declare(lambda.ident.clone(), Value::Function(function_value), lambda.constant);
        },
        Stmt::VarDecl(var_decl) => {
            // Skip type checking for object iteration patterns in for loops
            if var_decl.ident.contains(", ") {
                // This is likely a for-loop pattern, we'll validate types during evaluation
                env.declare(var_decl.ident.clone(), Value::Void, false);
                return;
            }
            
            if let Some(content) = &var_decl.value {
                match content {
                    Content::Expression(expr) => {
                        // Special handling for call expressions to native functions
                        if let Expr::Call(call) = &**expr {
                            if let Some(ident) = match &*call.callee {
                                Expr::Identifier(id) => Some(id),
                                _ => None,
                            } {
                                // Check if it's a native function by looking it up in the environment
                                if matches!(env.lookup(&ident.name), Some(Value::NativeFunction(_))) {
                                    // For @input specifically, we know it returns a string
                                    if ident.name == "@input" && var_decl.type_ != DataType::Any && var_decl.type_ != DataType::String {
                                        push_error(ZekkenError::type_error(
                                            &format!("Type mismatch in variable declaration '{}': expected {:?}, found string (from @input)", var_decl.ident, var_decl.type_),
                                            &format!("{:?}", var_decl.type_),
                                            "string",
                                            var_decl.location.line,
                                            var_decl.location.column
                                        ));
                                    }
                                } else {
                                    // Evaluate non-native function calls normally
                                    match evaluate_expression(expr, env) {
                                        Ok(val) => {
                                            if !check_value_type(&val, &var_decl.type_) {
                                                push_error(ZekkenError::type_error(
                                                    &format!("Type mismatch in variable declaration '{}': expected {:?}, found {}", var_decl.ident, var_decl.type_, value_type_name(&val)),
                                                    &format!("{:?}", var_decl.type_),
                                                    value_type_name(&val),
                                                    var_decl.location.line,
                                                    var_decl.location.column
                                                ));
                                            }
                                        },
                                        Err(_) => {}
                                    }
                                }
                            } else {
                                // Evaluate non-identifier callees normally
                                match evaluate_expression(expr, env) {
                                    Ok(val) => {
                                        if !check_value_type(&val, &var_decl.type_) {
                                            push_error(ZekkenError::type_error(
                                                &format!("Type mismatch in variable declaration '{}': expected {:?}, found {}", var_decl.ident, var_decl.type_, value_type_name(&val)),
                                                &format!("{:?}", var_decl.type_),
                                                value_type_name(&val),
                                                var_decl.location.line,
                                                var_decl.location.column
                                            ));
                                        }
                                    },
                                    Err(_) => {}
                                }
                            }
                        } else {
                            // Evaluate non-call expressions normally
                            match evaluate_expression(expr, env) {
                                Ok(val) => {
                                    if !check_value_type(&val, &var_decl.type_) {
                                        push_error(ZekkenError::type_error(
                                            &format!("Type mismatch in variable declaration '{}': expected {:?}, found {}", var_decl.ident, var_decl.type_, value_type_name(&val)),
                                            &format!("{:?}", var_decl.type_),
                                            value_type_name(&val),
                                            var_decl.location.line,
                                            var_decl.location.column
                                        ));
                                    }
                                },
                                Err(_) => {}
                            }
                        }
                    },
                    Content::Statement(_) => {}
                }
            }
            // Register variable with dummy value based on its type
            let dummy_val = create_dummy_value(&var_decl.type_);
            env.declare(var_decl.ident.clone(), dummy_val, var_decl.constant);
        },
        Stmt::FuncDecl(func_decl) => {
            // First, register the function itself in the environment
            let function_value = FunctionValue {
                params: func_decl.params.clone(),
                body: func_decl.body.clone(),
            };
            env.declare(func_decl.ident.clone(), Value::Function(function_value), false);
            
            // Process function parameters in the current environment
            for param in &func_decl.params {
                let dummy_val = create_dummy_value(&param.type_);
                env.declare(param.ident.clone(), dummy_val, false);
            }
            
            // Process the function body
            for content in &func_decl.body {
                if let Content::Statement(stmt) = &**content {
                    process_statement_scope(stmt, env);
                }
            }
        },
        Stmt::BlockStmt(block) => {
            // Process block contents in the current environment
            for content in &block.body {
                if let Content::Statement(stmt) = &**content {
                    process_statement_scope(stmt, env);
                }
            }
        },
        Stmt::ForStmt(for_stmt) => {
            // Process initializer if it exists
            if let Some(init) = &for_stmt.init {
                process_statement_scope(init, env);
            }
            
            // Process the loop body
            for content in &for_stmt.body {
                if let Content::Statement(stmt) = &**content {
                    process_statement_scope(stmt, env);
                }
            }
        },
        _ => {}
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
        Value::Function(_) => "",
        Value::NativeFunction(_) => "",
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
    let mut errors = Vec::new();

    // Create environment for processing
    let mut temp_env = env.clone();

    // First pass: Process imports and declarations
    for import in &program.imports {
        if let Content::Statement(stmt) = &*import {
            match **stmt {
                Stmt::Include(ref include) => {
                    // First check if the file exists
                    if let Err(e) = lint_include(include) {
                        errors.push(e);
                        continue;
                    }
                    // If file exists, evaluate it to set up the environment
                    if let Err(e) = evaluate_include(include, &mut temp_env) {
                        errors.push(e);
                    }
                },
                Stmt::Use(ref use_stmt) => {
                    // First check if the library is valid
                    if let Err(e) = lint_use(use_stmt) {
                        errors.push(e);
                        continue;
                    }
                    // If library is valid, load it to set up the environment
                    if let Err(e) = evaluate_use(use_stmt, &mut temp_env) {
                        errors.push(e);
                    }
                },
                _ => errors.push(ZekkenError::syntax(
                    "Invalid import statement",
                    0,
                    0,
                    None,
                    None,
                ))
            }
        }
    }

    // If there were import errors, report them and stop (except internal errors)
    if !errors.is_empty() {
        for error in errors {
            if error.kind == ErrorKind::Internal {
                continue; // Skip internal errors
            }
            push_error(error.clone());
        }
        // Just return an error to stop execution, but don't log it
        return Err(ZekkenError::internal("Import errors found"));
    }

    // Process top-level declarations using same environment
    let mut lint_errors = Vec::new();

    // Process all top-level statements in the same environment
    for content in &program.content {
        if let Content::Statement(stmt) = &**content {
            process_statement_scope(stmt, &mut temp_env);
        }
    }
    
    // Process all top-level statements
    for content in &program.content {
        if let Content::Statement(stmt) = &**content {
            process_statement_scope(stmt, &mut temp_env);
        }
    }
    
    // Second pass: Now lint everything with the complete environment
    for content in &program.content {
        match &**content {
            Content::Statement(stmt) => {
                if let Err(e) = lint_statement(stmt, &temp_env) {
                    lint_errors.push(e);
                }
            },
            Content::Expression(expr) => {
                if let Err(e) = lint_expression(expr, &temp_env) {
                    lint_errors.push(e);
                }
            }
        }
    }

    // If any errors were found during linting, report them (except internal errors)
    if !lint_errors.is_empty() {
        for error in lint_errors {
            if error.kind == ErrorKind::Internal {
                continue; // Skip internal errors
            }
            push_error(error.clone());
        }
        // Just return an error to stop execution, but don't log it
        return Err(ZekkenError::internal("Linting errors found"));
    }
    
    // Update the real environment with all the declarations we processed
    *env = temp_env;

    // If no errors found during linting, proceed with execution of the main content
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
    let mut had_error = false;
    for content in &program.content {
        match &**content {
            Content::Statement(stmt) => {
                match evaluate_statement(stmt, env) {
                    Ok(val) => last_value = val,
                    Err(e) => {
                        push_error(e.clone());
                        had_error = true;
                        // continue to next statement instead of returning
                    }
                }
            }
            Content::Expression(expr) => {
                match evaluate_expression(expr, env) {
                    Ok(val) => last_value = Some(val),
                    Err(e) => {
                        push_error(e.clone());
                        had_error = true;
                        // continue to next expression instead of returning
                    }
                }
            }
        }
    }

    if had_error {
        // Return a dummy error so main.rs knows not to print a result
        return Err(ZekkenError::internal("Multiple runtime errors occurred"));
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
                let mut err_obj = std::collections::HashMap::new();
                err_obj.insert("message".to_string(), Value::String(error.message.clone()));
                err_obj.insert("kind".to_string(), Value::String(format!("{:?}", error.kind)));
                err_obj.insert("line".to_string(), Value::Int(error.context.line as i64));
                err_obj.insert("column".to_string(), Value::Int(error.context.column as i64));
                // Add the pretty error string for display
                err_obj.insert("__zekken_error__".to_string(), Value::String(error.to_string()));
                catch_env.declare("e".to_string(), Value::Object(err_obj), false);
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
    match load_library(&use_stmt.module, env) {
        Ok(_) => {
            // If specific methods are requested, extract them from the library object
            if let Some(methods) = &use_stmt.methods {
                // Get the library object
                if let Some(Value::Object(lib_obj)) = env.lookup(&use_stmt.module) {
                    // Import each requested method directly into the target environment
                    for method in methods {
                        if let Some(value) = lib_obj.get(method) {
                            env.declare(method.clone(), value.clone(), false);
                        } else {
                            return Err(ZekkenError::runtime(
                                &format!("Method '{}' not found in library '{}'", method, use_stmt.module),
                                use_stmt.location.line,
                                use_stmt.location.column,
                                None,
                            ));
                        }
                    }
                }
            }
            Ok(None)
        },
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
    // Get the directory of the current file being processed
    let current_file = std::env::var("ZEKKEN_CURRENT_FILE").unwrap_or_else(|_| "<unknown>".to_string());
    let current_dir = if current_file == "<unknown>" {
        env.lookup("ZEKKEN_CURRENT_DIR")
            .and_then(|v| if let Value::String(s) = v { Some(s) } else { None })
            .unwrap_or_default()
    } else {
        Path::new(&current_file)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    };

    // Always resolve paths relative to current file's directory
    let mut path = std::path::PathBuf::from(&current_dir);
    path.push(&include.file_path);

    // Try to canonicalize but don't require it to succeed
    let file_path = path.to_string_lossy().to_string();
    
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
    // Extract key and value identifiers
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
    
    // Get the keys array from the object
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

    // Create a new environment for each iteration
    let mut iter_env = Environment::new_with_parent(env.clone());
    
    // Declare the variables with their proper types before the loop
    iter_env.declare(idents[0].clone(), Value::String(String::new()), false); // key is always string
    match var_decl.type_ {
        DataType::Any => {}, // For Any type, accept any value type
        DataType::String => iter_env.declare(idents[1].clone(), Value::String(String::new()), false),
        DataType::Int => iter_env.declare(idents[1].clone(), Value::Int(0), false),
        DataType::Float => iter_env.declare(idents[1].clone(), Value::Float(0.0), false),
        DataType::Bool => iter_env.declare(idents[1].clone(), Value::Boolean(false), false),
        DataType::Object => iter_env.declare(idents[1].clone(), Value::Object(HashMap::new()), false),
        DataType::Array => iter_env.declare(idents[1].clone(), Value::Array(Vec::new()), false),
        DataType::Fn => iter_env.declare(idents[1].clone(), Value::Function(FunctionValue { params: vec![], body: vec![] }), false),
    }

    for key_val in keys {
        if let Value::String(ref key) = key_val {
            if let Some(value) = map.get(key) {
                // Check if the value matches the declared type
                if var_decl.type_ != DataType::Any && !check_value_type(value, &var_decl.type_) {
                    return Err(ZekkenError::type_error(
                        &format!("Type mismatch in for loop value: expected {:?}, found {}", var_decl.type_, value_type_name(value)),
                        &format!("{:?}", var_decl.type_),
                        value_type_name(value),
                        var_decl.location.line,
                        var_decl.location.column
                    ));
                }
                
                iter_env.declare(idents[0].clone(), Value::String(key.clone()), false);
                iter_env.declare(idents[1].clone(), value.clone(), false);
                evaluate_block_content(body, &mut iter_env)?;
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