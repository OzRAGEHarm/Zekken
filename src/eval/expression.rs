use crate::ast::*;
use crate::environment::{Environment, Value};
use std::collections::HashMap;
use crate::eval::statement::evaluate_statement;
use crate::errors::{ZekkenError, runtime_error};

pub fn evaluate_expression(expr: &Expr, env: &mut Environment) -> Result<Value, ZekkenError> {
    match expr {
        Expr::IntLit(int) => Ok(Value::Int(int.value)),
        Expr::FloatLit(float) => Ok(Value::Float(float.value)),
        Expr::StringLit(string) => Ok(Value::String(string.value.clone())),
        Expr::BoolLit(bool) => Ok(Value::Boolean(bool.value)),
        Expr::ArrayLit(array) => {
            let mut values = Vec::new();
            for element in &array.elements {
                values.push(evaluate_expression(element, env)?);
            }
            Ok(Value::Array(values))
        },
        Expr::ObjectLit(object) => {
            let mut map = HashMap::new();
            for prop in &object.properties {
                let value = evaluate_expression(&prop.value, env)?;
                map.insert(prop.key.clone(), value);
            }
            Ok(Value::Object(map))
        },
        Expr::Identifier(ident) => {
            match env.lookup(&ident.name) {
                Some(value) => Ok(value),
                None => Err(runtime_error(
                    &format!("Variable '{}' not found", ident.name),
                    ident.location.line,
                    ident.location.column,
                ))
            }
        },
        Expr::Binary(binary) => evaluate_binary_expression(binary, env),
        Expr::Call(call) => evaluate_call_expression(call, env),
        Expr::Member(member) => evaluate_member_expression(member, env),
        Expr::Assign(assign) => evaluate_assignment(assign, env),
        Expr::Property(_) => Err(ZekkenError::InternalError("Property expression not supported in this context".to_string()))
    }
}

fn evaluate_binary_expression(expr: &BinaryExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    let left = evaluate_expression(&expr.left, env)?;
    let right = evaluate_expression(&expr.right, env)?;

    match expr.operator.as_str() {
        "+" => add_values(left, right).map_err(|e| runtime_error(&e, expr.location.line, expr.location.column)),
        "-" => subtract_values(left, right).map_err(|e| runtime_error(&e, expr.location.line, expr.location.column)),
        "*" => multiply_values(left, right).map_err(|e| runtime_error(&e, expr.location.line, expr.location.column)),
        "/" => divide_values(left, right).map_err(|e| runtime_error(&e, expr.location.line, expr.location.column)),
        "%" => modulo_values(left, right).map_err(|e| runtime_error(&e, expr.location.line, expr.location.column)),
        "==" => Ok(Value::Boolean(compare_values(&left, &right))),
        "!=" => Ok(Value::Boolean(!compare_values(&left, &right))),
        "<" => compare_less_than(left, right).map_err(|e| runtime_error(&e, expr.location.line, expr.location.column)),
        ">" => compare_greater_than(left, right).map_err(|e| runtime_error(&e, expr.location.line, expr.location.column)),
        "<=" => compare_less_equal(left, right).map_err(|e| runtime_error(&e, expr.location.line, expr.location.column)),
        ">=" => compare_greater_equal(left, right).map_err(|e| runtime_error(&e, expr.location.line, expr.location.column)),
        "&&" => logical_and(left, right).map_err(|e| runtime_error(&e, expr.location.line, expr.location.column)),
        "||" => logical_or(left, right).map_err(|e| runtime_error(&e, expr.location.line, expr.location.column)),
        operator => Err(runtime_error(&format!("Unknown operator: {}", operator), expr.location.line, expr.location.column))
    }
}

fn evaluate_call_expression(call: &CallExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    let callee = evaluate_expression(&call.callee, env)?;

    match callee {
        Value::NativeFunction(native_func) => {
            let mut args = Vec::new();
            for arg in &call.args {
                let evaluated = evaluate_expression(arg, env)?;
                args.push(evaluated);
            }
            native_func(args).map_err(|e| runtime_error(&e, call.location.line, call.location.column))
        },
        Value::Function(func) => {
            let mut args = Vec::new();
            for arg in &call.args {
                args.push(evaluate_expression(arg, env)?);
            }
            if args.len() != func.params.len() {
                return Err(runtime_error(
                    &format!("Expected {} arguments but got {}", func.params.len(), args.len()),
                    call.location.line,
                    call.location.column
                ));
            }
            let mut function_env = Environment::new_with_parent(env.clone());
            for (param, arg) in func.params.iter().zip(args) {
                function_env.declare(param.ident.clone(), arg, false);
            }
            let mut result = Value::Void;
            for stmt in &func.body {
                match **stmt {
                    Content::Expression(ref expr) => {
                        result = evaluate_expression(expr, &mut function_env)?;
                    },
                    Content::Statement(ref stmt) => {
                        if let Ok(Some(val)) = evaluate_statement(stmt, &mut function_env) {
                            result = val;
                        }
                    }
                }
            }
            Ok(result)
        },
        _ => Err(runtime_error("Cannot call non-function value", call.location.line, call.location.column))
    }
}

fn evaluate_member_expression(member: &MemberExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    let object = evaluate_expression(&member.object, env)?;
    let property = if member.computed {
        evaluate_expression(&member.property, env)?
    } else {
        match *member.property {
            Expr::Identifier(ref ident) => Value::String(ident.name.clone()),
            _ => return Err(runtime_error("Invalid property access", member.location.line, member.location.column))
        }
    };

    match (object, property) {
        (Value::Object(map), Value::String(key)) => {
            map.get(&key)
               .cloned()
               .ok_or_else(|| runtime_error(&format!("Property '{}' not found", key), member.location.line, member.location.column))
        },
        (Value::Array(arr), Value::Int(index)) => {
            if index < 0 || index >= arr.len() as i64 {
                Err(runtime_error(&format!("Index {} out of bounds", index), member.location.line, member.location.column))
            } else {
                Ok(arr[index as usize].clone())
            }
        },
        _ => Err(runtime_error("Invalid member access", member.location.line, member.location.column))
    }
}

fn evaluate_assignment(assign: &AssignExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    let value = evaluate_expression(&assign.right, env)?;
    
    match *assign.left {
        Expr::Identifier(ref ident) => {
            env.assign(&ident.name, value.clone())
                .map_err(|e| runtime_error(&e, assign.location.line, assign.location.column))?;
            Ok(value)
        },
        Expr::Member(ref member) => {
            let mut object = evaluate_expression(&member.object, env)?;
            let property = if member.computed {
                evaluate_expression(&member.property, env)?
            } else {
                match *member.property {
                    Expr::Identifier(ref ident) => Value::String(ident.name.clone()),
                    _ => return Err(runtime_error("Invalid property access", member.location.line, member.location.column))
                }
            };

            match (&mut object, property) {
                (Value::Object(ref mut map), Value::String(key)) => {
                    map.insert(key, value.clone());
                    env.assign(&format!("{:?}", member.object), object)
                        .map_err(|e| runtime_error(&e, assign.location.line, assign.location.column))?;
                    Ok(value)
                },
                (Value::Array(ref mut arr), Value::Int(index)) => {
                    if index < 0 || index >= arr.len() as i64 {
                        Err(runtime_error(&format!("Index {} out of bounds", index), member.location.line, member.location.column))
                    } else {
                        arr[index as usize] = value.clone();
                        env.assign(&format!("{:?}", member.object), object)
                            .map_err(|e| runtime_error(&e, assign.location.line, assign.location.column))?;
                        Ok(value)
                    }
                },
                _ => Err(runtime_error("Invalid assignment target", assign.location.line, assign.location.column))
            }
        },
        _ => Err(runtime_error("Invalid assignment target", assign.location.line, assign.location.column))
    }
}

// Helper functions for binary operations now return Result<Value, String> and are wrapped with runtime_error

fn add_values(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l + r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
        (Value::String(l), Value::String(r)) => Ok(Value::String(l + &r)),
        _ => Err("Invalid addition operation".to_string())
    }
}

fn subtract_values(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l - r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
        _ => Err("Invalid subtraction operation".to_string())
    }
}

fn multiply_values(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l * r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
        _ => Err("Invalid multiplication operation".to_string())
    }
}

fn divide_values(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => {
            if r == 0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Int(l / r))
            }
        }
        (Value::Float(l), Value::Float(r)) => {
            if r == 0.0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Float(l / r))
            }
        }
        _ => Err("Invalid division operation".to_string())
    }
}

fn modulo_values(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => {
            if r == 0 {
                Err("Modulo by zero".to_string())
            } else {
                Ok(Value::Int(l % r))
            }
        }
        _ => Err("Invalid modulo operation".to_string())
    }
}

fn compare_values(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => l == r,
        (Value::Float(l), Value::Float(r)) => l == r,
        (Value::String(l), Value::String(r)) => l == r,
        (Value::Boolean(l), Value::Boolean(r)) => l == r,
        _ => false
    }
}

fn compare_less_than(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Boolean(l < r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l < r)),
        _ => Err("Invalid comparison".to_string())
    }
}

fn compare_greater_than(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Boolean(l > r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l > r)),
        _ => Err("Invalid comparison".to_string())
    }
}

fn compare_less_equal(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Boolean(l <= r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l <= r)),
        _ => Err("Invalid comparison".to_string())
    }
}

fn compare_greater_equal(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Boolean(l >= r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l >= r)),
        _ => Err("Invalid comparison".to_string())
    }
}

fn logical_and(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Boolean(l), Value::Boolean(r)) => Ok(Value::Boolean(l && r)),
        _ => Err("Invalid logical AND operation".to_string())
    }
}

fn logical_or(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Boolean(l), Value::Boolean(r)) => Ok(Value::Boolean(l || r)),
        _ => Err("Invalid logical OR operation".to_string())
    }
}