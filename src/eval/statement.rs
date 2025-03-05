use crate::ast::*;
use crate::environment::{Environment, Value, FunctionValue};
use crate::parser::Parser;
use super::expression::evaluate_expression;
use crate::errors::ZekkenError;

fn runtime_error(error: &str, line: usize) -> ZekkenError {
    ZekkenError::RuntimeError {
        message: error.to_string(),
        filename: None,
        line: Some(line),
        column: None,
        line_content: None,
        pointer: None,
        expected: None,
        found: None,
    }
}

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

fn evaluate_program(program: &Program, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    for import in &program.imports {
        if let Content::Statement(stmt) = &*import {
            match &**stmt {
                Stmt::Include(include) => evaluate_include(include, env)?,
                Stmt::Use(use_stmt) => evaluate_use(use_stmt, env)?,
                _ => return Ok(None)
            };
        }
    }

    // Process all content
    for content in &program.content {
        match &**content {
            Content::Statement(stmt) => {
                evaluate_statement(stmt, env)?;
            },
            Content::Expression(expr) => {
                evaluate_expression(expr, env)?;
            }
        }
    }

    Ok(None)
}

fn evaluate_var_declaration(decl: &VarDecl, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let value = match &decl.value {
        Some(content) => match content {
            Content::Expression(expr) => Some(evaluate_expression(expr, env)?),
            Content::Statement(stmt) => evaluate_statement(stmt, env)?,
        },
        None => None,
    };

    if let Some(val) = value.clone() {
        env.declare(decl.ident.clone(), val, decl.constant);
    }

    Ok(value)
}

fn evaluate_function_declaration(func: &FuncDecl, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let function_value = FunctionValue {
        params: func.params.clone(),
        body: func.body.clone(),
    };

    env.declare(func.ident.clone(), Value::Function(function_value), false);
    Ok(None)
}

fn evaluate_object_declaration(obj: &ObjectDecl, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let mut object_map = std::collections::HashMap::new();

    for property in &obj.properties {
        let value = evaluate_expression(&property.value, env)?;
        object_map.insert(property.key.clone(), value);
    }

    env.declare(obj.ident.clone(), Value::Object(object_map), false);
    Ok(None)
}

fn evaluate_if_statement(if_stmt: &IfStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let test_result = evaluate_expression(&if_stmt.test, env)?;
    
    match test_result {
        Value::Boolean(true) => {
            let mut result = None;
            for stmt in &if_stmt.body {
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
        Value::Boolean(false) => {
            if let Some(alt) = &if_stmt.alt {
                let mut result = None;
                for stmt in alt {
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
            } else {
                Ok(None)
            }
        }
        _ => Err(runtime_error("If statement condition must evaluate to a boolean", if_stmt.location.line)),
    }
}

fn evaluate_for_statement(for_stmt: &ForStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    if let Some(ref init) = for_stmt.init {
        if let Stmt::VarDecl(var_decl) = &**init {
            let collection_value = match &var_decl.value {
                Some(content) => match content {
                    Content::Expression(expr) => evaluate_expression(expr, env)?,
                    _ => panic!("Expected expression in for loop initialization"),
                },
                None => panic!("Expected expression in for loop initialization"),
            };
            
            match collection_value {
                Value::Object(map) => {
                    let idents: Vec<String> = var_decl.ident.split(", ").map(|s| s.to_string()).collect();
                    
                    for (key, value) in map {
                        // Declare key and value in the environment
                        env.declare(idents[0].clone(), Value::String(key.clone()), false); // Key
                        env.declare(idents[1].clone(), value.clone(), false); // Value
                        
                        // Execute the body of the for loop
                        for content in &for_stmt.body {
                            match **content {
                                Content::Statement(ref stmt) => {
                                    evaluate_statement(stmt, env)?;
                                }
                                Content::Expression(ref expr) => {
                                    evaluate_expression(expr, env)?;
                                }
                            }
                        }
                    }
                }
                Value::Array(arr) => {
                    let idents: Vec<String> = var_decl.ident.split(", ").map(|s| s.to_string()).collect();
                    
                    for value in arr {
                        // Declare value in the environment
                        env.declare(idents[0].clone(), value.clone(), false); // Value
                        
                        // Execute the body of the for loop
                        for content in &for_stmt.body {
                            match **content {
                                Content::Statement(ref stmt) => {
                                    evaluate_statement(stmt, env)?;
                                }
                                Content::Expression(ref expr) => {
                                    evaluate_expression(expr, env)?;
                                }
                            }
                        }
                    }
                }
                _ => {
                    return Err(runtime_error("For loop must iterate over an object or array", for_stmt.location.line));
                }
            }
        } else {
            panic!("Expected variable declaration in for loop initialization");
        }
    } else {
        panic!("For loop requires an initialization");
    }
    
    Ok(None)
}

fn evaluate_while_statement(while_stmt: &WhileStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    loop {
        match evaluate_expression(&while_stmt.test, env)? {
            Value::Boolean(false) => break,
            Value::Boolean(true) => {
                let mut result = None;
                for stmt in &while_stmt.body {
                    match **stmt {
                        Content::Statement(ref stmt) => {
                            result = evaluate_statement(stmt, env)?;
                        }
                        Content::Expression(ref expr) => {
                            result = Some(evaluate_expression(expr, env)?);
                        }
                    }
                }
                if result.is_some() {
                    return Ok(result);
                }
            }
            _ => return Err(runtime_error("While loop condition must evaluate to a boolean", while_stmt.location.line)),
        }
    }
    Ok(None)
}

fn evaluate_try_catch(try_catch: &TryCatchStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let try_result = (|| -> Result<Option<Value>, ZekkenError> {
        let mut result = None;
        for stmt in &try_catch.try_block {
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
    })();

    match try_result {
        Ok(value) => Ok(value),
        Err(error) => {
            if let Some(catch_block) = &try_catch.catch_block {
                let mut result = None;
                for stmt in catch_block {
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
            } else {
                Err(error)
            }
        }
    }
}

fn evaluate_block(block: &BlockStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let mut result = None;
    for stmt in &block.body {
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

fn evaluate_return(ret: &ReturnStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match &ret.value {
        Some(content) => match **content {
            Content::Expression(ref expr) => Ok(Some(evaluate_expression(expr, env)?)),
            Content::Statement(ref stmt) => evaluate_statement(stmt, env),
        },
        None => Ok(Some(Value::Void)),
    }
}

fn evaluate_lambda(lambda: &LambdaDecl, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let function_value = FunctionValue {
        params: lambda.params.clone(),
        body: lambda.body.clone(),
    };

    env.declare(lambda.ident.clone(), Value::Function(function_value), lambda.constant);
    Ok(None)
}

fn evaluate_use(use_stmt: &UseStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let module_path = format!("{}.zk", use_stmt.module);
    let module_contents = std::fs::read_to_string(&module_path)
        .map_err(|e| ZekkenError::RuntimeError {
            message: format!("Failed to load module '{}': {}", module_path, e),
            filename: None,
            line: None,
            column: None,
            line_content: None,
            pointer: None,
            expected: None,
            found: None,
        })?;

    let mut parser = Parser::new();
    let module_ast = parser.produce_ast(module_contents);
    
    let mut module_env = Environment::new();
    
    evaluate_statement(&Stmt::Program(module_ast), &mut module_env)?;

    match &use_stmt.methods {
        Some(methods) => {
            for method in methods {
                if let Some(value) = module_env.lookup(method) {
                    env.declare(method.clone(), value, false);
                } else {
                    return Err(ZekkenError::RuntimeError {
                        message: format!("Method '{}' not found in module '{}'", method, use_stmt.module),
                        filename: None,
                        line: None,
                        column: None,
                        line_content: None,
                        pointer: None,
                        expected: None,
                        found: None,
                    });
                }
            }
        }
        None => {
            for (name, value) in &module_env.variables {
                env.declare(name.clone(), value.clone(), false);
            }
        }
    }

    Ok(None)
}

fn evaluate_include(include: &IncludeStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let file_contents = std::fs::read_to_string(&include.file_path)
        .map_err(|e| ZekkenError::RuntimeError {
            message: format!("Failed to include file '{}': {}", include.file_path, e),
            filename: None,
            line: None,
            column: None,
            line_content: None,
            pointer: None,
            expected: None,
            found: None,
        })?;

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
                    return Err(ZekkenError::RuntimeError {
                        message: format!("Method '{}' not found in included file", method),
                        filename: None,
                        line: None,
                        column: None,
                        line_content: None,
                        pointer: None,
                        expected: None,
                        found: None,
                    });
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

fn evaluate_export(exports: &ExportStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    for name in &exports.exports {
        if let Some(value) = env.lookup(name) {
            env.declare(name.clone(), value, false);
        } else {
            return Err(ZekkenError::RuntimeError {
                message: format!("Cannot export undefined value '{}'", name),
                filename: None,
                line: None,
                column: None,
                line_content: None,
                pointer: None,
                expected: None,
                found: None,
            });
        }
    }
    
    Ok(None)
}