use crate::ast::*;
use crate::bytecode;
use crate::environment::{Environment, FunctionValue, Value};
use crate::lexer::DataType;
use hashbrown::HashMap;
use std::sync::Arc;
use crate::errors::{ZekkenError};
use crate::parser::Parser;

fn check_value_type(value: &Value, expected: &DataType) -> bool {
    match (value, expected) {
        (_, DataType::Any) => true,
        (Value::Int(_), DataType::Int) => true,
        (Value::Float(_), DataType::Float) => true,
        (Value::String(_), DataType::String) => true,
        (Value::Boolean(_), DataType::Bool) => true,
        (Value::Array(_), DataType::Array) => true,
        (Value::Object(_), DataType::Object) => true,
        (Value::Function(_), DataType::Fn) => true,
        (Value::NativeFunction(_), DataType::Fn) => true,
        _ => false,
    }
}

fn value_type_name(val: &Value) -> &'static str {
    match val {
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Boolean(_) => "bool",
        Value::Array(_) => "arr",
        Value::Object(_) => "obj",
        Value::Function(_) | Value::NativeFunction(_) => "fn",
        _ => "other",
    }
}

pub fn evaluate_expression(expr: &Expr, env: &mut Environment) -> Result<Value, ZekkenError> {
    match expr {
        Expr::IntLit(int) => Ok(Value::Int(int.value)),
        Expr::FloatLit(float) => Ok(Value::Float(float.value)),
        Expr::StringLit(string) => {
            if string.value.as_bytes().contains(&b'{') {
                Ok(Value::String(interpolate_string_expressions(&string.value, env)))
            } else {
                Ok(Value::String(string.value.clone()))
            }
        },
        Expr::BoolLit(bool) => Ok(Value::Boolean(bool.value)),
        Expr::ArrayLit(array) => {
            let mut values = Vec::with_capacity(array.elements.len());
            for element in &array.elements {
                values.push(evaluate_expression(element, env)?);
            }
            Ok(Value::Array(values))
        },
        Expr::ObjectLit(object) => {
            let mut map = HashMap::with_capacity(object.properties.len());
            for prop in &object.properties {
                let value = evaluate_expression(&prop.value, env)?;
                map.insert(prop.key.clone(), value);
            }
            Ok(Value::Object(map))
        },
        Expr::Identifier(ident) => {
            if let Some(v) = env.variables.get(&ident.name).or_else(|| env.constants.get(&ident.name)) {
                return match v {
                    Value::Int(i) => Ok(Value::Int(*i)),
                    Value::Float(f) => Ok(Value::Float(*f)),
                    Value::Boolean(b) => Ok(Value::Boolean(*b)),
                    _ => Ok(v.clone()),
                };
            }
            env.lookup_ref(&ident.name).map(|v| {
                match v {
                    Value::Int(i) => Value::Int(*i),
                    Value::Float(f) => Value::Float(*f),
                    Value::Boolean(b) => Value::Boolean(*b),
                    _ => v.clone(),
                }
            }).ok_or_else(|| {
                ZekkenError::reference(
                    &format!("Variable '{}' not found", &ident.name),
                    "variable",
                    ident.location.line,
                    ident.location.column,
                )
            })
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

fn interpolate_string_expressions(template: &str, env: &mut Environment) -> String {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0usize;
    let mut segment_start = 0usize;

    while i < bytes.len() {
        if bytes[i] != b'{' {
            i += 1;
            continue;
        }

        let mut j = i + 1;
        while j < bytes.len() && bytes[j] != b'}' {
            j += 1;
        }
        if j >= bytes.len() {
            break;
        }

        let raw_inner = &template[i + 1..j];
        let inner = raw_inner.trim();
        out.push_str(&template[segment_start..i]);

        if inner.is_empty() {
            // Keep positional placeholder for println-style formatting.
            out.push_str("{}");
        } else {
            let mut parser = Parser::new();
            let program = parser.produce_ast(inner.to_string());
            let expr = program.content.first().and_then(|c| match c.as_ref() {
                Content::Expression(e) => Some(e.as_ref().clone()),
                _ => None,
            });

            if !parser.errors.is_empty() {
                out.push('{');
                out.push_str(raw_inner);
                out.push('}');
            } else if let Some(expr) = expr {
                match evaluate_expression(&expr, env) {
                    Ok(value) => out.push_str(&value.to_string()),
                    Err(_) => {
                        out.push('{');
                        out.push_str(raw_inner);
                        out.push('}');
                    }
                }
            } else {
                out.push('{');
                out.push_str(raw_inner);
                out.push('}');
            }
        }

        i = j + 1;
        segment_start = i;
    }

    out.push_str(&template[segment_start..]);
    out
}

fn evaluate_binary_expression(expr: &BinaryExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    if let Some(v) = try_eval_numeric_binary(expr, env)? {
        return Ok(v);
    }

    if expr.operator == "&&" {
        let left = evaluate_expression(&expr.left, env)?;
        return match left {
            Value::Boolean(false) => Ok(Value::Boolean(false)),
            Value::Boolean(true) => match evaluate_expression(&expr.right, env)? {
                Value::Boolean(r) => Ok(Value::Boolean(r)),
                _ => Err(ZekkenError::type_error(
                    "Invalid logical AND operation",
                    "boolean",
                    "non-boolean",
                    expr.location.line,
                    expr.location.column,
                )),
            },
            _ => Err(ZekkenError::type_error(
                "Invalid logical AND operation",
                "boolean",
                "non-boolean",
                expr.location.line,
                expr.location.column,
            )),
        };
    }

    if expr.operator == "||" {
        let left = evaluate_expression(&expr.left, env)?;
        return match left {
            Value::Boolean(true) => Ok(Value::Boolean(true)),
            Value::Boolean(false) => match evaluate_expression(&expr.right, env)? {
                Value::Boolean(r) => Ok(Value::Boolean(r)),
                _ => Err(ZekkenError::type_error(
                    "Invalid logical OR operation",
                    "boolean",
                    "non-boolean",
                    expr.location.line,
                    expr.location.column,
                )),
            },
            _ => Err(ZekkenError::type_error(
                "Invalid logical OR operation",
                "boolean",
                "non-boolean",
                expr.location.line,
                expr.location.column,
            )),
        };
    }

    let left = evaluate_expression(&expr.left, env)?;
    let right = evaluate_expression(&expr.right, env)?;
    
    match expr.operator.as_str() {
        "in" => match (&left, &right) {
            (_, Value::Array(arr)) => Ok(Value::Boolean(
                arr.iter().any(|v| compare_values(&left, v)),
            )),
            (Value::String(key), Value::Object(obj)) => Ok(Value::Boolean(obj.contains_key(key))),
            (Value::String(needle), Value::String(haystack)) => {
                Ok(Value::Boolean(haystack.contains(needle)))
            }
            _ => Err(ZekkenError::type_error(
                "Invalid 'in' operation",
                "value in array, string in object, or string in string",
                "incompatible operands",
                expr.location.line,
                expr.location.column,
            )),
        },
        "+" => match (&left, &right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l + r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 + r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l + *r as f64)),
            (Value::String(l), Value::String(r)) => Ok(Value::String(l.clone() + r)),
            (Value::String(l), other) => Ok(Value::String(l.clone() + &other.to_string())),
            (other, Value::String(r)) => Ok(Value::String(other.to_string() + r)),
            (Value::Array(l), Value::Array(r)) => {
                let mut result = Vec::with_capacity(l.len() + r.len());
                result.extend(l.iter().cloned());
                result.extend(r.iter().cloned());
                Ok(Value::Array(result))
            }
            _ => Err(ZekkenError::type_error(
                "Invalid operand types for addition",
                "valid types",
                "invalid types",
                expr.location.line,
                expr.location.column,
            )),
        },
        "-" => match (&left, &right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l - r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 - r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l - *r as f64)),
            _ => Err(ZekkenError::type_error(
                "Invalid operand types for subtraction",
                "valid types",
                "invalid types",
                expr.location.line,
                expr.location.column,
            )),
        },
        "*" => match (&left, &right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l * r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 * r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l * *r as f64)),
            _ => Err(ZekkenError::type_error(
                "Invalid operand types for multiplication",
                "valid types",
                "invalid types",
                expr.location.line,
                expr.location.column,
            )),
        },
        "/" => match (&left, &right) {
            (Value::Int(_), Value::Int(r)) if *r == 0 => Err(ZekkenError::runtime(
                "Division by zero",
                expr.location.line,
                expr.location.column,
                Some("division by zero"),
            )),
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l / r)),
            (Value::Float(_), Value::Float(r)) if *r == 0.0 => Err(ZekkenError::runtime(
                "Division by zero",
                expr.location.line,
                expr.location.column,
                Some("division by zero"),
            )),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l / r)),
            (Value::Int(_), Value::Float(r)) if *r == 0.0 => Err(ZekkenError::runtime(
                "Division by zero",
                expr.location.line,
                expr.location.column,
                Some("division by zero"),
            )),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 / r)),
            (Value::Float(_), Value::Int(r)) if *r == 0 => Err(ZekkenError::runtime(
                "Division by zero",
                expr.location.line,
                expr.location.column,
                Some("division by zero"),
            )),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l / *r as f64)),
            _ => Err(ZekkenError::runtime(
                "Invalid operand types for division",
                expr.location.line,
                expr.location.column,
                None,
            )),
        },
        "%" => match (&left, &right) {
            (Value::Int(_), Value::Int(r)) if *r == 0 => Err(ZekkenError::runtime(
                "Modulo by zero",
                expr.location.line,
                expr.location.column,
                None,
            )),
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l % r)),
            _ => Err(ZekkenError::type_error(
                "Invalid operand types for modulo",
                "valid types",
                "invalid types",
                expr.location.line,
                expr.location.column,
            )),
        },
        "==" => Ok(Value::Boolean(compare_values(&left, &right))),
        "!=" => Ok(Value::Boolean(!compare_values(&left, &right))),
        "<" => match (&left, &right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Boolean(l < r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l < r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Boolean((*l as f64) < *r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Boolean(*l < (*r as f64))),
            _ => Err(ZekkenError::type_error(
                "Invalid comparison",
                "valid types",
                "invalid types",
                expr.location.line,
                expr.location.column,
            )),
        },
        ">" => match (&left, &right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Boolean(l > r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l > r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Boolean((*l as f64) > *r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Boolean(*l > (*r as f64))),
            _ => Err(ZekkenError::type_error(
                "Invalid comparison",
                "valid types",
                "invalid types",
                expr.location.line,
                expr.location.column,
            )),
        },
        "<=" => match (&left, &right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Boolean(l <= r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l <= r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Boolean((*l as f64) <= *r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Boolean(*l <= (*r as f64))),
            _ => Err(ZekkenError::type_error(
                "Invalid comparison",
                "valid types",
                "invalid types",
                expr.location.line,
                expr.location.column,
            )),
        },
        ">=" => match (&left, &right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Boolean(l >= r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Boolean(l >= r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Boolean((*l as f64) >= *r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Boolean(*l >= (*r as f64))),
            _ => Err(ZekkenError::type_error(
                "Invalid comparison",
                "valid types",
                "invalid types",
                expr.location.line,
                expr.location.column,
            )),
        },
        operator => Err(ZekkenError::runtime(
            &format!("Unknown operator: {}", operator), 
            expr.location.line, 
            expr.location.column,
            None
        ))
    }
}

#[derive(Copy, Clone)]
enum NumValue {
    Int(i64),
    Float(f64),
}

impl NumValue {
    #[inline]
    fn as_f64(self) -> f64 {
        match self {
            NumValue::Int(i) => i as f64,
            NumValue::Float(f) => f,
        }
    }
}

fn try_eval_num_expr(expr: &Expr, env: &Environment) -> Option<NumValue> {
    match expr {
        Expr::IntLit(i) => Some(NumValue::Int(i.value)),
        Expr::FloatLit(f) => Some(NumValue::Float(f.value)),
        Expr::Identifier(id) => {
            if let Some(v) = env.variables.get(&id.name).or_else(|| env.constants.get(&id.name)) {
                return match v {
                    Value::Int(i) => Some(NumValue::Int(*i)),
                    Value::Float(f) => Some(NumValue::Float(*f)),
                    _ => None,
                };
            }
            match env.lookup_ref(&id.name) {
                Some(Value::Int(i)) => Some(NumValue::Int(*i)),
                Some(Value::Float(f)) => Some(NumValue::Float(*f)),
                _ => None,
            }
        }
        Expr::Binary(b) => {
            let l = try_eval_num_expr(&b.left, env)?;
            let r = try_eval_num_expr(&b.right, env)?;
            match b.operator.as_str() {
                "+" => Some(match (l, r) {
                    (NumValue::Int(li), NumValue::Int(ri)) => NumValue::Int(li + ri),
                    _ => NumValue::Float(l.as_f64() + r.as_f64()),
                }),
                "-" => Some(match (l, r) {
                    (NumValue::Int(li), NumValue::Int(ri)) => NumValue::Int(li - ri),
                    _ => NumValue::Float(l.as_f64() - r.as_f64()),
                }),
                "*" => Some(match (l, r) {
                    (NumValue::Int(li), NumValue::Int(ri)) => NumValue::Int(li * ri),
                    _ => NumValue::Float(l.as_f64() * r.as_f64()),
                }),
                "/" => Some(NumValue::Float(l.as_f64() / r.as_f64())),
                "%" => match (l, r) {
                    (NumValue::Int(li), NumValue::Int(ri)) => Some(NumValue::Int(li % ri)),
                    _ => None,
                },
                _ => None,
            }
        }
        _ => None,
    }
}

fn try_eval_numeric_binary(expr: &BinaryExpr, env: &Environment) -> Result<Option<Value>, ZekkenError> {
    let l = match try_eval_num_expr(&expr.left, env) {
        Some(v) => v,
        None => return Ok(None),
    };
    let r = match try_eval_num_expr(&expr.right, env) {
        Some(v) => v,
        None => return Ok(None),
    };

    let out = match expr.operator.as_str() {
        "+" => Some(match (l, r) {
            (NumValue::Int(li), NumValue::Int(ri)) => Value::Int(li + ri),
            _ => Value::Float(l.as_f64() + r.as_f64()),
        }),
        "-" => Some(match (l, r) {
            (NumValue::Int(li), NumValue::Int(ri)) => Value::Int(li - ri),
            _ => Value::Float(l.as_f64() - r.as_f64()),
        }),
        "*" => Some(match (l, r) {
            (NumValue::Int(li), NumValue::Int(ri)) => Value::Int(li * ri),
            _ => Value::Float(l.as_f64() * r.as_f64()),
        }),
        "/" => {
            if r.as_f64() == 0.0 {
                return Err(ZekkenError::runtime(
                    "Division by zero",
                    expr.location.line,
                    expr.location.column,
                    Some("division by zero"),
                ));
            }
            match (l, r) {
                (NumValue::Int(li), NumValue::Int(ri)) => Some(Value::Int(li / ri)),
                _ => Some(Value::Float(l.as_f64() / r.as_f64())),
            }
        }
        "%" => match (l, r) {
            (NumValue::Int(_), NumValue::Int(0)) => {
                return Err(ZekkenError::runtime(
                    "Modulo by zero",
                    expr.location.line,
                    expr.location.column,
                    None,
                ));
            }
            (NumValue::Int(li), NumValue::Int(ri)) => Some(Value::Int(li % ri)),
            _ => None,
        },
        "<" => Some(Value::Boolean(l.as_f64() < r.as_f64())),
        ">" => Some(Value::Boolean(l.as_f64() > r.as_f64())),
        "<=" => Some(Value::Boolean(l.as_f64() <= r.as_f64())),
        ">=" => Some(Value::Boolean(l.as_f64() >= r.as_f64())),
        "==" => Some(Value::Boolean(l.as_f64() == r.as_f64())),
        "!=" => Some(Value::Boolean(l.as_f64() != r.as_f64())),
        _ => None,
    };

    Ok(out)
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
    #[inline]
    fn builtin_requires_at(name: &str) -> bool {
        matches!(name, "println" | "input" | "parse_json" | "queue")
    }

    #[inline]
    fn eval_arg_hot(expr: &Expr, env: &mut Environment) -> Result<Value, ZekkenError> {
        match expr {
            Expr::IntLit(i) => Ok(Value::Int(i.value)),
            Expr::FloatLit(f) => Ok(Value::Float(f.value)),
            Expr::BoolLit(b) => Ok(Value::Boolean(b.value)),
            Expr::Identifier(id) => {
                if let Some(v) = env.variables.get(&id.name).or_else(|| env.constants.get(&id.name)) {
                    return Ok(match v {
                        Value::Int(i) => Value::Int(*i),
                        Value::Float(f) => Value::Float(*f),
                        Value::Boolean(b) => Value::Boolean(*b),
                        _ => v.clone(),
                    });
                }
                evaluate_expression(expr, env)
            }
            _ => evaluate_expression(expr, env),
        }
    }

    #[inline]
    fn eval_call_args(args: &[Box<Expr>], env: &mut Environment) -> Result<Vec<Value>, ZekkenError> {
        match args.len() {
            0 => Ok(Vec::new()),
            1 => Ok(vec![eval_arg_hot(&args[0], env)?]),
            2 => {
                let mut out = Vec::with_capacity(2);
                out.push(eval_arg_hot(&args[0], env)?);
                out.push(eval_arg_hot(&args[1], env)?);
                Ok(out)
            }
            3 => {
                let mut out = Vec::with_capacity(3);
                out.push(eval_arg_hot(&args[0], env)?);
                out.push(eval_arg_hot(&args[1], env)?);
                out.push(eval_arg_hot(&args[2], env)?);
                Ok(out)
            }
            _ => {
                let mut out = Vec::with_capacity(args.len());
                for arg in args {
                    out.push(eval_arg_hot(arg, env)?);
                }
                Ok(out)
            }
        }
    }

    // First check for member expressions (method calls)
    if let Expr::Member(ref member_expr) = *call.callee {
        if let Expr::Identifier(ref object_ident) = *member_expr.object {
            if let Expr::Identifier(ref method_ident) = *member_expr.property {
                if object_ident.name == "math" {
                    if let Some(result) =
                        try_eval_math_call(method_ident.name.as_str(), &call.args, env, call.location.line, call.location.column)
                    {
                        return result;
                    }
                }

                let lib_member_native = if let Some(Value::Object(obj)) = env.lookup_ref(&object_ident.name) {
                    if let Some(Value::NativeFunction(native)) = obj.get(&method_ident.name) {
                        Some(native.clone())
                    } else {
                        None
                    }
                } else {
                    None
                };
                if let Some(native) = lib_member_native {
                    let args = eval_call_args(&call.args, env)?;
                    return match (native)(args) {
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
                    };
                }

                match method_ident.name.as_str() {
                    "push" | "pop" | "shift" | "unshift" | "length" | "first" | "last" => {
                        let method = method_ident.name.as_str();
                        let insert_arg = match method {
                            "push" | "unshift" => {
                                if call.args.len() != 1 {
                                    return Err(ZekkenError::runtime(
                                        if method == "push" {
                                            "push requires exactly one argument"
                                        } else {
                                            "unshift requires exactly one argument"
                                        },
                                        call.location.line,
                                        call.location.column,
                                        None,
                                    ));
                                }
                                Some(evaluate_expression(&call.args[0], env)?)
                            }
                            _ => None,
                        };
                        if let Ok(slot) = env.lookup_mut_assignable(&object_ident.name) {
                            if let Value::Array(arr) = slot {
                                match method {
                                    "push" => {
                                        let v = insert_arg.expect("push arg pre-evaluated");
                                        arr.push(v);
                                        return Ok(Value::Array(arr.clone()));
                                    }
                                    "pop" => {
                                        if !call.args.is_empty() {
                                            return Err(ZekkenError::runtime(
                                                "pop requires no arguments",
                                                call.location.line,
                                                call.location.column,
                                                None,
                                            ));
                                        }
                                        return arr.pop().ok_or_else(|| {
                                            ZekkenError::runtime(
                                                "Array is empty",
                                                call.location.line,
                                                call.location.column,
                                                None,
                                            )
                                        });
                                    }
                                    "shift" => {
                                        if !call.args.is_empty() {
                                            return Err(ZekkenError::runtime(
                                                "shift requires no arguments",
                                                call.location.line,
                                                call.location.column,
                                                None,
                                            ));
                                        }
                                        if arr.is_empty() {
                                            return Err(ZekkenError::runtime(
                                                "Array is empty",
                                                call.location.line,
                                                call.location.column,
                                                None,
                                            ));
                                        }
                                        return Ok(arr.remove(0));
                                    }
                                    "unshift" => {
                                        let v = insert_arg.expect("unshift arg pre-evaluated");
                                        arr.insert(0, v);
                                        return Ok(Value::Array(arr.clone()));
                                    }
                                    "length" => {
                                        if !call.args.is_empty() {
                                            return Err(ZekkenError::runtime(
                                                "length requires no arguments",
                                                call.location.line,
                                                call.location.column,
                                                None,
                                            ));
                                        }
                                        return Ok(Value::Int(arr.len() as i64));
                                    }
                                    "first" => {
                                        if !call.args.is_empty() {
                                            return Err(ZekkenError::runtime(
                                                "first requires no arguments",
                                                call.location.line,
                                                call.location.column,
                                                None,
                                            ));
                                        }
                                        return arr.first().cloned().ok_or_else(|| {
                                            ZekkenError::runtime(
                                                "Array is empty",
                                                call.location.line,
                                                call.location.column,
                                                None,
                                            )
                                        });
                                    }
                                    "last" => {
                                        if !call.args.is_empty() {
                                            return Err(ZekkenError::runtime(
                                                "last requires no arguments",
                                                call.location.line,
                                                call.location.column,
                                                None,
                                            ));
                                        }
                                        return arr.last().cloned().ok_or_else(|| {
                                            ZekkenError::runtime(
                                                "Array is empty",
                                                call.location.line,
                                                call.location.column,
                                                None,
                                            )
                                        });
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        let object = evaluate_expression(&member_expr.object, env)?;
        let method_name = match member_expr.property.as_ref() {
            Expr::Identifier(ident) => ident.name.as_str(),
            _ => return Err(ZekkenError::type_error(
                "Invalid method name",
                "identifier",
                "other",
                call.location.line,
                call.location.column,
            )),
        };

        if method_name == "cast" {
            if call.args.len() != 1 {
                return Err(ZekkenError::runtime(
                    "cast requires one string argument (target type)",
                    call.location.line,
                    call.location.column,
                    None,
                ));
            }

            let target_value = evaluate_expression(&call.args[0], env)?;
            let target = match target_value {
                Value::String(s) => s.trim().to_ascii_lowercase(),
                _ => {
                    return Err(ZekkenError::runtime_with_span(
                        "cast target type must be a string",
                        call.location.line,
                        call.location.column,
                        1,
                        None,
                    ));
                }
            };

            return cast_value(&object, &target).map_err(|msg| {
                let (line, column, span_len) = call
                    .args
                    .first()
                    .map(|arg| {
                        let loc = expr_location(arg);
                        (loc.line, loc.column, expr_span_len(arg))
                    })
                    .unwrap_or((call.location.line, call.location.column, 1));
                ZekkenError::runtime_with_span(&msg, line, column, span_len, None)
            });
        }

        // Call the method on any value type
        let args = eval_call_args(&call.args, env)?;

        // Try to call the method on any value type (strings, arrays, objects, etc)
        match object.call_method(method_name, args, Some(env), if let Expr::Identifier(ref ident) = *member_expr.object {
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

    // When resolving the callee, try identifier dispatch first.
    if let Expr::Identifier(ref ident) = *call.callee {
        let args = eval_call_args(&call.args, env)?;
        if let Some(Value::Function(func_def)) = env.variables.get(&ident.name) {
            return evaluate_function_value_call_with_args(
                func_def,
                args,
                env,
                call.location.line,
                call.location.column,
            );
        }
        if let Some(Value::NativeFunction(native)) = env.variables.get(&ident.name) {
            if builtin_requires_at(&ident.name) && !call.is_native {
                return Err(ZekkenError::runtime(
                    &format!("{} is a built-in; call it with '@{} => |...|'", ident.name, ident.name),
                    call.location.line,
                    call.location.column,
                    None,
                ));
            }
            return evaluate_native_function_value_call_with_args(
                native,
                args,
                call.location.line,
                call.location.column,
            );
        }
        if let Some(Value::Function(func_def)) = env.constants.get(&ident.name) {
            return evaluate_function_value_call_with_args(
                func_def,
                args,
                env,
                call.location.line,
                call.location.column,
            );
        }
        if let Some(Value::NativeFunction(native)) = env.constants.get(&ident.name) {
            if builtin_requires_at(&ident.name) && !call.is_native {
                return Err(ZekkenError::runtime(
                    &format!("{} is a built-in; call it with '@{} => |...|'", ident.name, ident.name),
                    call.location.line,
                    call.location.column,
                    None,
                ));
            }
            return evaluate_native_function_value_call_with_args(
                native,
                args,
                call.location.line,
                call.location.column,
            );
        }

        return match env.lookup_ref(&ident.name) {
            Some(Value::Function(func_def)) => evaluate_function_value_call_with_args(
                func_def,
                args,
                env,
                call.location.line,
                call.location.column,
            ),
            Some(Value::NativeFunction(native)) => {
                if builtin_requires_at(&ident.name) && !call.is_native {
                    return Err(ZekkenError::runtime(
                        &format!("{} is a built-in; call it with '@{} => |...|'", ident.name, ident.name),
                        call.location.line,
                        call.location.column,
                        None,
                    ));
                }
                evaluate_native_function_value_call_with_args(
                    native,
                    args,
                    call.location.line,
                    call.location.column,
                )
            }
            _ => Err(ZekkenError::reference_with_span(
                &format!("Function '{}' not found", &ident.name),
                "function",
                ident.location.line,
                ident.location.column,
                ident.name.chars().count().max(1),
            )),
        };
    }

    // If not a method call or identifier, evaluate as a regular expression
    let callee_val = evaluate_expression(&call.callee, env)?;
    let args = eval_call_args(&call.args, env)?;

    match callee_val {
        Value::Function(func_def) => evaluate_function_value_call_with_args(
            &func_def,
            args,
            env,
            call.location.line,
            call.location.column,
        ),
        Value::NativeFunction(native) => {
            evaluate_native_function_value_call_with_args(&native, args, call.location.line, call.location.column)
        }
        _ => Err(ZekkenError::type_error(
            "Cannot call non-function value",
            "function",
            "non-function",
            call.location.line,
            call.location.column,
        )),
    }
}

fn cast_value(value: &Value, target: &str) -> Result<Value, String> {
    fn value_type_name_local(v: &Value) -> &'static str {
        match v {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Boolean(_) => "bool",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::Function(_) => "function",
            Value::NativeFunction(_) => "native function",
            Value::Complex { .. } => "complex",
            Value::Vector(_) => "vector",
            Value::Matrix(_) => "matrix",
            Value::Void => "void",
        }
    }

    match target {
        "string" => Ok(Value::String(value.to_string())),
        "str" => Err("Unsupported cast target 'str'. Use 'string'.".to_string()),
        "int" => match value {
            Value::Int(i) => Ok(Value::Int(*i)),
            Value::Float(f) => Ok(Value::Int(*f as i64)),
            Value::Boolean(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
            Value::String(s) => s
                .trim()
                .parse::<i64>()
                .map(Value::Int)
                .map_err(|_| format!("Cannot cast string '{}' to int", s)),
            _ => Err(format!("Cannot cast type '{}' to int", value_type_name_local(value))),
        },
        "float" => match value {
            Value::Float(f) => Ok(Value::Float(*f)),
            Value::Int(i) => Ok(Value::Float(*i as f64)),
            Value::Boolean(b) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
            Value::String(s) => s
                .trim()
                .parse::<f64>()
                .map(Value::Float)
                .map_err(|_| format!("Cannot cast string '{}' to float", s)),
            _ => Err(format!("Cannot cast type '{}' to float", value_type_name_local(value))),
        },
        "bool" => match value {
            Value::Boolean(b) => Ok(Value::Boolean(*b)),
            Value::Int(i) => Ok(Value::Boolean(*i != 0)),
            Value::Float(f) => Ok(Value::Boolean(*f != 0.0)),
            Value::String(s) => {
                let lower = s.trim().to_ascii_lowercase();
                match lower.as_str() {
                    "true" | "1" => Ok(Value::Boolean(true)),
                    "false" | "0" => Ok(Value::Boolean(false)),
                        _ => Err(format!("Cannot cast string '{}' to bool", s)),
                    }
                }
            _ => Err(format!("Cannot cast type '{}' to bool", value_type_name_local(value))),
        },
        _ => Err(format!("Unsupported cast target '{}'", target)),
    }
}

fn try_eval_math_call(
    method: &str,
    args: &[Box<Expr>],
    env: &mut Environment,
    line: usize,
    column: usize,
) -> Option<Result<Value, ZekkenError>> {
    #[inline]
    fn as_num(v: Value, line: usize, column: usize) -> Result<f64, ZekkenError> {
        match v {
            Value::Int(i) => Ok(i as f64),
            Value::Float(f) => Ok(f),
            _ => Err(ZekkenError::type_error("Expected number", "number", "other", line, column)),
        }
    }

    match method {
        "sin" | "cos" | "tan" | "sqrt" | "abs" => Some((|| -> Result<Value, ZekkenError> {
            if args.len() != 1 {
                return Err(ZekkenError::runtime(
                    "Expected 1 argument",
                    line,
                    column,
                    Some("argument mismatch"),
                ));
            }
            let n = as_num(evaluate_expression(&args[0], env)?, line, column)?;
            Ok(Value::Float(match method {
                "sin" => n.sin(),
                "cos" => n.cos(),
                "tan" => n.tan(),
                "sqrt" => n.sqrt(),
                _ => n.abs(),
            }))
        })()),
        "pow" => Some((|| -> Result<Value, ZekkenError> {
            if args.len() != 2 {
                return Err(ZekkenError::runtime(
                    "Expected 2 arguments",
                    line,
                    column,
                    Some("argument mismatch"),
                ));
            }
            let l = as_num(evaluate_expression(&args[0], env)?, line, column)?;
            let r = as_num(evaluate_expression(&args[1], env)?, line, column)?;
            Ok(Value::Float(l.powf(r)))
        })()),
        "log" => Some((|| -> Result<Value, ZekkenError> {
            if args.is_empty() || args.len() > 2 {
                return Err(ZekkenError::runtime(
                    "Expected 1 or 2 arguments",
                    line,
                    column,
                    Some("argument mismatch"),
                ));
            }
            let n = as_num(evaluate_expression(&args[0], env)?, line, column)?;
            if args.len() == 2 {
                let base = as_num(evaluate_expression(&args[1], env)?, line, column)?;
                Ok(Value::Float(n.log(base)))
            } else {
                Ok(Value::Float(n.ln()))
            }
        })()),
        "exp" | "floor" | "ceil" | "round" => Some((|| -> Result<Value, ZekkenError> {
            if args.len() != 1 {
                return Err(ZekkenError::runtime(
                    "Expected 1 argument",
                    line,
                    column,
                    Some("argument mismatch"),
                ));
            }
            let n = as_num(evaluate_expression(&args[0], env)?, line, column)?;
            Ok(Value::Float(match method {
                "exp" => n.exp(),
                "floor" => n.floor(),
                "ceil" => n.ceil(),
                _ => n.round(),
            }))
        })()),
        "min" | "max" => Some((|| -> Result<Value, ZekkenError> {
            if args.len() != 2 {
                return Err(ZekkenError::runtime(
                    "Expected 2 arguments",
                    line,
                    column,
                    Some("argument mismatch"),
                ));
            }
            let l = as_num(evaluate_expression(&args[0], env)?, line, column)?;
            let r = as_num(evaluate_expression(&args[1], env)?, line, column)?;
            Ok(Value::Float(if method == "min" { l.min(r) } else { l.max(r) }))
        })()),
        "clamp" => Some((|| -> Result<Value, ZekkenError> {
            if args.len() != 3 {
                return Err(ZekkenError::runtime(
                    "Expected 3 arguments",
                    line,
                    column,
                    Some("argument mismatch"),
                ));
            }
            let x = as_num(evaluate_expression(&args[0], env)?, line, column)?;
            let min = as_num(evaluate_expression(&args[1], env)?, line, column)?;
            let max = as_num(evaluate_expression(&args[2], env)?, line, column)?;
            Ok(Value::Float(x.max(min).min(max)))
        })()),
        "atan2" => Some((|| -> Result<Value, ZekkenError> {
            if args.len() != 2 {
                return Err(ZekkenError::runtime(
                    "Expected 2 arguments",
                    line,
                    column,
                    Some("argument mismatch"),
                ));
            }
            let y = as_num(evaluate_expression(&args[0], env)?, line, column)?;
            let x = as_num(evaluate_expression(&args[1], env)?, line, column)?;
            Ok(Value::Float(y.atan2(x)))
        })()),
        _ => None,
    }
}

fn evaluate_function_value_call_with_args(
    func_def: &FunctionValue,
    args: Vec<Value>,
    env: &Environment,
    line: usize,
    column: usize,
) -> Result<Value, ZekkenError> {
    if args.len() > func_def.params.len() {
        return Err(ZekkenError::runtime(
            &format!("Expected {} arguments but got {}", func_def.params.len(), args.len()),
            line,
            column,
            Some("argument mismatch"),
        ));
    }

    let mut function_env = if func_def.needs_parent {
        Environment::new_with_parent_capacity(env.clone(), func_def.params.len())
    } else {
        Environment::take_pooled_scope(func_def.params.len())
    };

    // If we didn't clone the parent environment, make sure the function body can still
    // resolve globals/built-ins (and maintain expected lexical behavior).
    //
    // Note: This prevents returning the env to the pool (parent != None), but fixes
    // correctness issues where function bodies can't see global names.
    if !func_def.needs_parent {
        function_env.parent = Some(Box::new(env.clone()));
    }

    if !func_def.needs_parent && !func_def.captures.is_empty() {
        for name in func_def.captures.iter() {
            if let Some(val) = env.lookup_ref(name) {
                function_env.declare_ref(name, val.clone(), false);
            }
        }
    }

    let bind_and_execute = || -> Result<Value, ZekkenError> {
        let provided = args;

        // Bind provided args first, then fill missing params from defaults.
        for (idx, param) in func_def.params.iter().enumerate() {
            let value = if let Some(arg) = provided.get(idx) {
                arg.clone()
            } else if let Some(default_expr) = param.default_value.as_ref() {
                evaluate_expression(default_expr, &mut function_env)?
            } else {
                return Err(ZekkenError::runtime(
                    &format!("Missing required argument '{}'", param.ident),
                    line,
                    column,
                    Some("argument mismatch"),
                ));
            };
            if !check_value_type(&value, &param.type_) {
                return Err(ZekkenError::type_error(
                    &format!("Type mismatch for parameter '{}'", param.ident),
                    &format!("{:?}", param.type_),
                    value_type_name(&value),
                    line,
                    column,
                ));
            }
            function_env.declare_ref_typed(param.ident.as_str(), value, param.type_, false);
        }

        let result = bytecode::execute_contents(func_def.body.as_ref(), &mut function_env)?;
        Ok(result.unwrap_or(Value::Void))
    };

    let out = bind_and_execute().and_then(|v| {
        if let Some(ret_ty) = func_def.return_type {
            if !check_value_type(&v, &ret_ty) {
                return Err(ZekkenError::type_error(
                    "Type mismatch in function return value",
                    &format!("{:?}", ret_ty),
                    value_type_name(&v),
                    line,
                    column,
                ));
            }
        }
        Ok(v)
    });
    if !func_def.needs_parent {
        Environment::return_pooled_scope(function_env);
    }
    out
}

fn evaluate_native_function_value_call_with_args(
    native: &Arc<dyn Fn(Vec<Value>) -> Result<Value, String> + Send + Sync + 'static>,
    args: Vec<Value>,
    line: usize,
    column: usize,
) -> Result<Value, ZekkenError> {
    match (native)(args) {
        Ok(val) => Ok(val),
        Err(s) => {
            Err(ZekkenError::runtime_with_span(&s, line, column, 1, None))
        }
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
    if let Some(value) = evaluate_member_expression_chain(member, env)? {
        return Ok(value);
    }

    let object = evaluate_expression(&member.object, env)?;
    let result = match &*member.property {
        Expr::Identifier(ref ident) => {
            if member.is_method {
                // Bracket indexing: obj[expr] / arr[expr]
                //
                // For identifier keys inside brackets, prefer runtime lookup:
                // - int/float-as-int => numeric indexing (arrays, and objects via __keys__)
                // - string => object property lookup by that string
                // - otherwise fall back to literal property name (obj[foo] -> "foo")
                if let Some(v) = env.lookup_ref(&ident.name) {
                    match v {
                        Value::Int(i) if *i >= 0 => {
                            evaluate_index_access(&object, *i as usize, member.location.line, member.location.column)
                        }
                        Value::Float(f) if *f >= 0.0 && f.fract() == 0.0 => {
                            evaluate_index_access(&object, *f as usize, member.location.line, member.location.column)
                        }
                        Value::String(s) => {
                            evaluate_property_access(&object, s, member.location.line, member.location.column)
                        }
                        _ => evaluate_property_access(&object, &ident.name, member.location.line, member.location.column),
                    }
                } else {
                    evaluate_property_access(&object, &ident.name, member.location.line, member.location.column)
                }
            } else {
                // Dot access: obj.key
                evaluate_property_access(&object, &ident.name, member.location.line, member.location.column)
            }
        }
        Expr::StringLit(ref lit) => evaluate_property_access(&object, &lit.value, member.location.line, member.location.column),
        Expr::IntLit(ref lit) => evaluate_index_access(&object, lit.value as usize, member.location.line, member.location.column),
        _ => {
            if member.is_method {
                // Bracket indexing supports dynamic expressions.
                match evaluate_expression(&member.property, env)? {
                    Value::Int(i) if i >= 0 => {
                        evaluate_index_access(&object, i as usize, member.location.line, member.location.column)
                    }
                    Value::Float(f) if f >= 0.0 && f.fract() == 0.0 => {
                        evaluate_index_access(&object, f as usize, member.location.line, member.location.column)
                    }
                    Value::String(s) => {
                        evaluate_property_access(&object, &s, member.location.line, member.location.column)
                    }
                    _ => Err(ZekkenError::type_error(
                        "Invalid property access",
                        "string/int",
                        "other",
                        member.location.line,
                        member.location.column,
                    )),
                }
            } else {
                Err(ZekkenError::type_error(
                    "Invalid property access",
                    "identifier",
                    "other",
                    member.location.line,
                    member.location.column,
                ))
            }
        }
    }?;
    Ok(result)
}

fn evaluate_member_expression_chain(member: &MemberExpr, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    fn collect_chain<'a>(expr: &'a Expr, out: &mut Vec<(&'a Expr, bool)>) -> Option<&'a Identifier> {
        match expr {
            Expr::Member(m) => {
                let root = collect_chain(m.object.as_ref(), out)?;
                out.push((m.property.as_ref(), m.is_method));
                Some(root)
            }
            Expr::Identifier(id) => Some(id),
            _ => None,
        }
    }

    let mut chain: Vec<(&Expr, bool)> = Vec::new();
    let root_ident = match collect_chain(member.object.as_ref(), &mut chain) {
        Some(id) => id,
        None => return Ok(None),
    };
    chain.push((member.property.as_ref(), member.is_method));

    let supports_fast_chain = chain.iter().all(|(prop, _)| {
        matches!(
            prop,
            Expr::IntLit(_) | Expr::FloatLit(_) | Expr::Identifier(_) | Expr::StringLit(_)
        )
    });
    if !supports_fast_chain {
        return Ok(None);
    }

    let mut current = env.lookup_ref(&root_ident.name).ok_or_else(|| {
        ZekkenError::reference(
            &format!("Variable '{}' not found", root_ident.name),
            "variable",
            root_ident.location.line,
            root_ident.location.column,
        )
    })?;

    for (prop, computed) in chain {
        current = match current {
            Value::Array(arr) => {
                let idx = match prop {
                    Expr::IntLit(lit) if lit.value >= 0 => Some(lit.value as usize),
                    Expr::FloatLit(lit) if lit.value >= 0.0 && lit.value.fract() == 0.0 => Some(lit.value as usize),
                    Expr::Identifier(ident) => {
                        if computed {
                            match env.lookup_ref(&ident.name) {
                                Some(Value::Int(i)) if *i >= 0 => Some(*i as usize),
                                Some(Value::Float(f)) if *f >= 0.0 && f.fract() == 0.0 => Some(*f as usize),
                                _ => None,
                            }
                        } else {
                            None
                        }
                    }
                    _ => None,
                };

                if let Some(i) = idx {
                    arr.get(i).ok_or_else(|| {
                        ZekkenError::runtime(
                            &format!("Array index {} out of bounds", i),
                            member.location.line,
                            member.location.column,
                            None,
                        )
                    })?
                } else {
                    return Err(ZekkenError::type_error(
                        "Invalid member access",
                        "object",
                        "other",
                        member.location.line,
                        member.location.column,
                    ));
                }
            }
            Value::Object(map) => match prop {
                Expr::Identifier(ident) => {
                    if computed {
                        // Bracket indexing: allow obj[a] where `a` is an int/float-as-int/string at runtime.
                        if let Some(v) = env.lookup_ref(&ident.name) {
                            match v {
                                Value::String(s) => map.get(s).ok_or_else(|| {
                                    ZekkenError::reference(
                                        &format!("Property '{}' not found", s),
                                        s,
                                        member.location.line,
                                        member.location.column,
                                    )
                                })?,
                                Value::Int(i) if *i >= 0 => {
                                    let idx = *i as usize;
                                    let key = match map.get("__keys__") {
                                        Some(Value::Array(keys)) => match keys.get(idx) {
                                            Some(Value::String(key)) => key,
                                            _ => {
                                                return Err(ZekkenError::runtime(
                                                    &format!("Object index {} out of bounds", idx),
                                                    member.location.line,
                                                    member.location.column,
                                                    None,
                                                ));
                                            }
                                        },
                                        _ => {
                                            return Err(ZekkenError::runtime(
                                                "Object does not support numeric indexing",
                                                member.location.line,
                                                member.location.column,
                                                None,
                                            ));
                                        }
                                    };
                                    map.get(key).ok_or_else(|| {
                                        ZekkenError::reference(
                                            &format!("Property '{}' not found", key),
                                            key,
                                            member.location.line,
                                            member.location.column,
                                        )
                                    })?
                                }
                                Value::Float(f) if *f >= 0.0 && f.fract() == 0.0 => {
                                    let idx = *f as usize;
                                    let key = match map.get("__keys__") {
                                        Some(Value::Array(keys)) => match keys.get(idx) {
                                            Some(Value::String(key)) => key,
                                            _ => {
                                                return Err(ZekkenError::runtime(
                                                    &format!("Object index {} out of bounds", idx),
                                                    member.location.line,
                                                    member.location.column,
                                                    None,
                                                ));
                                            }
                                        },
                                        _ => {
                                            return Err(ZekkenError::runtime(
                                                "Object does not support numeric indexing",
                                                member.location.line,
                                                member.location.column,
                                                None,
                                            ));
                                        }
                                    };
                                    map.get(key).ok_or_else(|| {
                                        ZekkenError::reference(
                                            &format!("Property '{}' not found", key),
                                            key,
                                            member.location.line,
                                            member.location.column,
                                        )
                                    })?
                                }
                                _ => map.get(&ident.name).ok_or_else(|| {
                                    ZekkenError::reference(
                                        &format!("Property '{}' not found", ident.name),
                                        &ident.name,
                                        member.location.line,
                                        member.location.column,
                                    )
                                })?,
                            }
                        } else {
                            map.get(&ident.name).ok_or_else(|| {
                                ZekkenError::reference(
                                    &format!("Property '{}' not found", ident.name),
                                    &ident.name,
                                    member.location.line,
                                    member.location.column,
                                )
                            })?
                        }
                    } else {
                        map.get(&ident.name).ok_or_else(|| {
                            ZekkenError::reference(
                                &format!("Property '{}' not found", ident.name),
                                &ident.name,
                                member.location.line,
                                member.location.column,
                            )
                        })?
                    }
                }
                Expr::StringLit(lit) => map.get(&lit.value).ok_or_else(|| {
                    ZekkenError::reference(
                        &format!("Property '{}' not found", lit.value),
                        &lit.value,
                        member.location.line,
                        member.location.column,
                    )
                })?,
                Expr::IntLit(lit) if lit.value >= 0 => {
                    let idx = lit.value as usize;
                    let key = match map.get("__keys__") {
                        Some(Value::Array(keys)) => match keys.get(idx) {
                            Some(Value::String(key)) => key,
                            _ => {
                                return Err(ZekkenError::runtime(
                                    &format!("Object index {} out of bounds", idx),
                                    member.location.line,
                                    member.location.column,
                                    None,
                                ));
                            }
                        },
                        _ => {
                            return Err(ZekkenError::runtime(
                                "Object does not support numeric indexing",
                                member.location.line,
                                member.location.column,
                                None,
                            ));
                        }
                    };
                    map.get(key).ok_or_else(|| {
                        ZekkenError::reference(
                            &format!("Property '{}' not found", key),
                            key,
                            member.location.line,
                            member.location.column,
                        )
                    })?
                }
                _ => {
                    return Err(ZekkenError::type_error(
                        "Invalid property access",
                        "string/int/identifier",
                        "other",
                        member.location.line,
                        member.location.column,
                    ));
                }
            },
            _ => {
                return Err(ZekkenError::type_error(
                    "Invalid member access",
                    "object/array",
                    "other",
                    member.location.line,
                    member.location.column,
                ));
            }
        };
    }

    Ok(Some(current.clone()))
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
            if let Some(v) = arr.get(idx) {
                return match v {
                    Value::Int(i) => Ok(Value::Int(*i)),
                    Value::Float(f) => Ok(Value::Float(*f)),
                    Value::Boolean(b) => Ok(Value::Boolean(*b)),
                    _ => Ok(v.clone()),
                };
            }
            Err(ZekkenError::runtime(
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

pub fn evaluate_assignment_discard(assign: &AssignExpr, env: &mut Environment) -> Result<(), ZekkenError> {
    let _ = evaluate_assignment_internal(assign, env, false)?;
    Ok(())
}

fn evaluate_assignment(assign: &AssignExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    evaluate_assignment_internal(assign, env, true)
}

fn evaluate_assignment_internal(assign: &AssignExpr, env: &mut Environment, want_result: bool) -> Result<Value, ZekkenError> {
    enum AssignTarget<'a> {
        Identifier(&'a str),
        Member(&'a Expr),
    }

    let target = match assign.left.as_ref() {
        Expr::Identifier(ident) => AssignTarget::Identifier(&ident.name),
        Expr::Member(_) => AssignTarget::Member(assign.left.as_ref()),
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

    // Fast path: in-place assignment for identifiers in tight loops.
    if let AssignTarget::Identifier(name) = target {
        if assign.operator == "=" {
            let right_val = evaluate_expression(&assign.right, env)?;
            let expected = env.lookup_type(name).unwrap_or(DataType::Any);
            if expected != DataType::Any && !check_value_type(&right_val, &expected) {
                let loc = expr_location(&assign.right);
                return Err(ZekkenError::type_error(
                    &format!("Type mismatch in assignment to '{}'", name),
                    &format!("{:?}", expected),
                    value_type_name(&right_val),
                    loc.line,
                    loc.column,
                ));
            }
            if let Ok(slot) = env.lookup_mut_assignable(name) {
                return match right_val {
                    Value::Int(i) => {
                        *slot = Value::Int(i);
                        Ok(if want_result { Value::Int(i) } else { Value::Void })
                    }
                    Value::Float(f) => {
                        *slot = Value::Float(f);
                        Ok(if want_result { Value::Float(f) } else { Value::Void })
                    }
                    Value::Boolean(b) => {
                        *slot = Value::Boolean(b);
                        Ok(if want_result { Value::Boolean(b) } else { Value::Void })
                    }
                    value => {
                        if want_result {
                            *slot = value.clone();
                            Ok(value)
                        } else {
                            *slot = value;
                            Ok(Value::Void)
                        }
                    }
                };
            }
            // Fall through to generic path for not-found/other errors.
        }

        // Fast path: in-place compound assignment for identifiers to avoid cloning
        // large values (especially arrays) in tight loops.
        if assign.operator != "=" {
            let right_val = evaluate_expression(&assign.right, env)?;
            let expected = env.lookup_type(name).unwrap_or(DataType::Any);
            let loc = expr_location(&assign.right);
            if let Ok(left_slot) = env.lookup_mut_assignable(name) {
                match assign.operator.as_str() {
                    "+=" => match left_slot {
                        Value::Int(l) => match &right_val {
                            Value::Int(r) => return Ok(Value::Int({ *l += *r; *l })),
                            Value::Float(r) => {
                                if expected != DataType::Any && expected != DataType::Float {
                                    return Err(ZekkenError::type_error(
                                        &format!("Type mismatch in assignment to '{}'", name),
                                        &format!("{:?}", expected),
                                        "float",
                                        loc.line,
                                        loc.column,
                                    ));
                                }
                                let v = *l as f64 + *r;
                                *left_slot = Value::Float(v);
                                return Ok(if want_result { Value::Float(v) } else { Value::Void });
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
                                return Ok(if want_result { Value::String(l.clone()) } else { Value::Void });
                            }
                            other => {
                                l.push_str(&other.to_string());
                                return Ok(if want_result { Value::String(l.clone()) } else { Value::Void });
                            }
                        },
                        Value::Array(l) => {
                            if let Value::Array(r) = &right_val {
                                l.extend(r.iter().cloned());
                                return Ok(if want_result { Value::Array(l.clone()) } else { Value::Void });
                            }
                        }
                        _ => {}
                    },
                    "-=" => match left_slot {
                        Value::Int(l) => match &right_val {
                            Value::Int(r) => return Ok(Value::Int({ *l -= *r; *l })),
                            Value::Float(r) => {
                                if expected != DataType::Any && expected != DataType::Float {
                                    return Err(ZekkenError::type_error(
                                        &format!("Type mismatch in assignment to '{}'", name),
                                        &format!("{:?}", expected),
                                        "float",
                                        loc.line,
                                        loc.column,
                                    ));
                                }
                                let v = *l as f64 - *r;
                                *left_slot = Value::Float(v);
                                return Ok(if want_result { Value::Float(v) } else { Value::Void });
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
                                if expected != DataType::Any && expected != DataType::Float {
                                    return Err(ZekkenError::type_error(
                                        &format!("Type mismatch in assignment to '{}'", name),
                                        &format!("{:?}", expected),
                                        "float",
                                        loc.line,
                                        loc.column,
                                    ));
                                }
                                let v = *l as f64 * *r;
                                *left_slot = Value::Float(v);
                                return Ok(if want_result { Value::Float(v) } else { Value::Void });
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
                                if expected != DataType::Any && expected != DataType::Float {
                                    return Err(ZekkenError::type_error(
                                        &format!("Type mismatch in assignment to '{}'", name),
                                        &format!("{:?}", expected),
                                        "float",
                                        loc.line,
                                        loc.column,
                                    ));
                                }
                                let v = *l as f64 / *r;
                                *left_slot = Value::Float(v);
                                return Ok(if want_result { Value::Float(v) } else { Value::Void });
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
                                return Ok(if want_result { Value::Int(*l) } else { Value::Void });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let value_to_store = if assign.operator != "=" {
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

    match (target, value_to_store) {
        (AssignTarget::Identifier(name), Value::Int(i)) => {
            env.assign(name, Value::Int(i)).map_err(|err| {
                ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)
            })?;
            Ok(if want_result { Value::Int(i) } else { Value::Void })
        }
        (AssignTarget::Identifier(name), Value::Float(f)) => {
            env.assign(name, Value::Float(f)).map_err(|err| {
                ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)
            })?;
            Ok(if want_result { Value::Float(f) } else { Value::Void })
        }
        (AssignTarget::Identifier(name), Value::Boolean(b)) => {
            env.assign(name, Value::Boolean(b)).map_err(|err| {
                ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)
            })?;
            Ok(if want_result { Value::Boolean(b) } else { Value::Void })
        }
        (AssignTarget::Identifier(name), value) => {
            if want_result {
                env.assign(name, value.clone()).map_err(|err| {
                    ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)
                })?;
                Ok(value)
            } else {
                env.assign(name, value).map_err(|err| {
                    ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)
                })?;
                Ok(Value::Void)
            }
        }
        (AssignTarget::Member(member_expr), Value::Int(i)) => {
            assign_to_member(member_expr, Value::Int(i), env).map_err(|err| {
                ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)
            })?;
            Ok(if want_result { Value::Int(i) } else { Value::Void })
        }
        (AssignTarget::Member(member_expr), Value::Float(f)) => {
            assign_to_member(member_expr, Value::Float(f), env).map_err(|err| {
                ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)
            })?;
            Ok(if want_result { Value::Float(f) } else { Value::Void })
        }
        (AssignTarget::Member(member_expr), Value::Boolean(b)) => {
            assign_to_member(member_expr, Value::Boolean(b), env).map_err(|err| {
                ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)
            })?;
            Ok(if want_result { Value::Boolean(b) } else { Value::Void })
        }
        (AssignTarget::Member(member_expr), value) => {
            if want_result {
                assign_to_member(member_expr, value.clone(), env).map_err(|err| {
                    ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)
                })?;
                Ok(value)
            } else {
                assign_to_member(member_expr, value, env).map_err(|err| {
                    ZekkenError::runtime(&err, assign.location.line, assign.location.column, None)
                })?;
                Ok(Value::Void)
            }
        }
    }
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
