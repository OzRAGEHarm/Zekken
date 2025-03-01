use crate::ast::*;
use crate::environment::{Environment, Value, FunctionValue};
use crate::parser::Parser;
use super::expression::evaluate_expression;

pub fn evaluate_statement(stmt: &Stmt, env: &mut Environment) -> Result<Option<Value>, String> {
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

fn evaluate_program(program: &Program, env: &mut Environment) -> Result<Option<Value>, String> {
  let mut last_result = None;
  for content in &program.body {
      match content {
          Content::Statement(stmt) => {
              last_result = evaluate_statement(stmt, env)?;
          }
          Content::Expression(expr) => {
              last_result = Some(evaluate_expression(expr, env)?);
          }
      }
  }
  Ok(last_result)
}

fn evaluate_var_declaration(decl: &VarDecl, env: &mut Environment) -> Result<Option<Value>, String> {
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

fn evaluate_function_declaration(func: &FuncDecl, env: &mut Environment) -> Result<Option<Value>, String> {
    let function_value = FunctionValue {
        params: func.params.clone(),
        body: func.body.clone(),
        //closure: env.clone(),
    };

    env.declare(func.ident.clone(), Value::Function(function_value), false);
    Ok(None)
}

fn evaluate_object_declaration(obj: &ObjectDecl, env: &mut Environment) -> Result<Option<Value>, String> {
    let mut object_map = std::collections::HashMap::new();

    for property in &obj.properties {
        let value = evaluate_expression(&property.value, env)?;
        object_map.insert(property.key.clone(), value);
    }

    env.declare(obj.ident.clone(), Value::Object(object_map), false);
    Ok(None)
}

fn evaluate_if_statement(if_stmt: &IfStmt, env: &mut Environment) -> Result<Option<Value>, String> {
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
        _ => Err("If statement condition must evaluate to a boolean".to_string())
    }
}

fn evaluate_for_statement(for_stmt: &ForStmt, env: &mut Environment) -> Result<Option<Value>, String> {
    if let Some(ref init) = for_stmt.init {
        evaluate_statement(init, env)?;
    }

    loop {
        if let Some(ref test) = for_stmt.test {
            match evaluate_expression(test, env)? {
                Value::Boolean(false) => break,
                Value::Boolean(true) => (),
                _ => return Err("For loop test must evaluate to a boolean".to_string())
            }
        }

        let mut result = None;
        for stmt in &for_stmt.body {
            match **stmt {
                Content::Statement(ref stmt) => {
                    result = evaluate_statement(stmt, env)?;
                }
                Content::Expression(ref expr) => {
                    result = Some(evaluate_expression(expr, env)?);
                }
            }
        }

        if let Some(ref update) = for_stmt.update {
            evaluate_expression(update, env)?;
        }

        if result.is_some() {
            return Ok(result);
        }
    }

    Ok(None)
}

fn evaluate_while_statement(while_stmt: &WhileStmt, env: &mut Environment) -> Result<Option<Value>, String> {
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
            _ => return Err("While loop condition must evaluate to a boolean".to_string())
        }
    }
    Ok(None)
}

fn evaluate_try_catch(try_catch: &TryCatchStmt, env: &mut Environment) -> Result<Option<Value>, String> {
    let try_result = (|| -> Result<Option<Value>, String> {
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

fn evaluate_block(block: &BlockStmt, env: &mut Environment) -> Result<Option<Value>, String> {
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

fn evaluate_return(ret: &ReturnStmt, env: &mut Environment) -> Result<Option<Value>, String> {
    match &ret.value {
        Some(content) => match **content {
            Content::Expression(ref expr) => Ok(Some(evaluate_expression(expr, env)?)),
            Content::Statement(ref stmt) => evaluate_statement(stmt, env),
        },
        None => Ok(Some(Value::Void)),
    }
}

fn evaluate_lambda(lambda: &LambdaDecl, env: &mut Environment) -> Result<Option<Value>, String> {
    let function_value = FunctionValue {
        params: lambda.params.clone(),
        body: lambda.body.clone(),
        //closure: env.clone(),
    };

    env.declare(lambda.ident.clone(), Value::Function(function_value), lambda.constant);
    Ok(None)
}

fn evaluate_use(use_stmt: &UseStmt, env: &mut Environment) -> Result<Option<Value>, String> {
  // Load and parse module file
  let module_path = format!("{}.zk", use_stmt.module);
  let module_contents = std::fs::read_to_string(&module_path)
      .map_err(|e| format!("Failed to load module '{}': {}", module_path, e))?;

  let mut parser = Parser::new();
  let module_ast = parser.produce_ast(module_contents);
  
  // Create new environment for module
  let mut module_env = Environment::new();
  
  // Evaluate module code
  evaluate_statement(&Stmt::Program(module_ast), &mut module_env)?;

  // Import specified items or everything
  match &use_stmt.methods {
      Some(methods) => {
          // Import only specified methods
          for method in methods {
              if let Some(value) = module_env.lookup(method) {
                  env.declare(method.clone(), value, false);
              } else {
                  return Err(format!("Method '{}' not found in module '{}'", method, use_stmt.module));
              }
          }
      }
      None => {
          // Import all values by checking through lookups
          for (name, value) in &module_env.variables {
              env.declare(name.clone(), value.clone(), false);
          }
      }
  }

  Ok(None)
}

fn evaluate_include(include: &IncludeStmt, env: &mut Environment) -> Result<Option<Value>, String> {
  let file_contents = std::fs::read_to_string(&include.file_path)
      .map_err(|e| format!("Failed to include file '{}': {}", include.file_path, e))?;

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
                  return Err(format!("Method '{}' not found in included file", method));
              }
          }
      }
      None => {
          // Import all values
          for (name, value) in &temp_env.variables {
              env.declare(name.clone(), value.clone(), false);
          }
      }
  }

  Ok(None)
}

fn evaluate_export(exports: &Vec<String>, env: &mut Environment) -> Result<Option<Value>, String> {
  for name in exports {
      if let Some(value) = env.lookup(name) {
          // Just re-declare as variable which makes it accessible
          env.declare(name.clone(), value, false);
      } else {
          return Err(format!("Cannot export undefined value '{}'", name));
      }
  }
  
  Ok(None)
}