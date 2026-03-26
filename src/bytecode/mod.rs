use crate::ast::*;
use crate::environment::{Environment, FunctionValue, Value};
use crate::errors::{push_error, ZekkenError};
use crate::libraries::load_library;
use crate::parser::Parser;
use hashbrown::HashMap;
use std::sync::Arc;
use std::path::Path;

mod inst;
mod compiler;
mod runtime;
mod libraries;

use compiler::Compiler;
use compiler::analyze_function_parent_usage;
use runtime::{run_insts, check_value_type, clone_value_hot, compare_values, value_type_name};

fn eval_binary(left: &Value, right: &Value, op: &str, location: &Location) -> Result<Value, ZekkenError> {
    #[inline]
    fn cmp_num<F: FnOnce(f64, f64) -> bool>(left: &Value, right: &Value, location: &Location, cmp: F) -> Result<Value, ZekkenError> {
        let l = match left {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err(ZekkenError::type_error("Invalid comparison", "number", value_type_name(left), location.line, location.column)),
        };
        let r = match right {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err(ZekkenError::type_error("Invalid comparison", "number", value_type_name(right), location.line, location.column)),
        };
        Ok(Value::Boolean(cmp(l, r)))
    }

    match op {
        "+" => match (left, right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l + r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l + r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 + r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l + *r as f64)),
            (Value::String(l), Value::String(r)) => Ok(Value::String(format!("{}{}", l, r))),
            (Value::String(l), other) => Ok(Value::String(format!("{}{}", l, other))),
            (other, Value::String(r)) => Ok(Value::String(format!("{}{}", other, r))),
            (Value::Array(l), Value::Array(r)) => {
                let mut out = Vec::with_capacity(l.len() + r.len());
                out.extend(l.iter().cloned());
                out.extend(r.iter().cloned());
                Ok(Value::Array(out))
            }
            _ => Err(ZekkenError::type_error(
                "Invalid operand types for addition",
                "compatible numbers/strings/arrays",
                "incompatible operands",
                location.line,
                location.column,
            )),
        },
        "-" => match (left, right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l - r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 - r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l - *r as f64)),
            _ => Err(ZekkenError::type_error("Invalid operand types for subtraction", "number", "non-number", location.line, location.column)),
        },
        "*" => match (left, right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l * r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 * r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l * *r as f64)),
            _ => Err(ZekkenError::type_error("Invalid operand types for multiplication", "number", "non-number", location.line, location.column)),
        },
        "/" => match (left, right) {
            (Value::Int(_), Value::Int(r)) if *r == 0 => Err(ZekkenError::runtime("Division by zero", location.line, location.column, Some("division by zero"))),
            (Value::Float(_), Value::Float(r)) if *r == 0.0 => Err(ZekkenError::runtime("Division by zero", location.line, location.column, Some("division by zero"))),
            (Value::Int(_), Value::Float(r)) if *r == 0.0 => Err(ZekkenError::runtime("Division by zero", location.line, location.column, Some("division by zero"))),
            (Value::Float(_), Value::Int(r)) if *r == 0 => Err(ZekkenError::runtime("Division by zero", location.line, location.column, Some("division by zero"))),
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l / r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l / r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 / r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l / *r as f64)),
            _ => Err(ZekkenError::type_error("Invalid operand types for division", "number", "non-number", location.line, location.column)),
        },
        "%" => match (left, right) {
            (Value::Int(_), Value::Int(r)) if *r == 0 => Err(ZekkenError::runtime("Modulo by zero", location.line, location.column, Some("modulo by zero"))),
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l % r)),
            _ => Err(ZekkenError::type_error("Invalid operand types for modulo", "int", "non-int", location.line, location.column)),
        },
        "==" => Ok(Value::Boolean(compare_values(left, right))),
        "!=" => Ok(Value::Boolean(!compare_values(left, right))),
        "<" => cmp_num(left, right, location, |l, r| l < r),
        ">" => cmp_num(left, right, location, |l, r| l > r),
        "<=" => cmp_num(left, right, location, |l, r| l <= r),
        ">=" => cmp_num(left, right, location, |l, r| l >= r),
        _ => Err(ZekkenError::runtime(&format!("Unknown operator: {}", op), location.line, location.column, None)),
    }
}
#[derive(Clone)]
enum MemberKey {
    Prop(String),
    Index(usize),
}

fn resolve_member_key(expr: &Expr, env: &Environment) -> Result<MemberKey, String> {
    match expr {
        Expr::Identifier(id) => {
            if let Some(v) = env.lookup_ref(&id.name) {
                match v {
                    Value::Int(i) if *i >= 0 => Ok(MemberKey::Index(*i as usize)),
                    Value::Float(f) if *f >= 0.0 && f.fract() == 0.0 => Ok(MemberKey::Index(*f as usize)),
                    _ => Ok(MemberKey::Prop(id.name.clone())),
                }
            } else {
                Ok(MemberKey::Prop(id.name.clone()))
            }
        }
        Expr::StringLit(s) => Ok(MemberKey::Prop(s.value.clone())),
        Expr::IntLit(i) if i.value >= 0 => Ok(MemberKey::Index(i.value as usize)),
        Expr::FloatLit(f) if f.value >= 0.0 && f.value.fract() == 0.0 => Ok(MemberKey::Index(f.value as usize)),
        _ => Err("Invalid member key".to_string()),
    }
}

fn collect_member_path(expr: &Expr, env: &Environment) -> Result<(String, Vec<MemberKey>), String> {
    match expr {
        Expr::Identifier(id) => Ok((id.name.clone(), Vec::new())),
        Expr::Member(m) => {
            let (root, mut path) = collect_member_path(&m.object, env)?;
            path.push(resolve_member_key(&m.property, env)?);
            Ok((root, path))
        }
        _ => Err("Invalid assignment target".to_string()),
    }
}

fn get_at_path(current: &Value, path: &[MemberKey]) -> Result<Value, String> {
    if path.is_empty() {
        return Ok(current.clone());
    }
    match (&path[0], current) {
        (MemberKey::Index(i), Value::Array(arr)) => {
            let next = arr.get(*i).ok_or_else(|| format!("Array index {} out of bounds", i))?;
            get_at_path(next, &path[1..])
        }
        (MemberKey::Prop(p), Value::Object(map)) => {
            let next = map.get(p).ok_or_else(|| format!("Property '{}' not found", p))?;
            get_at_path(next, &path[1..])
        }
        (MemberKey::Index(i), Value::Object(map)) => {
            let key = match map.get("__keys__") {
                Some(Value::Array(keys)) => match keys.get(*i) {
                    Some(Value::String(k)) => k,
                    _ => return Err(format!("Object index {} out of bounds", i)),
                },
                _ => return Err("Object does not support numeric indexing".to_string()),
            };
            let next = map.get(key).ok_or_else(|| format!("Property '{}' not found", key))?;
            get_at_path(next, &path[1..])
        }
        _ => Err("Invalid member access".to_string()),
    }
}

fn assign_at_path(current: &mut Value, path: &[MemberKey], value: Value) -> Result<(), String> {
    if path.is_empty() {
        *current = value;
        return Ok(());
    }

    match (&path[0], current) {
        (MemberKey::Index(i), Value::Array(arr)) => {
            if *i >= arr.len() {
                return Err(format!("Array index {} out of bounds", i));
            }
            if path.len() == 1 {
                arr[*i] = value;
                return Ok(());
            }
            assign_at_path(&mut arr[*i], &path[1..], value)
        }
        (MemberKey::Prop(p), Value::Object(map)) => {
            if path.len() == 1 {
                map.insert(p.clone(), value);
                return Ok(());
            }
            let next = map.get_mut(p).ok_or_else(|| format!("Property '{}' not found", p))?;
            assign_at_path(next, &path[1..], value)
        }
        (MemberKey::Index(i), Value::Object(map)) => {
            let key = match map.get("__keys__") {
                Some(Value::Array(keys)) => match keys.get(*i) {
                    Some(Value::String(k)) => k.clone(),
                    _ => return Err(format!("Object index {} out of bounds", i)),
                },
                _ => return Err("Object does not support numeric indexing".to_string()),
            };
            if path.len() == 1 {
                map.insert(key, value);
                return Ok(());
            }
            let next = map.get_mut(&key).ok_or_else(|| format!("Property '{}' not found", key))?;
            assign_at_path(next, &path[1..], value)
        }
        _ => Err("Invalid member assignment target".to_string()),
    }
}

fn eval_assignment_native(assign: &AssignExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    let right = eval_expr_native(&assign.right, env)?;
    let base_op = match assign.operator.as_str() {
        "+=" => Some("+"),
        "-=" => Some("-"),
        "*=" => Some("*"),
        "/=" => Some("/"),
        "%=" => Some("%"),
        "=" => None,
        _ => {
            return Err(ZekkenError::runtime(
                &format!("Unsupported assignment operator '{}'", assign.operator),
                assign.location.line,
                assign.location.column,
                None,
            ))
        }
    };

    match assign.left.as_ref() {
        Expr::Identifier(id) => {
            // Hot path: in-place compound assignment avoids cloning full arrays/strings.
            if assign.operator != "=" {
                if let Ok(slot) = env.lookup_mut_assignable(&id.name) {
                    match assign.operator.as_str() {
                        "+=" => match slot {
                            Value::Int(l) => match &right {
                                Value::Int(r) => {
                                    *l += *r;
                                    return Ok(Value::Int(*l));
                                }
                                Value::Float(r) => {
                                    let v = *l as f64 + *r;
                                    *slot = Value::Float(v);
                                    return Ok(Value::Float(v));
                                }
                                _ => {}
                            },
                            Value::Float(l) => match &right {
                                Value::Float(r) => {
                                    *l += *r;
                                    return Ok(Value::Float(*l));
                                }
                                Value::Int(r) => {
                                    *l += *r as f64;
                                    return Ok(Value::Float(*l));
                                }
                                _ => {}
                            },
                            Value::String(l) => {
                                l.push_str(&right.to_string());
                                return Ok(Value::String(l.clone()));
                            }
                            Value::Array(l) => {
                                if let Value::Array(r) = &right {
                                    l.extend(r.iter().cloned());
                                    return Ok(Value::Array(l.clone()));
                                }
                            }
                            _ => {}
                        },
                        "-=" => match slot {
                            Value::Int(l) => match &right {
                                Value::Int(r) => {
                                    *l -= *r;
                                    return Ok(Value::Int(*l));
                                }
                                Value::Float(r) => {
                                    let v = *l as f64 - *r;
                                    *slot = Value::Float(v);
                                    return Ok(Value::Float(v));
                                }
                                _ => {}
                            },
                            Value::Float(l) => match &right {
                                Value::Float(r) => {
                                    *l -= *r;
                                    return Ok(Value::Float(*l));
                                }
                                Value::Int(r) => {
                                    *l -= *r as f64;
                                    return Ok(Value::Float(*l));
                                }
                                _ => {}
                            },
                            _ => {}
                        },
                        "*=" => match slot {
                            Value::Int(l) => match &right {
                                Value::Int(r) => {
                                    *l *= *r;
                                    return Ok(Value::Int(*l));
                                }
                                Value::Float(r) => {
                                    let v = *l as f64 * *r;
                                    *slot = Value::Float(v);
                                    return Ok(Value::Float(v));
                                }
                                _ => {}
                            },
                            Value::Float(l) => match &right {
                                Value::Float(r) => {
                                    *l *= *r;
                                    return Ok(Value::Float(*l));
                                }
                                Value::Int(r) => {
                                    *l *= *r as f64;
                                    return Ok(Value::Float(*l));
                                }
                                _ => {}
                            },
                            _ => {}
                        },
                        "/=" => match slot {
                            Value::Int(l) => match &right {
                                Value::Int(r) => {
                                    if *r == 0 {
                                        return Err(ZekkenError::runtime(
                                            "Division by zero",
                                            assign.location.line,
                                            assign.location.column,
                                            Some("division by zero"),
                                        ));
                                    }
                                    *l /= *r;
                                    return Ok(Value::Int(*l));
                                }
                                Value::Float(r) => {
                                    if *r == 0.0 {
                                        return Err(ZekkenError::runtime(
                                            "Division by zero",
                                            assign.location.line,
                                            assign.location.column,
                                            Some("division by zero"),
                                        ));
                                    }
                                    let v = *l as f64 / *r;
                                    *slot = Value::Float(v);
                                    return Ok(Value::Float(v));
                                }
                                _ => {}
                            },
                            Value::Float(l) => match &right {
                                Value::Float(r) => {
                                    if *r == 0.0 {
                                        return Err(ZekkenError::runtime(
                                            "Division by zero",
                                            assign.location.line,
                                            assign.location.column,
                                            Some("division by zero"),
                                        ));
                                    }
                                    *l /= *r;
                                    return Ok(Value::Float(*l));
                                }
                                Value::Int(r) => {
                                    if *r == 0 {
                                        return Err(ZekkenError::runtime(
                                            "Division by zero",
                                            assign.location.line,
                                            assign.location.column,
                                            Some("division by zero"),
                                        ));
                                    }
                                    *l /= *r as f64;
                                    return Ok(Value::Float(*l));
                                }
                                _ => {}
                            },
                            _ => {}
                        },
                        "%=" => match slot {
                            Value::Int(l) => {
                                if let Value::Int(r) = &right {
                                    if *r == 0 {
                                        return Err(ZekkenError::runtime(
                                            "Modulo by zero",
                                            assign.location.line,
                                            assign.location.column,
                                            Some("modulo by zero"),
                                        ));
                                    }
                                    *l %= *r;
                                    return Ok(Value::Int(*l));
                                }
                            }
                            _ => {}
                        },
                        _ => {}
                    }
                }
            }

            let final_value = if let Some(op) = base_op {
                let left = env.lookup_ref(&id.name).ok_or_else(|| {
                    ZekkenError::reference_with_span(
                        &format!("Variable '{}' not found", id.name),
                        "variable",
                        id.location.line,
                        id.location.column,
                        id.name.len().max(1),
                    )
                })?;
                eval_binary(left, &right, op, &assign.location)?
            } else {
                right
            };
            env.assign(&id.name, final_value.clone()).map_err(|e| {
                ZekkenError::runtime(&e, assign.location.line, assign.location.column, None)
            })?;
            Ok(final_value)
        }
        Expr::Member(_) => {
            let (root, path) = collect_member_path(assign.left.as_ref(), env).map_err(|e| {
                ZekkenError::runtime(&e, assign.location.line, assign.location.column, None)
            })?;

            let final_value = if let Some(op) = base_op {
                let root_ref = env.lookup_ref(&root).ok_or_else(|| {
                    ZekkenError::reference_with_span(
                        &format!("Variable '{}' not found", root),
                        "variable",
                        assign.location.line,
                        assign.location.column,
                        root.len().max(1),
                    )
                })?;
                let current = get_at_path(root_ref, &path).map_err(|e| {
                    ZekkenError::runtime(&e, assign.location.line, assign.location.column, None)
                })?;
                eval_binary(&current, &right, op, &assign.location)?
            } else {
                right
            };

            let root_slot = env.lookup_mut_assignable(&root).map_err(|e| {
                ZekkenError::runtime(&e, assign.location.line, assign.location.column, None)
            })?;
            assign_at_path(root_slot, &path, final_value.clone()).map_err(|e| {
                ZekkenError::runtime(&e, assign.location.line, assign.location.column, None)
            })?;
            Ok(final_value)
        }
        _ => Err(ZekkenError::type_error(
            "Invalid assignment target",
            "identifier or member access",
            "other",
            assign.location.line,
            assign.location.column,
        )),
    }
}

fn eval_member_native(member: &MemberExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    // Fast path for member chains like a[i][k] or obj.key.other:
    // walk by reference and only clone final value.
    fn collect_chain<'a>(expr: &'a Expr, out: &mut Vec<&'a Expr>) -> Option<&'a Identifier> {
        match expr {
            Expr::Member(m) => {
                let root = collect_chain(m.object.as_ref(), out)?;
                out.push(m.property.as_ref());
                Some(root)
            }
            Expr::Identifier(id) => Some(id),
            _ => None,
        }
    }

    let mut chain = Vec::new();
    if let Some(root_ident) = collect_chain(member.object.as_ref(), &mut chain) {
        chain.push(member.property.as_ref());

        let supports_fast_chain = chain.iter().all(|prop| {
            matches!(
                prop,
                Expr::IntLit(_) | Expr::FloatLit(_) | Expr::Identifier(_) | Expr::StringLit(_)
            )
        });

        if supports_fast_chain {
            let mut current = env.lookup_ref(&root_ident.name).ok_or_else(|| {
                ZekkenError::reference_with_span(
                    &format!("Variable '{}' not found", root_ident.name),
                    "variable",
                    root_ident.location.line,
                    root_ident.location.column,
                    root_ident.name.len().max(1),
                )
            })?;

            for prop in chain {
                current = match current {
                    Value::Array(arr) => {
                        let idx = match prop {
                            Expr::IntLit(lit) if lit.value >= 0 => Some(lit.value as usize),
                            Expr::FloatLit(lit) if lit.value >= 0.0 && lit.value.fract() == 0.0 => Some(lit.value as usize),
                            Expr::Identifier(ident) => match env.lookup_ref(&ident.name) {
                                Some(Value::Int(i)) if *i >= 0 => Some(*i as usize),
                                Some(Value::Float(f)) if *f >= 0.0 && f.fract() == 0.0 => Some(*f as usize),
                                _ => None,
                            },
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
                    Expr::Identifier(ident) => map.get(&ident.name).ok_or_else(|| {
                        ZekkenError::reference_with_span(
                            &format!("Property '{}' not found", ident.name),
                            &ident.name,
                            member.location.line,
                            member.location.column,
                            ident.name.len().max(1),
                        )
                    })?,
                    Expr::StringLit(lit) => map.get(&lit.value).ok_or_else(|| {
                        ZekkenError::reference_with_span(
                            &format!("Property '{}' not found", lit.value),
                            &lit.value,
                            member.location.line,
                            member.location.column,
                            lit.value.len().max(1),
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
                            ZekkenError::reference_with_span(
                                &format!("Property '{}' not found", key),
                                key,
                                member.location.line,
                                member.location.column,
                                key.len().max(1),
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
            return Ok(current.clone());
        }
    }

    let object = eval_expr_native(&member.object, env)?;

    let key = match member.property.as_ref() {
        Expr::Identifier(ident) => {
            if matches!(object, Value::Array(_)) {
                match env.lookup_ref(&ident.name) {
                    Some(Value::Int(i)) if *i >= 0 => MemberKey::Index(*i as usize),
                    Some(Value::Float(f)) if *f >= 0.0 && f.fract() == 0.0 => MemberKey::Index(*f as usize),
                    _ => match eval_expr_native(member.property.as_ref(), env)? {
                        Value::Int(i) if i >= 0 => MemberKey::Index(i as usize),
                        Value::Float(f) if f >= 0.0 && f.fract() == 0.0 => MemberKey::Index(f as usize),
                        _ => MemberKey::Prop(ident.name.clone()),
                    },
                }
            } else {
                MemberKey::Prop(ident.name.clone())
            }
        }
        Expr::StringLit(s) => MemberKey::Prop(s.value.clone()),
        Expr::IntLit(i) if i.value >= 0 => MemberKey::Index(i.value as usize),
        Expr::FloatLit(f) if f.value >= 0.0 && f.value.fract() == 0.0 => MemberKey::Index(f.value as usize),
        _ => match eval_expr_native(member.property.as_ref(), env)? {
            Value::Int(i) if i >= 0 => MemberKey::Index(i as usize),
            Value::Float(f) if f >= 0.0 && f.fract() == 0.0 => MemberKey::Index(f as usize),
            _ => {
                return Err(ZekkenError::type_error(
                    "Invalid property access",
                    "string/int/identifier",
                    "other",
                    member.location.line,
                    member.location.column,
                ))
            }
        },
    };

    match (object, key) {
        (Value::Array(arr), MemberKey::Index(i)) => arr.get(i).cloned().ok_or_else(|| {
            ZekkenError::runtime(&format!("Array index {} out of bounds", i), member.location.line, member.location.column, None)
        }),
        (Value::Array(arr), MemberKey::Prop(prop)) => match prop.as_str() {
            "length" => Ok(Value::Int(arr.len() as i64)),
            "first" => arr.first().cloned().ok_or_else(|| ZekkenError::runtime("Array is empty", member.location.line, member.location.column, None)),
            "last" => arr.last().cloned().ok_or_else(|| ZekkenError::runtime("Array is empty", member.location.line, member.location.column, None)),
            _ => Err(ZekkenError::type_error("Invalid member access", "array index or known array member", "other", member.location.line, member.location.column)),
        },
        (Value::Object(map), MemberKey::Prop(prop)) => map.get(&prop).cloned().ok_or_else(|| {
            ZekkenError::reference_with_span(
                &format!("Property '{}' not found", prop),
                &prop,
                member.location.line,
                member.location.column,
                prop.len().max(1),
            )
        }),
        (Value::Object(map), MemberKey::Index(i)) => {
            let key = match map.get("__keys__") {
                Some(Value::Array(keys)) => match keys.get(i) {
                    Some(Value::String(s)) => s.clone(),
                    _ => {
                        return Err(ZekkenError::runtime(
                            &format!("Object index {} out of bounds", i),
                            member.location.line,
                            member.location.column,
                            None,
                        ))
                    }
                },
                _ => {
                    return Err(ZekkenError::runtime(
                        "Object does not support numeric indexing",
                        member.location.line,
                        member.location.column,
                        None,
                    ))
                }
            };
            map.get(&key).cloned().ok_or_else(|| {
                ZekkenError::reference_with_span(
                    &format!("Property '{}' not found", key),
                    &key,
                    member.location.line,
                    member.location.column,
                    key.len().max(1),
                )
            })
        }
        (_, _) => Err(ZekkenError::type_error(
            "Invalid member access",
            "object/array",
            "other",
            member.location.line,
            member.location.column,
        )),
    }
}

#[inline]
fn eval_arg_hot_native(expr: &Expr, env: &mut Environment) -> Result<Value, ZekkenError> {
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
            eval_expr_native(expr, env)
        }
        _ => eval_expr_native(expr, env),
    }
}

#[inline]
fn eval_call_args_native(args: &[Box<Expr>], env: &mut Environment) -> Result<Vec<Value>, ZekkenError> {
    match args.len() {
        0 => Ok(Vec::new()),
        1 => Ok(vec![eval_arg_hot_native(&args[0], env)?]),
        2 => {
            let mut out = Vec::with_capacity(2);
            out.push(eval_arg_hot_native(&args[0], env)?);
            out.push(eval_arg_hot_native(&args[1], env)?);
            Ok(out)
        }
        3 => {
            let mut out = Vec::with_capacity(3);
            out.push(eval_arg_hot_native(&args[0], env)?);
            out.push(eval_arg_hot_native(&args[1], env)?);
            out.push(eval_arg_hot_native(&args[2], env)?);
            Ok(out)
        }
        _ => {
            let mut out = Vec::with_capacity(args.len());
            for arg in args {
                out.push(eval_arg_hot_native(arg, env)?);
            }
            Ok(out)
        }
    }
}

fn try_eval_math_call_native(
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
                return Err(ZekkenError::runtime("Expected 1 argument", line, column, Some("argument mismatch")));
            }
            let n = as_num(eval_arg_hot_native(&args[0], env)?, line, column)?;
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
                return Err(ZekkenError::runtime("Expected 2 arguments", line, column, Some("argument mismatch")));
            }
            let l = as_num(eval_arg_hot_native(&args[0], env)?, line, column)?;
            let r = as_num(eval_arg_hot_native(&args[1], env)?, line, column)?;
            Ok(Value::Float(l.powf(r)))
        })()),
        "log" => Some((|| -> Result<Value, ZekkenError> {
            if args.is_empty() || args.len() > 2 {
                return Err(ZekkenError::runtime("Expected 1 or 2 arguments", line, column, Some("argument mismatch")));
            }
            let n = as_num(eval_arg_hot_native(&args[0], env)?, line, column)?;
            if args.len() == 2 {
                let base = as_num(eval_arg_hot_native(&args[1], env)?, line, column)?;
                Ok(Value::Float(n.log(base)))
            } else {
                Ok(Value::Float(n.ln()))
            }
        })()),
        "exp" | "floor" | "ceil" | "round" => Some((|| -> Result<Value, ZekkenError> {
            if args.len() != 1 {
                return Err(ZekkenError::runtime("Expected 1 argument", line, column, Some("argument mismatch")));
            }
            let n = as_num(eval_arg_hot_native(&args[0], env)?, line, column)?;
            Ok(Value::Float(match method {
                "exp" => n.exp(),
                "floor" => n.floor(),
                "ceil" => n.ceil(),
                _ => n.round(),
            }))
        })()),
        "min" | "max" => Some((|| -> Result<Value, ZekkenError> {
            if args.len() != 2 {
                return Err(ZekkenError::runtime("Expected 2 arguments", line, column, Some("argument mismatch")));
            }
            let l = as_num(eval_arg_hot_native(&args[0], env)?, line, column)?;
            let r = as_num(eval_arg_hot_native(&args[1], env)?, line, column)?;
            Ok(Value::Float(if method == "min" { l.min(r) } else { l.max(r) }))
        })()),
        "clamp" => Some((|| -> Result<Value, ZekkenError> {
            if args.len() != 3 {
                return Err(ZekkenError::runtime("Expected 3 arguments", line, column, Some("argument mismatch")));
            }
            let x = as_num(eval_arg_hot_native(&args[0], env)?, line, column)?;
            let min = as_num(eval_arg_hot_native(&args[1], env)?, line, column)?;
            let max = as_num(eval_arg_hot_native(&args[2], env)?, line, column)?;
            Ok(Value::Float(x.max(min).min(max)))
        })()),
        "atan2" => Some((|| -> Result<Value, ZekkenError> {
            if args.len() != 2 {
                return Err(ZekkenError::runtime("Expected 2 arguments", line, column, Some("argument mismatch")));
            }
            let y = as_num(eval_arg_hot_native(&args[0], env)?, line, column)?;
            let x = as_num(eval_arg_hot_native(&args[1], env)?, line, column)?;
            Ok(Value::Float(y.atan2(x)))
        })()),
        _ => None,
    }
}

fn eval_call_native(call: &CallExpr, env: &mut Environment) -> Result<Value, ZekkenError> {
    if let Expr::Member(member) = call.callee.as_ref() {
        let method_name = match member.property.as_ref() {
            Expr::Identifier(id) => id.name.clone(),
            Expr::StringLit(s) => s.value.clone(),
            _ => {
                return Err(ZekkenError::type_error(
                    "Invalid method name",
                    "identifier/string",
                    "other",
                    call.location.line,
                    call.location.column,
                ))
            }
        };

        if let Expr::Identifier(object_ident) = member.object.as_ref() {
            // Hot path for math library calls.
            if object_ident.name == "math" {
                if let Some(result) = try_eval_math_call_native(
                    method_name.as_str(),
                    &call.args,
                    env,
                    call.location.line,
                    call.location.column,
                ) {
                    return result;
                }
            }

            // Hot path for object-backed native methods (queue, fs/os/math objects, etc.)
            // Avoid cloning the entire object value just to reach a native function.
            let native_member = match env.lookup_ref(&object_ident.name) {
                Some(Value::Object(map)) => match map.get(&method_name) {
                    Some(Value::NativeFunction(native)) => Some(native.clone()),
                    _ => None,
                },
                _ => None,
            };
            if let Some(native) = native_member {
                let args = eval_call_args_native(&call.args, env)?;
                return native(args).map_err(|msg| {
                    ZekkenError::runtime(&msg, call.location.line, call.location.column, None)
                });
            }
        }

        let args = eval_call_args_native(&call.args, env)?;
        let var_name = match member.object.as_ref() {
            Expr::Identifier(id) => Some(id.name.as_str()),
            _ => None,
        };

        if let Expr::Identifier(id) = member.object.as_ref() {
            if let Some(obj_owned) = env.lookup_ref(&id.name).cloned() {
                return obj_owned
                    .call_method(&method_name, args, Some(env), Some(id.name.as_str()))
                    .map_err(|msg| ZekkenError::runtime(&msg, call.location.line, call.location.column, None));
            }
        }

        let object = eval_expr_native(&member.object, env)?;

        return object
            .call_method(&method_name, args, Some(env), var_name)
            .map_err(|msg| ZekkenError::runtime(&msg, call.location.line, call.location.column, None));
    }

    if let Expr::Identifier(id) = call.callee.as_ref() {
        if id.name == "queue" && !call.is_native {
            return Err(ZekkenError::runtime(
                "queue is a native constructor; call it with '@queue => ||'",
                call.location.line,
                call.location.column,
                None,
            ));
        }

        let args = eval_call_args_native(&call.args, env)?;

        if let Some(func) = match env.variables.get(&id.name) {
            Some(Value::Function(f)) => Some(f.clone()),
            _ => None,
        } {
            return call_function_native(&func, args, env, call.location.line, call.location.column);
        }
        if let Some(native) = match env.variables.get(&id.name) {
            Some(Value::NativeFunction(n)) => Some(n.clone()),
            _ => None,
        } {
            return native(args).map_err(|msg| ZekkenError::runtime(&msg, call.location.line, call.location.column, None));
        }
        if let Some(func) = match env.constants.get(&id.name) {
            Some(Value::Function(f)) => Some(f.clone()),
            _ => None,
        } {
            return call_function_native(&func, args, env, call.location.line, call.location.column);
        }
        if let Some(native) = match env.constants.get(&id.name) {
            Some(Value::NativeFunction(n)) => Some(n.clone()),
            _ => None,
        } {
            return native(args).map_err(|msg| ZekkenError::runtime(&msg, call.location.line, call.location.column, None));
        }

        let callee = env.lookup_ref(&id.name).cloned().ok_or_else(|| {
            ZekkenError::reference_with_span(
                &format!("Function '{}' not found", id.name),
                "function",
                id.location.line,
                id.location.column,
                id.name.len().max(1),
            )
        })?;

        return match callee {
            Value::Function(func) => call_function_native(&func, args, env, call.location.line, call.location.column),
            Value::NativeFunction(native) => native(args).map_err(|msg| ZekkenError::runtime(&msg, call.location.line, call.location.column, None)),
            other => Err(ZekkenError::type_error(
                "Attempted to call a non-callable value",
                "function or native function",
                value_type_name(&other),
                call.location.line,
                call.location.column,
            )),
        };
    }

    let args = eval_call_args_native(&call.args, env)?;
    let callee = eval_expr_native(&call.callee, env)?;
    match callee {
        Value::Function(func) => call_function_native(&func, args, env, call.location.line, call.location.column),
        Value::NativeFunction(native) => native(args).map_err(|msg| ZekkenError::runtime(&msg, call.location.line, call.location.column, None)),
        other => Err(ZekkenError::type_error(
            "Attempted to call a non-callable value",
            "function or native function",
            value_type_name(&other),
            call.location.line,
            call.location.column,
        )),
    }
}

fn call_function_native(
    func: &FunctionValue,
    args: Vec<Value>,
    env: &mut Environment,
    _line: usize,
    _column: usize,
) -> Result<Value, ZekkenError> {
    if args.len() > func.params.len() {
        return Err(ZekkenError::runtime(
            &format!("Expected {} arguments but got {}", func.params.len(), args.len()),
            _line,
            _column,
            Some("argument mismatch"),
        ));
    }

    if func.needs_parent {
        let mut function_env = Environment::new_with_parent(env.clone());
        for (idx, param) in func.params.iter().enumerate() {
            let value = if let Some(arg) = args.get(idx) {
                arg.clone()
            } else if let Some(default_expr) = param.default_value.as_ref() {
                eval_expr_native(default_expr, &mut function_env)?
            } else {
                return Err(ZekkenError::runtime(
                    &format!("Missing required argument '{}'", param.ident),
                    _line,
                    _column,
                    Some("argument mismatch"),
                ));
            };
            if !check_value_type(&value, &param.type_) {
                return Err(ZekkenError::type_error(
                    &format!("Type mismatch for parameter '{}'", param.ident),
                    &format!("{:?}", param.type_),
                    value_type_name(&value),
                    _line,
                    _column,
                ));
            }
            function_env.declare_ref_typed(param.ident.as_str(), value, param.type_, false);
        }
        let result = eval_contents_native(func.body.as_ref(), &mut function_env)?;
        let out = result.unwrap_or(Value::Void);
        if let Some(ret_ty) = func.return_type {
            if !check_value_type(&out, &ret_ty) {
                return Err(ZekkenError::type_error(
                    "Type mismatch in function return value",
                    &format!("{:?}", ret_ty),
                    value_type_name(&out),
                    _line,
                    _column,
                ));
            }
        }
        return Ok(out);
    }

    let mut function_env = Environment::take_pooled_scope(func.params.len() + func.captures.len() + 8);
    if !func.captures.is_empty() {
        for capture in func.captures.iter() {
            if let Some(v) = env.lookup_ref(capture) {
                function_env.declare_ref(capture.as_str(), clone_value_hot(v), false);
            }
        }
    }

    let bind_result = (|| -> Result<(), ZekkenError> {
        for (idx, param) in func.params.iter().enumerate() {
            let value = if let Some(arg) = args.get(idx) {
                arg.clone()
            } else if let Some(default_expr) = param.default_value.as_ref() {
                eval_expr_native(default_expr, &mut function_env)?
            } else {
                return Err(ZekkenError::runtime(
                    &format!("Missing required argument '{}'", param.ident),
                    _line,
                    _column,
                    Some("argument mismatch"),
                ));
            };
            if !check_value_type(&value, &param.type_) {
                return Err(ZekkenError::type_error(
                    &format!("Type mismatch for parameter '{}'", param.ident),
                    &format!("{:?}", param.type_),
                    value_type_name(&value),
                    _line,
                    _column,
                ));
            }
            function_env.declare_ref_typed(param.ident.as_str(), value, param.type_, false);
        }
        Ok(())
    })();

    if let Err(e) = bind_result {
        Environment::return_pooled_scope(function_env);
        return Err(e);
    }

    let result = eval_contents_native(func.body.as_ref(), &mut function_env);
    let out = match result {
        Ok(v) => Ok(v.unwrap_or(Value::Void)),
        Err(e) => Err(e),
    }.and_then(|v| {
        if let Some(ret_ty) = func.return_type {
            if !check_value_type(&v, &ret_ty) {
                return Err(ZekkenError::type_error(
                    "Type mismatch in function return value",
                    &format!("{:?}", ret_ty),
                    value_type_name(&v),
                    _line,
                    _column,
                ));
            }
        }
        Ok(v)
    });
    Environment::return_pooled_scope(function_env);
    out
}

fn eval_expr_native(expr: &Expr, env: &mut Environment) -> Result<Value, ZekkenError> {
    match expr {
        Expr::Assign(assign) => eval_assignment_native(assign, env),
        Expr::Member(member) => eval_member_native(member, env),
        Expr::Call(call) => eval_call_native(call, env),
        Expr::Binary(binary) => {
            if binary.operator == "&&" {
                let left = eval_expr_native(&binary.left, env)?;
                return match left {
                    Value::Boolean(false) => Ok(Value::Boolean(false)),
                    Value::Boolean(true) => match eval_expr_native(&binary.right, env)? {
                        Value::Boolean(b) => Ok(Value::Boolean(b)),
                        other => Err(ZekkenError::type_error("Invalid logical AND operation", "bool", value_type_name(&other), binary.location.line, binary.location.column)),
                    },
                    other => Err(ZekkenError::type_error("Invalid logical AND operation", "bool", value_type_name(&other), binary.location.line, binary.location.column)),
                };
            }
            if binary.operator == "||" {
                let left = eval_expr_native(&binary.left, env)?;
                return match left {
                    Value::Boolean(true) => Ok(Value::Boolean(true)),
                    Value::Boolean(false) => match eval_expr_native(&binary.right, env)? {
                        Value::Boolean(b) => Ok(Value::Boolean(b)),
                        other => Err(ZekkenError::type_error("Invalid logical OR operation", "bool", value_type_name(&other), binary.location.line, binary.location.column)),
                    },
                    other => Err(ZekkenError::type_error("Invalid logical OR operation", "bool", value_type_name(&other), binary.location.line, binary.location.column)),
                };
            }
            let left = eval_expr_native(&binary.left, env)?;
            let right = eval_expr_native(&binary.right, env)?;
            eval_binary(&left, &right, &binary.operator, &binary.location)
        }
        Expr::Identifier(ident) => {
            if let Some(v) = env.variables.get(&ident.name).or_else(|| env.constants.get(&ident.name)) {
                return Ok(match v {
                    Value::Int(i) => Value::Int(*i),
                    Value::Float(f) => Value::Float(*f),
                    Value::Boolean(b) => Value::Boolean(*b),
                    _ => v.clone(),
                });
            }
            env.lookup_ref(&ident.name).map(|v| {
                match v {
                    Value::Int(i) => Value::Int(*i),
                    Value::Float(f) => Value::Float(*f),
                    Value::Boolean(b) => Value::Boolean(*b),
                    _ => v.clone(),
                }
            }).ok_or_else(|| {
                ZekkenError::reference_with_span(
                    &format!("Variable '{}' not found", ident.name),
                    "variable",
                    ident.location.line,
                    ident.location.column,
                    ident.name.len().max(1),
                )
            })
        },
        Expr::Property(_) => Err(ZekkenError::internal("Property expression not supported in this context")),
        Expr::IntLit(v) => Ok(Value::Int(v.value)),
        Expr::FloatLit(v) => Ok(Value::Float(v.value)),
        Expr::StringLit(v) => {
            if v.value.as_bytes().contains(&b'{') {
                Ok(Value::String(interpolate_string_expressions_native(&v.value, env)))
            } else {
                Ok(Value::String(v.value.clone()))
            }
        }
        Expr::BoolLit(v) => Ok(Value::Boolean(v.value)),
        Expr::ArrayLit(arr) => {
            let mut out = Vec::with_capacity(arr.elements.len());
            for e in &arr.elements {
                out.push(eval_expr_native(e.as_ref(), env)?);
            }
            Ok(Value::Array(out))
        }
        Expr::ObjectLit(obj) => {
            let mut map = HashMap::new();
            let mut keys = Vec::with_capacity(obj.properties.len());
            for p in &obj.properties {
                keys.push(Value::String(p.key.clone()));
                map.insert(p.key.clone(), eval_expr_native(&p.value, env)?);
            }
            map.insert("__keys__".to_string(), Value::Array(keys));
            Ok(Value::Object(map))
        }
    }
}

fn interpolate_string_expressions_native(template: &str, env: &mut Environment) -> String {
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
                match eval_expr_native(&expr, env) {
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

fn eval_content_native(content: &Content, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match content {
        Content::Statement(stmt) => eval_stmt_native(stmt.as_ref(), env),
        Content::Expression(expr) => Ok(Some(eval_expr_native(expr.as_ref(), env)?)),
    }
}

fn eval_contents_native(contents: &[Box<Content>], env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let mut last = None;
    for content in contents {
        last = eval_content_native(content.as_ref(), env)?;
    }
    Ok(last)
}

fn content_has_return(content: &Content) -> bool {
    match content {
        Content::Statement(stmt) => stmt_has_return(stmt),
        Content::Expression(_) => false,
    }
}

fn block_has_return(content: &[Box<Content>]) -> bool {
    content.iter().any(|c| content_has_return(c))
}

fn stmt_has_return(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(_) => true,
        Stmt::IfStmt(i) => block_has_return(&i.body) || i.alt.as_ref().map(|b| block_has_return(b)).unwrap_or(false),
        Stmt::ForStmt(f) => block_has_return(&f.body),
        Stmt::WhileStmt(w) => block_has_return(&w.body),
        Stmt::TryCatchStmt(t) => {
            block_has_return(&t.try_block)
                || t.catch_block.as_ref().map(|b| block_has_return(b)).unwrap_or(false)
        }
        Stmt::BlockStmt(b) => block_has_return(&b.body),
        Stmt::Program(p) => {
            p.imports.iter().any(|c| content_has_return(c))
                || p.content.iter().any(|c| content_has_return(c))
        }
        Stmt::FuncDecl(_)
        | Stmt::Lambda(_)
        | Stmt::VarDecl(_)
        | Stmt::ObjectDecl(_)
        | Stmt::Use(_)
        | Stmt::Include(_)
        | Stmt::Export(_) => false,
    }
}

fn eval_contents_discard_native(contents: &[Box<Content>], env: &mut Environment) -> Result<(), ZekkenError> {
    for content in contents {
        match content.as_ref() {
            Content::Statement(stmt) => {
                let _ = eval_stmt_native(stmt, env)?;
            }
            Content::Expression(expr) => match expr.as_ref() {
                Expr::Assign(assign) => {
                    let _ = eval_assignment_native(assign, env)?;
                }
                _ => {
                    let _ = eval_expr_native(expr, env)?;
                }
            },
        }
    }
    Ok(())
}

fn eval_use_native(use_stmt: &UseStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match load_library(&use_stmt.module, env) {
        Ok(_) => {
            if let Some(methods) = &use_stmt.methods {
                if let Some(Value::Object(lib_obj)) = env.lookup(&use_stmt.module) {
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
        }
        Err(e) => Err(ZekkenError::runtime(
            &format!("Failed to load library '{}': {}", use_stmt.module, e),
            use_stmt.location.line,
            use_stmt.location.column,
            None,
        )),
    }
}

fn eval_include_native(include: &IncludeStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
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

    let mut path = std::path::PathBuf::from(&current_dir);
    path.push(&include.file_path);
    let file_path = path.to_string_lossy().to_string();

    let file_contents = std::fs::read_to_string(&file_path).map_err(|e| {
        ZekkenError::runtime(
            &format!("Failed to include file '{}': {}", file_path, e),
            include.location.line,
            include.location.column,
            None,
        )
    })?;

    let prev_file = std::env::var("ZEKKEN_CURRENT_FILE").unwrap_or_else(|_| "<unknown>".to_string());
    std::env::set_var("ZEKKEN_CURRENT_FILE", &file_path);

    let mut parser = Parser::new();
    let included_ast = parser.produce_ast(file_contents);
    if !parser.errors.is_empty() {
        for parse_error in parser.errors {
            push_error(parse_error);
        }
        return Err(ZekkenError::syntax(
            "Failed to parse included file",
            include.location.line,
            include.location.column,
            Some("valid zekken source"),
            Some(&include.file_path),
        ));
    }

    let mut child_env = Environment::new_with_parent(env.clone());
    let result = execute_program(&included_ast, &mut child_env);

    std::env::set_var("ZEKKEN_CURRENT_FILE", prev_file);
    result?;

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

fn eval_export_native(exports: &ExportStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
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

fn set_or_declare_loop_var(env: &mut Environment, name: &str, value: Value) {
    if let Some(slot) = env.variables.get_mut(name) {
        *slot = value;
    } else {
        env.declare_ref(name, value, false);
    }
}

fn eval_for_native(for_stmt: &ForStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let init = for_stmt.init.as_ref().ok_or_else(|| {
        ZekkenError::runtime("For loop requires an initialization", for_stmt.location.line, for_stmt.location.column, None)
    })?;

    let var_decl = match init.as_ref() {
        Stmt::VarDecl(v) => v,
        _ => {
            return Err(ZekkenError::runtime(
                "For loop requires a variable declaration",
                for_stmt.location.line,
                for_stmt.location.column,
                None,
            ))
        }
    };

    let collection = match &var_decl.value {
        Some(Content::Expression(expr)) => eval_expr_native(expr, env)?,
        _ => {
            return Err(ZekkenError::runtime(
                "Expected expression in for loop initialization",
                for_stmt.location.line,
                for_stmt.location.column,
                Some("for |x| in array { ... }"),
            ))
        }
    };

    let mut last = None;
    match collection {
        Value::Array(arr) => {
            let ids: Vec<String> = var_decl.ident.split(", ").map(|s| s.to_string()).collect();
            if ids.len() != 1 {
                return Err(ZekkenError::syntax(
                    "Array iteration requires one identifier",
                    var_decl.location.line,
                    var_decl.location.column,
                    None,
                    None,
                ));
            }
            let body_may_return = block_has_return(&for_stmt.body);
            for value in arr {
                set_or_declare_loop_var(env, &ids[0], value);
                if body_may_return {
                    if let Some(v) = eval_contents_native(&for_stmt.body, env)? {
                        last = Some(v);
                    }
                } else {
                    eval_contents_discard_native(&for_stmt.body, env)?;
                }
            }
        }
        Value::Object(map) => {
            let ids: Vec<String> = var_decl.ident.split(", ").map(|s| s.to_string()).collect();
            if ids.len() != 2 {
                return Err(ZekkenError::syntax(
                    "Object iteration requires two identifiers (key, value)",
                    var_decl.location.line,
                    var_decl.location.column,
                    None,
                    None,
                ));
            }

            let ordered_keys: Vec<String> = match map.get("__keys__") {
                Some(Value::Array(keys)) => keys
                    .iter()
                    .filter_map(|k| if let Value::String(s) = k { Some(s.clone()) } else { None })
                    .collect(),
                _ => map.keys().filter(|k| k.as_str() != "__keys__").cloned().collect(),
            };

            let body_may_return = block_has_return(&for_stmt.body);
            for key in ordered_keys {
                if key == "__keys__" {
                    continue;
                }
                if let Some(value) = map.get(&key) {
                    set_or_declare_loop_var(env, &ids[0], Value::String(key));
                    set_or_declare_loop_var(env, &ids[1], value.clone());
                    if body_may_return {
                        if let Some(v) = eval_contents_native(&for_stmt.body, env)? {
                            last = Some(v);
                        }
                    } else {
                        eval_contents_discard_native(&for_stmt.body, env)?;
                    }
                }
            }
        }
        other => {
            return Err(ZekkenError::type_error(
                "For loop must iterate over an object or array",
                "object or array",
                value_type_name(&other),
                for_stmt.location.line,
                for_stmt.location.column,
            ))
        }
    }

    Ok(last)
}

fn eval_try_catch_native(try_catch: &TryCatchStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match eval_contents_native(&try_catch.try_block, env) {
        Ok(v) => Ok(v),
        Err(error) => {
            if let Some(catch_block) = &try_catch.catch_block {
                let mut err_obj = HashMap::new();
                err_obj.insert("message".to_string(), Value::String(error.message.clone()));
                err_obj.insert("kind".to_string(), Value::String(format!("{:?}", error.kind)));
                err_obj.insert("line".to_string(), Value::Int(error.context.line as i64));
                err_obj.insert("column".to_string(), Value::Int(error.context.column as i64));
                err_obj.insert("__zekken_error__".to_string(), Value::String(error.to_string()));

                let prev_var = env.variables.remove("e");
                let prev_const = env.constants.remove("e");
                env.declare("e".to_string(), Value::Object(err_obj), false);

                let catch_result = eval_contents_native(catch_block, env);

                env.variables.remove("e");
                if let Some(v) = prev_var {
                    env.variables.insert("e".to_string(), v);
                }
                if let Some(c) = prev_const {
                    env.constants.insert("e".to_string(), c);
                }

                catch_result
            } else {
                Err(error)
            }
        }
    }
}

fn eval_stmt_native(stmt: &Stmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match stmt {
        Stmt::Program(program) => execute_program(program, env),
        Stmt::VarDecl(decl) => {
            let value = match decl.value.as_ref() {
                Some(Content::Expression(expr)) => eval_expr_native(expr, env)?,
                Some(Content::Statement(stmt)) => eval_stmt_native(stmt.as_ref(), env)?.unwrap_or(Value::Void),
                None => Value::Void,
            };

            if !check_value_type(&value, &decl.type_) {
                return Err(ZekkenError::type_error(
                    &format!("Type mismatch in variable declaration '{}'", decl.ident),
                    &format!("{:?}", decl.type_),
                    value_type_name(&value),
                    decl.location.line,
                    decl.location.column,
                ));
            }

            env.declare_ref_typed(&decl.ident, value, decl.type_, decl.constant);
            Ok(None)
        }
        Stmt::FuncDecl(func) => {
            let usage = analyze_function_parent_usage(&func.params, &func.body);
            let captures = if usage.requires_parent_clone {
                vec![]
            } else {
                let mut v: Vec<String> = usage.captures.into_iter().collect();
                v.sort_unstable();
                v
            };
            let function_value = FunctionValue {
                params: Arc::new(func.params.clone()),
                body: Arc::new(func.body.clone()),
                return_type: func.return_type,
                needs_parent: usage.requires_parent_clone,
                captures: Arc::new(captures),
            };
            env.declare(func.ident.clone(), Value::Function(function_value), false);
            Ok(None)
        }
        Stmt::ObjectDecl(obj) => {
            let mut map = HashMap::new();
            let mut keys = Vec::with_capacity(obj.properties.len());
            for p in &obj.properties {
                keys.push(Value::String(p.key.clone()));
                map.insert(p.key.clone(), eval_expr_native(&p.value, env)?);
            }
            map.insert("__keys__".to_string(), Value::Array(keys));
            env.declare(obj.ident.clone(), Value::Object(map), false);
            Ok(None)
        }
        Stmt::IfStmt(if_stmt) => {
            let test = eval_expr_native(&if_stmt.test, env)?;
            match test {
                Value::Boolean(true) => eval_contents_native(&if_stmt.body, env),
                Value::Boolean(false) => {
                    if let Some(alt) = &if_stmt.alt {
                        eval_contents_native(alt, env)
                    } else {
                        Ok(None)
                    }
                }
                other => Err(ZekkenError::type_error(
                    "If statement condition must evaluate to a boolean",
                    "bool",
                    value_type_name(&other),
                    if_stmt.location.line,
                    if_stmt.location.column,
                )),
            }
        }
        Stmt::ForStmt(for_stmt) => eval_for_native(for_stmt, env),
        Stmt::WhileStmt(while_stmt) => {
            let body_may_return = block_has_return(&while_stmt.body);
            #[derive(Clone)]
            enum NumCondOperand {
                Ident(String),
                Int(i64),
                Float(f64),
            }

            #[derive(Copy, Clone)]
            enum NumCondOp {
                Lt,
                Lte,
                Gt,
                Gte,
                Eq,
                Neq,
            }

            #[derive(Clone)]
            struct NumCond {
                left: NumCondOperand,
                right: NumCondOperand,
                op: NumCondOp,
            }

            fn as_num_operand(expr: &Expr) -> Option<NumCondOperand> {
                match expr {
                    Expr::Identifier(id) => Some(NumCondOperand::Ident(id.name.clone())),
                    Expr::IntLit(i) => Some(NumCondOperand::Int(i.value)),
                    Expr::FloatLit(f) => Some(NumCondOperand::Float(f.value)),
                    _ => None,
                }
            }

            fn as_num_value(op: &NumCondOperand, env: &Environment) -> Option<f64> {
                match op {
                    NumCondOperand::Int(i) => Some(*i as f64),
                    NumCondOperand::Float(f) => Some(*f),
                    NumCondOperand::Ident(name) => {
                        if let Some(v) = env.variables.get(name).or_else(|| env.constants.get(name)) {
                            return match v {
                                Value::Int(i) => Some(*i as f64),
                                Value::Float(f) => Some(*f),
                                _ => None,
                            };
                        }
                        match env.lookup_ref(name) {
                            Some(Value::Int(i)) => Some(*i as f64),
                            Some(Value::Float(f)) => Some(*f),
                            _ => None,
                        }
                    }
                }
            }

            fn as_int_value(op: &NumCondOperand, env: &Environment) -> Option<i64> {
                match op {
                    NumCondOperand::Int(i) => Some(*i),
                    NumCondOperand::Float(f) => {
                        if f.fract() == 0.0 {
                            Some(*f as i64)
                        } else {
                            None
                        }
                    }
                    NumCondOperand::Ident(name) => {
                        if let Some(v) = env.variables.get(name).or_else(|| env.constants.get(name)) {
                            return match v {
                                Value::Int(i) => Some(*i),
                                Value::Float(f) if f.fract() == 0.0 => Some(*f as i64),
                                _ => None,
                            };
                        }
                        match env.lookup_ref(name) {
                            Some(Value::Int(i)) => Some(*i),
                            Some(Value::Float(f)) if f.fract() == 0.0 => Some(*f as i64),
                            _ => None,
                        }
                    }
                }
            }

            fn build_numeric_cond(test: &Expr) -> Option<NumCond> {
                let bin = match test {
                    Expr::Binary(b) => b,
                    _ => return None,
                };
                let op = match bin.operator.as_str() {
                    "<" => NumCondOp::Lt,
                    "<=" => NumCondOp::Lte,
                    ">" => NumCondOp::Gt,
                    ">=" => NumCondOp::Gte,
                    "==" => NumCondOp::Eq,
                    "!=" => NumCondOp::Neq,
                    _ => return None,
                };
                let left = as_num_operand(&bin.left)?;
                let right = as_num_operand(&bin.right)?;
                Some(NumCond { left, right, op })
            }

            if let Some(cond) = build_numeric_cond(&while_stmt.test) {
                let mut result = None;
                loop {
                    let test_true = if let (Some(l), Some(r)) = (as_int_value(&cond.left, env), as_int_value(&cond.right, env)) {
                        match cond.op {
                            NumCondOp::Lt => l < r,
                            NumCondOp::Lte => l <= r,
                            NumCondOp::Gt => l > r,
                            NumCondOp::Gte => l >= r,
                            NumCondOp::Eq => l == r,
                            NumCondOp::Neq => l != r,
                        }
                    } else {
                        let l = as_num_value(&cond.left, env).ok_or_else(|| {
                            ZekkenError::type_error(
                                "While loop condition must evaluate to a boolean",
                                "bool",
                                "non-boolean",
                                while_stmt.location.line,
                                while_stmt.location.column,
                            )
                        })?;
                        let r = as_num_value(&cond.right, env).ok_or_else(|| {
                            ZekkenError::type_error(
                                "While loop condition must evaluate to a boolean",
                                "bool",
                                "non-boolean",
                                while_stmt.location.line,
                                while_stmt.location.column,
                            )
                        })?;
                        match cond.op {
                            NumCondOp::Lt => l < r,
                            NumCondOp::Lte => l <= r,
                            NumCondOp::Gt => l > r,
                            NumCondOp::Gte => l >= r,
                            NumCondOp::Eq => l == r,
                            NumCondOp::Neq => l != r,
                        }
                    };

                    if !test_true {
                        break;
                    }
                    if body_may_return {
                        if let Some(v) = eval_contents_native(&while_stmt.body, env)? {
                            result = Some(v);
                        }
                    } else {
                        eval_contents_discard_native(&while_stmt.body, env)?;
                    }
                }
                return Ok(result);
            }

            let mut last = None;
            loop {
                let test = eval_expr_native(&while_stmt.test, env)?;
                match test {
                    Value::Boolean(true) => {
                        if body_may_return {
                            if let Some(v) = eval_contents_native(&while_stmt.body, env)? {
                                last = Some(v);
                            }
                        } else {
                            eval_contents_discard_native(&while_stmt.body, env)?;
                        }
                    }
                    Value::Boolean(false) => break,
                    other => {
                        return Err(ZekkenError::type_error(
                            "While loop condition must evaluate to a boolean",
                            "bool",
                            value_type_name(&other),
                            while_stmt.location.line,
                            while_stmt.location.column,
                        ))
                    }
                }
            }
            Ok(last)
        }
        Stmt::TryCatchStmt(try_catch) => eval_try_catch_native(try_catch, env),
        Stmt::BlockStmt(block) => eval_contents_native(&block.body, env),
        Stmt::Use(use_stmt) => eval_use_native(use_stmt, env),
        Stmt::Include(include) => eval_include_native(include, env),
        Stmt::Export(exports) => eval_export_native(exports, env),
        Stmt::Return(ret) => {
            let value = match &ret.value {
                Some(content) => match content.as_ref() {
                    Content::Expression(expr) => eval_expr_native(expr, env)?,
                    Content::Statement(stmt) => eval_stmt_native(stmt.as_ref(), env)?.unwrap_or(Value::Void),
                },
                None => Value::Void,
            };
            Ok(Some(value))
        }
        Stmt::Lambda(lambda) => {
            let usage = analyze_function_parent_usage(&lambda.params, &lambda.body);
            let captures = if usage.requires_parent_clone {
                vec![]
            } else {
                let mut v: Vec<String> = usage.captures.into_iter().collect();
                v.sort_unstable();
                v
            };
            let function_value = FunctionValue {
                params: Arc::new(lambda.params.clone()),
                body: Arc::new(lambda.body.clone()),
                return_type: lambda.return_type,
                needs_parent: usage.requires_parent_clone,
                captures: Arc::new(captures),
            };
            env.declare(lambda.ident.clone(), Value::Function(function_value), lambda.constant);
            Ok(None)
        }
    }
}

#[allow(dead_code)]
pub fn execute_program(program: &Program, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let mut compiler = Compiler::new();
    for import in &program.imports {
        compiler.compile_content(import);
    }
    compiler.compile_contents(&program.content);
    run_insts(&compiler.insts, compiler.next_reg, env)
}

pub fn execute_contents(contents: &[Box<Content>], env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    if contents.is_empty() {
        return Ok(None);
    }
    let mut compiler = Compiler::new();
    compiler.compile_contents(contents);
    run_insts(&compiler.insts, compiler.next_reg, env)
}
