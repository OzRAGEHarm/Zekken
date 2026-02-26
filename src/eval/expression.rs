use crate::ast::*;
use crate::environment::{Environment, Value};
use std::collections::HashMap;
use crate::eval::statement::evaluate_statement;
use crate::errors::{ZekkenError};
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
            let (val, kind) = env.lookup_with_kind(&ident.name);
            let kind_str = kind.unwrap_or("variable");
            val.ok_or_else(|| ZekkenError::reference(
                &format!("{} '{}' not found", kind_str[0..1].to_uppercase() + &kind_str[1..], &ident.name),
                kind_str,
                ident.location.line,
                ident.location.column,
            ))
        }
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
    // First check for member expressions (method calls)
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

        // Call the method on any value type
        let mut args = Vec::new();
        for arg in &call.args {
            args.push(evaluate_expression(arg, env)?);
        }

        // Try to call the method on any value type (strings, arrays, objects, etc)
        match object.call_method(&method_name, args, Some(env), if let Expr::Identifier(ref ident) = *member_expr.object {
            Some(ident.name.as_str())
        } else {
            None
        }) {
            Ok(result) => return Ok(result),
            Err(msg) => {
                let (line, column, span_len) = call
                    .args
                    .first()
                    .map(|arg| {
                        let loc = expr_location(arg);
                        (loc.line, loc.column, expr_span_len(arg))
                    })
                    .unwrap_or((call.location.line, call.location.column, 1));
                return Err(ZekkenError::runtime_with_span(&msg, line, column, span_len, None));
            }
        };
    }

    // When resolving the callee, try to look up as an identifier first
    if let Expr::Identifier(ref ident) = *call.callee {
        // Try to look up the identifier in the environment
        if let Some(val) = env.lookup_ref(&ident.name).cloned() {
            match &val {
                Value::Function(_) => {
                    return evaluate_function_call(&val, call, env);
                },
                Value::NativeFunction(_) => {
                    return evaluate_native_function_call(&val, call, env);
                },
                _ => {}
            }
        }
        
        return Err(ZekkenError::reference(
            &format!("Function '{}' not found", &ident.name),
            "function",
            call.location.line,
            call.location.column,
        ));
    }

    // If not a method call or identifier, evaluate as a regular expression
    let callee_val = evaluate_expression(&call.callee, env)?;
    match callee_val {
        Value::Function(_) => evaluate_function_call(&callee_val, call, env),
        Value::NativeFunction(_) => evaluate_native_function_call(&callee_val, call, env),
        _ => Err(ZekkenError::type_error(
            "Cannot call non-function value",
            "function",
            "non-function",
            call.location.line,
            call.location.column,
        )),
    }
}

fn evaluate_function_call(func: &Value, call: &CallExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    if let Value::Function(ref func_def) = func {
        if call.args.len() != func_def.params.len() {
            return Err(ZekkenError::runtime(
                &format!("Expected {} arguments but got {}", func_def.params.len(), call.args.len()),
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
        for (param, arg) in func_def.params.iter().zip(args.into_iter()) {
            function_env.declare(param.ident.clone(), arg, false);
        }

        let mut result = Value::Void;
        for stmt in func_def.body.iter() {
            match **stmt {
                Content::Expression(ref expr) => {
                    result = evaluate_expression(expr, &mut function_env)?;
                },
                Content::Statement(ref stmt) => {
                    match evaluate_statement(stmt, &mut function_env) {
                        Ok(Some(val)) => {
                            result = val;
                            if matches!(**stmt, Stmt::Return(_)) {
                                break;
                            }
                        }
                        Ok(None) => {}
                        Err(err) => return Err(err),
                    }
                }
            }
        }
        Ok(result)
    } else {
        Err(ZekkenError::type_error(
            "Cannot call non-function value",
            "function",
            "non-function",
            call.location.line,
            call.location.column,
        ))
    }
}

fn evaluate_native_function_call(native_func: &Value, call: &CallExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    if let Value::NativeFunction(ref native) = native_func {
        let mut args = Vec::new();
        for arg in &call.args {
            args.push(evaluate_expression(arg, env)?);
        }
        match (native)(args) {
            Ok(val) => Ok(val),
            Err(s) => {
                let (line, column, span_len) = call
                    .args
                    .first()
                    .map(|arg| {
                        let loc = expr_location(arg);
                        (loc.line, loc.column, expr_span_len(arg))
                    })
                    .unwrap_or((call.location.line, call.location.column, 1));
                Err(ZekkenError::runtime_with_span(&s, line, column, span_len, None))
            }
        }
    } else {
        Err(ZekkenError::type_error(
            "Cannot call non-function value",
            "function",
            "non-function",
            call.location.line,
            call.location.column,
        ))
    }
}

fn expr_location(expr: &Expr) -> Location {
    match expr {
        Expr::Assign(e) => e.location.clone(),
        Expr::Member(e) => e.location.clone(),
        Expr::Call(e) => e.location.clone(),
        Expr::Binary(e) => e.location.clone(),
        Expr::Identifier(e) => e.location.clone(),
        Expr::Property(e) => e.location.clone(),
        Expr::IntLit(e) => e.location.clone(),
        Expr::FloatLit(e) => e.location.clone(),
        Expr::StringLit(e) => e.location.clone(),
        Expr::BoolLit(e) => e.location.clone(),
        Expr::ArrayLit(e) => e.location.clone(),
        Expr::ObjectLit(e) => e.location.clone(),
    }
}

fn expr_span_len(expr: &Expr) -> usize {
    match expr {
        Expr::StringLit(lit) => lit.value.chars().count() + 2, // include quotes
        Expr::Identifier(id) => id.name.chars().count().max(1),
        Expr::IntLit(lit) => lit.value.to_string().chars().count().max(1),
        Expr::FloatLit(lit) => lit.value.to_string().chars().count().max(1),
        Expr::BoolLit(lit) => {
            if lit.value { 4 } else { 5 }
        }
        _ => 1,
    }
}

fn evaluate_member_expression(member: &MemberExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    let object = evaluate_expression(&member.object, env)?;
    let result = match &*member.property {
        Expr::Identifier(ref ident) => {
            // Support dynamic indexing like arr[i] / obj[i] when `i` resolves to a number.
            if let Some(index_val) = env.lookup_ref(&ident.name) {
                match index_val {
                    Value::Int(i) if *i >= 0 => {
                        evaluate_index_access(&object, *i as usize, member.location.line, member.location.column)
                    }
                    Value::Float(f) if *f >= 0.0 && f.fract() == 0.0 => {
                        evaluate_index_access(&object, *f as usize, member.location.line, member.location.column)
                    }
                    _ => evaluate_property_access(&object, &ident.name, member.location.line, member.location.column),
                }
            } else {
                evaluate_property_access(&object, &ident.name, member.location.line, member.location.column)
            }
        }
        Expr::StringLit(ref lit) => evaluate_property_access(&object, &lit.value, member.location.line, member.location.column),
        Expr::IntLit(ref lit) => evaluate_index_access(&object, lit.value as usize, member.location.line, member.location.column),
        _ => Err(ZekkenError::type_error(
            "Invalid property access",
            "string/int/identifier",
            "other",
            member.location.line,
            member.location.column,
        )),
    }?;
    Ok(result)
}

fn evaluate_property_access(object: &Value, property: &str, line: usize, column: usize) -> Result<Value, ZekkenError> {
    match object {
        Value::Object(map) => {
            map.get(property)
                .cloned()
                .ok_or_else(|| ZekkenError::reference(
                    &format!("Property '{}' not found", property),
                    property,
                    line,
                    column,
                ))
        }
        _ => Err(ZekkenError::type_error(
            "Invalid member access",
            "object",
            "other",
            line,
            column,
        )),
    }
}

fn evaluate_index_access(object: &Value, idx: usize, line: usize, column: usize) -> Result<Value, ZekkenError> {
    match object {
        Value::Array(arr) => {
            arr.get(idx)
                .cloned()
                .ok_or_else(|| ZekkenError::runtime(
                    &format!("Array index {} out of bounds", idx),
                    line,
                    column,
                    None,
                ))
        }
        Value::Object(map) => {
            // Support numeric indexing for objects with __keys__
            if let Some(Value::Array(keys)) = map.get("__keys__") {
                if let Some(Value::String(key)) = keys.get(idx) {
                    map.get(key)
                        .cloned()
                        .ok_or_else(|| ZekkenError::reference(
                            &format!("Property '{}' not found", key),
                            key,
                            line,
                            column,
                        ))
                } else {
                    Err(ZekkenError::runtime(
                        &format!("Object index {} out of bounds", idx),
                        line,
                        column,
                        None,
                    ))
                }
            } else {
                Err(ZekkenError::runtime(
                    "Object does not support numeric indexing",
                    line,
                    column,
                    None,
                ))
            }
        }
        _ => Err(ZekkenError::type_error(
            "Invalid member access",
            "object/array",
            "other",
            line,
            column,
        )),
    }
}

fn evaluate_assignment(assign: &AssignExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    enum AssignTarget {
        Identifier(String),
        Member(Box<Expr>),
    }

    let target = match &*assign.left {
        Expr::Identifier(ident) => AssignTarget::Identifier(ident.name.clone()),
        Expr::Member(_) => AssignTarget::Member(assign.left.clone()),
        _ => {
            return Err(ZekkenError::type_error(
                "Invalid assignment target",
                "identifier or member access",
                "other",
                assign.location.line,
                assign.location.column,
            ))
        }
    };

    // Fast path: in-place compound assignment for identifiers to avoid cloning
    // large values (especially arrays) in tight loops.
    if let AssignTarget::Identifier(name) = &target {
        if assign.operator != "=" {
            let right_val = evaluate_expression(&assign.right, env)?;
            if let Ok(left_slot) = env.lookup_mut_assignable(name) {
                match assign.operator.as_str() {
                    "+=" => match left_slot {
                        Value::Int(l) => match &right_val {
                            Value::Int(r) => return Ok(Value::Int({ *l += *r; *l })),
                            Value::Float(r) => {
                                let v = *l as f64 + *r;
                                *left_slot = Value::Float(v);
                                return Ok(Value::Float(v));
                            }
                            _ => {}
                        },
                        Value::Float(l) => match &right_val {
                            Value::Float(r) => return Ok(Value::Float({ *l += *r; *l })),
                            Value::Int(r) => return Ok(Value::Float({ *l += *r as f64; *l })),
                            _ => {}
                        },
                        Value::String(l) => match &right_val {
                            Value::String(r) => {
                                l.push_str(r);
                                return Ok(Value::String(l.clone()));
                            }
                            other => {
                                l.push_str(&other.to_string());
                                return Ok(Value::String(l.clone()));
                            }
                        },
                        Value::Array(l) => {
                            if let Value::Array(r) = &right_val {
                                l.extend(r.iter().cloned());
                                return Ok(Value::Array(l.clone()));
                            }
                        }
                        _ => {}
                    },
                    "-=" => match left_slot {
                        Value::Int(l) => match &right_val {
                            Value::Int(r) => return Ok(Value::Int({ *l -= *r; *l })),
                            Value::Float(r) => {
                                let v = *l as f64 - *r;
                                *left_slot = Value::Float(v);
                                return Ok(Value::Float(v));
                            }
                            _ => {}
                        },
                        Value::Float(l) => match &right_val {
                            Value::Float(r) => return Ok(Value::Float({ *l -= *r; *l })),
                            Value::Int(r) => return Ok(Value::Float({ *l -= *r as f64; *l })),
                            _ => {}
                        },
                        _ => {}
                    },
                    "*=" => match left_slot {
                        Value::Int(l) => match &right_val {
                            Value::Int(r) => return Ok(Value::Int({ *l *= *r; *l })),
                            Value::Float(r) => {
                                let v = *l as f64 * *r;
                                *left_slot = Value::Float(v);
                                return Ok(Value::Float(v));
                            }
                            _ => {}
                        },
                        Value::Float(l) => match &right_val {
                            Value::Float(r) => return Ok(Value::Float({ *l *= *r; *l })),
                            Value::Int(r) => return Ok(Value::Float({ *l *= *r as f64; *l })),
                            _ => {}
                        },
                        _ => {}
                    },
                    "/=" => match left_slot {
                        Value::Int(l) => match &right_val {
                            Value::Int(r) => {
                                if *r == 0 {
                                    return Err(ZekkenError::runtime("Division by zero", assign.location.line, assign.location.column, None));
                                }
                                return Ok(Value::Int({ *l /= *r; *l }));
                            }
                            Value::Float(r) => {
                                if *r == 0.0 {
                                    return Err(ZekkenError::runtime("Division by zero", assign.location.line, assign.location.column, None));
                                }
                                let v = *l as f64 / *r;
                                *left_slot = Value::Float(v);
                                return Ok(Value::Float(v));
                            }
                            _ => {}
                        },
                        Value::Float(l) => match &right_val {
                            Value::Float(r) => {
                                if *r == 0.0 {
                                    return Err(ZekkenError::runtime("Division by zero", assign.location.line, assign.location.column, None));
                                }
                                return Ok(Value::Float({ *l /= *r; *l }));
                            }
                            Value::Int(r) => {
                                if *r == 0 {
                                    return Err(ZekkenError::runtime("Division by zero", assign.location.line, assign.location.column, None));
                                }
                                return Ok(Value::Float({ *l /= *r as f64; *l }));
                            }
                            _ => {}
                        },
                        _ => {}
                    },
                    "%=" => {
                        if let Value::Int(l) = left_slot {
                            if let Value::Int(r) = &right_val {
                                if *r == 0 {
                                    return Err(ZekkenError::runtime("Modulo by zero", assign.location.line, assign.location.column, None));
                                }
                                *l %= *r;
                                return Ok(Value::Int(*l));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let left_val = match &target {
        AssignTarget::Identifier(name) => env.lookup(name).ok_or_else(|| {
            ZekkenError::reference(
                &format!("Variable '{}' not found", name),
                name,
                assign.location.line,
                assign.location.column,
            )
        })?,
        AssignTarget::Member(expr) => evaluate_expression(expr, env)?,
    };

    let value_to_store = if assign.operator != "=" {
        let right_val = evaluate_expression(&assign.right, env)?;
        match assign.operator.as_str() {
            "+=" => add_values(&left_val, &right_val),
            "-=" => subtract_values(&left_val, &right_val),
            "*=" => multiply_values(&left_val, &right_val),
            "/=" => divide_values(&left_val, &right_val),
            "%=" => modulo_values(left_val, right_val),
            _ => {
                return Err(ZekkenError::runtime(
                    &format!("Unknown operator: {}", assign.operator),
                    assign.location.line,
                    assign.location.column,
                    None,
                ))
            }
        }
        .map_err(|e| ZekkenError::runtime(&e, assign.location.line, assign.location.column, None))?
    } else {
        evaluate_expression(&assign.right, env)?
    };

    match target {
        AssignTarget::Identifier(name) => {
            env.assign(&name, value_to_store.clone()).map_err(|err| {
                ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)
            })?;
        }
        AssignTarget::Member(member_expr) => {
            assign_to_member(&member_expr, value_to_store.clone(), env).map_err(|err| {
                ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)
            })?;
        }
    }

    Ok(value_to_store)
}

#[derive(Debug, Clone)]
enum MemberKey {
    Property(String),
    Index(usize),
}

fn resolve_member_key(expr: &Expr, env: &Environment) -> Result<MemberKey, String> {
    match expr {
        Expr::Identifier(id) => {
            if let Some(val) = env.lookup_ref(&id.name) {
                match val {
                    Value::Int(i) if *i >= 0 => Ok(MemberKey::Index(*i as usize)),
                    Value::Float(f) if *f >= 0.0 && f.fract() == 0.0 => Ok(MemberKey::Index(*f as usize)),
                    _ => Ok(MemberKey::Property(id.name.clone())),
                }
            } else {
                Ok(MemberKey::Property(id.name.clone()))
            }
        }
        Expr::StringLit(s) => Ok(MemberKey::Property(s.value.clone())),
        Expr::IntLit(i) if i.value >= 0 => Ok(MemberKey::Index(i.value as usize)),
        Expr::FloatLit(f) if f.value >= 0.0 && f.value.fract() == 0.0 => Ok(MemberKey::Index(f.value as usize)),
        _ => Err("Invalid member key for assignment".to_string()),
    }
}

fn collect_member_path(expr: &Expr, env: &Environment) -> Result<(String, Vec<MemberKey>), String> {
    match expr {
        Expr::Identifier(id) => Ok((id.name.clone(), Vec::new())),
        Expr::Member(member) => {
            let (root, mut path) = collect_member_path(&member.object, env)?;
            path.push(resolve_member_key(&member.property, env)?);
            Ok((root, path))
        }
        _ => Err("Invalid member assignment target".to_string()),
    }
}

fn assign_at_path(current: &mut Value, path: &[MemberKey], value: Value) -> Result<(), String> {
    if path.is_empty() {
        *current = value;
        return Ok(());
    }

    match &path[0] {
        MemberKey::Index(idx) => match current {
            Value::Array(arr) => {
                if *idx >= arr.len() {
                    return Err(format!("Array index {} out of bounds", idx));
                }
                assign_at_path(&mut arr[*idx], &path[1..], value)
            }
            Value::Object(map) => {
                let key_for_index = match map.get("__keys__") {
                    Some(Value::Array(keys)) => match keys.get(*idx) {
                        Some(Value::String(key)) => Some(key.clone()),
                        _ => None,
                    },
                    _ => None,
                };
                if let Some(key) = key_for_index {
                    if let Some(next) = map.get_mut(&key) {
                        assign_at_path(next, &path[1..], value)
                    } else {
                        Err(format!("Property '{}' not found", key))
                    }
                } else {
                    Err("Object does not support numeric indexing".to_string())
                }
            }
            _ => Err("Invalid member assignment target (indexing non-array/object)".to_string()),
        },
        MemberKey::Property(prop) => match current {
            Value::Object(map) => {
                if path.len() == 1 {
                    map.insert(prop.clone(), value);
                    Ok(())
                } else if let Some(next) = map.get_mut(prop) {
                    assign_at_path(next, &path[1..], value)
                } else {
                    Err(format!("Property '{}' not found", prop))
                }
            }
            _ => Err("Invalid member assignment target (property on non-object)".to_string()),
        },
    }
}

fn assign_to_member(member_expr: &Expr, value: Value, env: &mut Environment) -> Result<(), String> {
    let (root, path) = collect_member_path(member_expr, env)?;
    let root_value = env.lookup_mut_assignable(&root)?;
    assign_at_path(root_value, &path, value)
}

fn add_values(left: &Value, right: &Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l + r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
        (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 + r)),
        (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l + *r as f64)),
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
        (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 - r)),
        (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l - *r as f64)),
        _ => Err("Invalid operand types for subtraction".to_string())
    }
}

fn multiply_values(left: &Value, right: &Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l * r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
        (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 * r)),
        (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l * *r as f64)),
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
        (Value::Int(l), Value::Float(r)) => {
            if *r == 0.0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Float(*l as f64 / r))
            }
        }
        (Value::Float(l), Value::Int(r)) => {
            if *r == 0 {
                Err("Division by zero".to_string())
            } else {
                Ok(Value::Float(l / *r as f64))
            }
        }
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
        (Value::Int(l), Value::Float(r)) => Ok(Value::Boolean((l as f64) < r)),
        (Value::Float(l), Value::Int(r)) => Ok(Value::Boolean(l < (r as f64))),
        _ => Err("Invalid comparison".to_string())
    }
}

fn compare_greater_than(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Boolean(l > r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l > r)),
        (Value::Int(l), Value::Float(r)) => Ok(Value::Boolean((l as f64) > r)),
        (Value::Float(l), Value::Int(r)) => Ok(Value::Boolean(l > (r as f64))),
        _ => Err("Invalid comparison".to_string())
    }
}

fn compare_less_equal(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Boolean(l <= r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l <= r)),
        (Value::Int(l), Value::Float(r)) => Ok(Value::Boolean((l as f64) <= r)),
        (Value::Float(l), Value::Int(r)) => Ok(Value::Boolean(l <= (r as f64))),
        _ => Err("Invalid comparison".to_string())
    }
}

fn compare_greater_equal(left: Value, right: Value) -> Result<Value, String> {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => Ok(Value::Boolean(l >= r)),
        (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l >= r)),
        (Value::Int(l), Value::Float(r)) => Ok(Value::Boolean((l as f64) >= r)),
        (Value::Float(l), Value::Int(r)) => Ok(Value::Boolean(l >= (r as f64))),
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
