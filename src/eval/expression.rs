use crate::ast::*;
use crate::environment::{Environment, Value};
use std::collections::HashMap;
use crate::eval::statement::evaluate_statement;
use crate::errors::ZekkenError;
//use regex::Regex;
//use crate::lexer::DataType;
//use std::sync::Arc;

/*
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
*/

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
                None => Err(ZekkenError::reference(
                    &format!("Variable '{}' not found", ident.name),
                    &ident.name,
                    ident.location.line,
                    ident.location.column
                ))
            }
        },
        Expr::Binary(binary) => evaluate_binary_expression(binary, env),
        Expr::Call(call) => evaluate_call_expression(call, env),
        Expr::Member(member) => evaluate_member_expression(member, env),
        Expr::Assign(assign) => evaluate_assignment(assign, env),
        Expr::Property(_) => Err(ZekkenError::internal(
            "Property expression not supported in this context",
        ))
    }
}

fn evaluate_binary_expression(expr: &BinaryExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    let left = evaluate_expression(&expr.left, env)?;
    let right = evaluate_expression(&expr.right, env)?;

    if (matches!(left, Value::Int(_)) && matches!(right, Value::Float(_))) ||
       (matches!(left, Value::Float(_)) && matches!(right, Value::Int(_))) {
        return Err(ZekkenError::type_error(
            &format!("Cannot perform '{}' operation between int and float", expr.operator),
            "int or float",
            "mixed types",
            expr.location.line,
            expr.location.column,
        ));
    }
    
    match expr.operator.as_str() {
        "+" => add_values(&left, &right)
            .map_err(|msg| ZekkenError::type_error(&msg, "valid types", "invalid types", expr.location.line, expr.location.column)),
        "-" => subtract_values(&left, &right)
            .map_err(|msg| ZekkenError::type_error(&msg, "valid types", "invalid types", expr.location.line, expr.location.column)),
        "*" => multiply_values(&left, &right)
            .map_err(|msg| ZekkenError::type_error(&msg, "valid types", "invalid types", expr.location.line, expr.location.column)),
        "/" => divide_values(&left, &right)
            .map_err(|msg| ZekkenError::runtime(
                &msg, 
                expr.location.line,
                expr.location.column,
                if msg.contains("zero") { Some("division by zero") } else { None },
            )),
        "%" => modulo_values(left, right)
            .map_err(|msg| ZekkenError::type_error(&msg, "valid types", "invalid types", expr.location.line, expr.location.column)),
        "==" => Ok(Value::Boolean(compare_values(&left, &right))),
        "!=" => Ok(Value::Boolean(!compare_values(&left, &right))),
        "<" => compare_less_than(left, right)
            .map_err(|e| ZekkenError::type_error(&e, "valid types", "invalid types", expr.location.line, expr.location.column)),
        ">" => compare_greater_than(left, right)
            .map_err(|e| ZekkenError::type_error(&e, "valid types", "invalid types", expr.location.line, expr.location.column)),
        "<=" => compare_less_equal(left, right)
            .map_err(|e| ZekkenError::type_error(&e, "valid types", "invalid types", expr.location.line, expr.location.column)),
        ">=" => compare_greater_equal(left, right)
            .map_err(|e| ZekkenError::type_error(&e, "valid types", "invalid types", expr.location.line, expr.location.column)),
        "&&" => logical_and(left, right)
            .map_err(|e| ZekkenError::type_error(&e, "boolean", "non-boolean", expr.location.line, expr.location.column)),
        "||" => logical_or(left, right)
            .map_err(|e| ZekkenError::type_error(&e, "boolean", "non-boolean", expr.location.line, expr.location.column)),
        operator => Err(ZekkenError::runtime(
            &format!("Unknown operator: {}", operator), 
            expr.location.line, 
            expr.location.column,
            None
        ))
    }
}

/*
fn interpolate_string(template: &str, env: &Environment) -> String {
    let re = Regex::new(r"\{(\w+)\}").unwrap();
    re.replace_all(template, |caps: &regex::Captures| {
        let var_name = &caps[1];
        match env.lookup(var_name) {
            Some(value) => format!("{}", value),
            None => format!("{{{}}}", var_name)
        }
    }).to_string()
}
*/

fn evaluate_call_expression(call: &CallExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    if let Expr::Member(ref member_expr) = *call.callee {
        let object = evaluate_expression(&member_expr.object, env)?;
        let method_name = match *member_expr.property {
            Expr::Identifier(ref ident) => ident.name.clone(),
            _ => return Err(ZekkenError::type_error(
                "Invalid method name",
                "identifier",
                "other",
                call.location.line,
                call.location.column,
            )),
        };

        let mut args = Vec::new();
        for arg in &call.args {
            args.push(evaluate_expression(arg, env)?);
        }

        // Determine the variable name, if applicable
        let variable_name = if let Expr::Identifier(ref ident) = *member_expr.object {
            Some(ident.name.as_str())
        } else {
            None
        };

        // Call the method with the environment and variable name
        return object.call_method(&method_name, args, Some(env), variable_name)
            .map_err(|s| ZekkenError::runtime(&s, call.location.line, call.location.column, None));
    }

    let callee = evaluate_expression(&call.callee, env)?;
    match callee {
        Value::NativeFunction(native_func) => {
            let mut args = Vec::new();
            for arg in &call.args {
                args.push(evaluate_expression(arg, env)?);
            }
            (native_func)(args).map_err(|s| ZekkenError::runtime(&s, call.location.line, call.location.column, None))
        },
        Value::Function(func) => {
            if call.args.len() != func.params.len() {
                return Err(ZekkenError::runtime(
                    &format!("Expected {} arguments but got {}", func.params.len(), call.args.len()),
                    call.location.line,
                    call.location.column,
                    Some("argument mismatch"),
                ));
            }

            let mut args = Vec::new();
            for arg in &call.args {
                args.push(evaluate_expression(arg, env)?);
            }

            let mut function_env = Environment::new_with_parent(env.clone());
            for (param, arg) in func.params.iter().zip(args.into_iter()) {
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
        _ => Err(ZekkenError::type_error(
            "Cannot call non-function value",
            "function",
            "non-function",
            call.location.line,
            call.location.column
        ))
    }
}

fn evaluate_member_expression(member: &MemberExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    let object = evaluate_expression(&member.object, env)?;

    match &*member.property {
        Expr::Identifier(ref ident) => {
            let property = ident.name.clone();
            match object {
                Value::Object(ref map) => {
                    if let Some(value) = map.get(&property) {
                        Ok(value.clone())
                    } else {
                        Err(ZekkenError::reference(
                            &format!("Property '{}' not found", property),
                            &property,
                            member.location.line,
                            member.location.column,
                        ))
                    }
                }
                _ => Err(ZekkenError::type_error(
                    "Invalid member access",
                    "object",
                    "other",
                    member.location.line,
                    member.location.column,
                )),
            }
        }
        Expr::StringLit(ref lit) => {
            let property = lit.value.clone();
            match object {
                Value::Object(ref map) => {
                    if let Some(value) = map.get(&property) {
                        Ok(value.clone())
                    } else {
                        Err(ZekkenError::reference(
                            &format!("Property '{}' not found", property),
                            &property,
                            member.location.line,
                            member.location.column,
                        ))
                    }
                }
                _ => Err(ZekkenError::type_error(
                    "Invalid member access",
                    "object",
                    "other",
                    member.location.line,
                    member.location.column,
                )),
            }
        }
        Expr::IntLit(ref lit) => {
            let idx = lit.value as usize;
            match object {
                Value::Array(ref arr) => {
                    arr.get(idx)
                        .cloned()
                        .ok_or_else(|| ZekkenError::runtime(
                            &format!("Array index {} out of bounds", idx),
                            member.location.line,
                            member.location.column,
                            None,
                        ))
                }
                Value::Object(ref map) => {
                    // Support numeric indexing for objects with __keys__
                    if let Some(Value::Array(keys)) = map.get("__keys__") {
                        if let Some(Value::String(key)) = keys.get(idx) {
                            if let Some(value) = map.get(key) {
                                Ok(value.clone())
                            } else {
                                Err(ZekkenError::reference(
                                    &format!("Property '{}' not found", key),
                                    key,
                                    member.location.line,
                                    member.location.column,
                                ))
                            }
                        } else {
                            Err(ZekkenError::runtime(
                                &format!("Object index {} out of bounds", idx),
                                member.location.line,
                                member.location.column,
                                None,
                            ))
                        }
                    } else {
                        Err(ZekkenError::runtime(
                            "Object does not support numeric indexing",
                            member.location.line,
                            member.location.column,
                            None,
                        ))
                    }
                }
                _ => Err(ZekkenError::type_error(
                    "Invalid member access",
                    "object/array",
                    "other",
                    member.location.line,
                    member.location.column,
                )),
            }
        }
        _ => Err(ZekkenError::type_error(
            "Invalid property access",
            "string/int/identifier",
            "other",
            member.location.line,
            member.location.column,
        )),
    }
}

fn evaluate_assignment(assign: &AssignExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    let left = match *assign.left {
        Expr::Identifier(ref ident) => ident.name.clone(),
        _ => return Err(ZekkenError::type_error(
            "Invalid assignment target",
            "identifier",
            "other",
            assign.location.line,
            assign.location.column
        )),
    };

    // Handle compound assignments (+=, -=, *=, /=, %=)
    if assign.operator != "=" {
        let left_val = env.lookup(&left).ok_or_else(|| ZekkenError::reference(
            &format!("Variable '{}' not found", left),
            &left,
            assign.location.line,
            assign.location.column
        ))?;
        let right_val = evaluate_expression(&assign.right, env)?;

        let result = match assign.operator.as_str() {
            "+=" => add_values(&left_val, &right_val),
            "-=" => subtract_values(&left_val, &right_val),
            "*=" => multiply_values(&left_val, &right_val),
            "/=" => divide_values(&left_val, &right_val),
            "%=" => modulo_values(left_val, right_val),
            _ => return Err(ZekkenError::runtime(
                &format!("Unknown operator: {}", assign.operator),
                assign.location.line,
                assign.location.column,
                None
            )),
        };

        let result_cloned = result.clone();
        let result = result.map_err(|e| ZekkenError::runtime(&e, assign.location.line, assign.location.column, None))?;
        match result_cloned {
            Ok(value) => {
                env.assign(&left, value).map_err(|err| ZekkenError::runtime(&err, assign.location.line, assign.location.column, None))?;
            },
            Err(err) => return Err(ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)),
        }
        return Ok(result);
    }

    // Regular assignment
    let value = evaluate_expression(&assign.right, env)?;
    if let Err(err) = env.assign(&left, value.clone()) {
        return Err(ZekkenError::runtime(&err, assign.location.line, assign.location.column, None));
    }
    Ok(value)
}

fn add_values(left: &Value, right: &Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l + r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
        (Value::String(l), Value::String(r)) => Ok(Value::String(l.clone() + r)),
        (Value::String(l), other) => Ok(Value::String(l.clone() + &other.to_string())),
        (other, Value::String(r)) => Ok(Value::String(other.to_string() + r)),
        (Value::Array(l), Value::Array(r)) => {
            let mut result = l.clone();
            result.extend(r.clone());
            Ok(Value::Array(result))
        },
        _ => Err("Invalid operand types for addition".to_string())
    }
}

fn subtract_values(left: &Value, right: &Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l - r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
        _ => Err("Invalid operand types for subtraction".to_string())
    }
}

fn multiply_values(left: &Value, right: &Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l * r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
        _ => Err("Invalid operand types for multiplication".to_string())
    }
}

fn divide_values(left: &Value, right: &Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => {
            if *r == 0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Int(l / r))
            }
        },
        (Value::Float(l), Value::Float(r)) => {
            if *r == 0.0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Float(l / r))
            }
        },
        _ => Err("Invalid operand types for division".to_string())
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
        },
        _ => Err("Invalid operand types for modulo".to_string())
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