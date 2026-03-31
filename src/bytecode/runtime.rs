use crate::ast::*;
use crate::environment::{Environment, FunctionValue, Value};
use crate::errors::ZekkenError;
use crate::lexer::DataType;
use hashbrown::HashMap;
use std::sync::Arc;

use super::compiler::analyze_function_parent_usage;
use super::inst::{BinaryOpCode, Inst, Reg};

#[inline]
pub(super) fn value_type_name(val: &Value) -> &'static str {
    match val {
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

#[inline]
pub(super) fn clone_value_hot(v: &Value) -> Value {
    match v {
        Value::Int(i) => Value::Int(*i),
        Value::Float(f) => Value::Float(*f),
        Value::Boolean(b) => Value::Boolean(*b),
        _ => v.clone(),
    }
}

#[inline]
pub(super) fn check_value_type(value: &Value, expected: &DataType) -> bool {
    match (value, expected) {
        (_, DataType::Any) => true,
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

pub(super) fn compare_values(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Int(l), Value::Int(r)) => l == r,
        (Value::Float(l), Value::Float(r)) => l == r,
        (Value::Int(l), Value::Float(r)) => (*l as f64) == *r,
        (Value::Float(l), Value::Int(r)) => *l == (*r as f64),
        (Value::String(l), Value::String(r)) => l == r,
        (Value::Boolean(l), Value::Boolean(r)) => l == r,
        (Value::Void, Value::Void) => true,
        _ => false,
    }
}

#[inline]
pub(super) fn eval_binary_opcode(left: &Value, right: &Value, op: BinaryOpCode, location: &Location) -> Result<Value, ZekkenError> {
    match op {
        BinaryOpCode::Add => match (left, right) {
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
        BinaryOpCode::Sub => match (left, right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l - r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l - r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 - r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l - *r as f64)),
            _ => Err(ZekkenError::type_error("Invalid operand types for subtraction", "number", "non-number", location.line, location.column)),
        },
        BinaryOpCode::Mul => match (left, right) {
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l * r)),
            (Value::Float(l), Value::Float(r)) => Ok(Value::Float(l * r)),
            (Value::Int(l), Value::Float(r)) => Ok(Value::Float(*l as f64 * r)),
            (Value::Float(l), Value::Int(r)) => Ok(Value::Float(l * *r as f64)),
            _ => Err(ZekkenError::type_error("Invalid operand types for multiplication", "number", "non-number", location.line, location.column)),
        },
        BinaryOpCode::Div => match (left, right) {
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
        BinaryOpCode::Mod => match (left, right) {
            (Value::Int(_), Value::Int(r)) if *r == 0 => Err(ZekkenError::runtime("Modulo by zero", location.line, location.column, Some("modulo by zero"))),
            (Value::Int(l), Value::Int(r)) => Ok(Value::Int(l % r)),
            _ => Err(ZekkenError::type_error("Invalid operand types for modulo", "int", "non-int", location.line, location.column)),
        },
        BinaryOpCode::In => match (left, right) {
            (_, Value::Array(arr)) => Ok(Value::Boolean(arr.iter().any(|v| compare_values(left, v)))),
            (Value::String(key), Value::Object(obj)) => Ok(Value::Boolean(obj.contains_key(key))),
            (Value::String(needle), Value::String(haystack)) => Ok(Value::Boolean(haystack.contains(needle))),
            _ => Err(ZekkenError::type_error(
                "Invalid 'in' operation",
                "value in array, string in object, or string in string",
                "incompatible operands",
                location.line,
                location.column,
            )),
        },
        BinaryOpCode::Eq => Ok(Value::Boolean(compare_values(left, right))),
        BinaryOpCode::Ne => Ok(Value::Boolean(!compare_values(left, right))),
        BinaryOpCode::Lt => cmp_num(left, right, location, |l, r| l < r),
        BinaryOpCode::Gt => cmp_num(left, right, location, |l, r| l > r),
        BinaryOpCode::Le => cmp_num(left, right, location, |l, r| l <= r),
        BinaryOpCode::Ge => cmp_num(left, right, location, |l, r| l >= r),
    }
}

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

#[inline]
fn value_to_non_negative_index(value: &Value) -> Option<usize> {
    match value {
        Value::Int(i) if *i >= 0 => Some(*i as usize),
        Value::Float(f) if *f >= 0.0 && f.fract() == 0.0 => Some(*f as usize),
        _ => None,
    }
}

pub(super) fn run_insts(insts: &[Inst], reg_count: usize, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let mut regs = vec![Value::Void; reg_count.max(1)];
    let mut ip = 0usize;
    let mut last_value: Option<Value> = None;

    #[inline]
    fn get_reg(regs: &[Value], reg: Reg) -> &Value {
        debug_assert!(reg < regs.len());
        &regs[reg]
    }

    #[inline]
    fn get_reg_mut(regs: &mut [Value], reg: Reg) -> &mut Value {
        debug_assert!(reg < regs.len());
        &mut regs[reg]
    }

    while ip < insts.len() {
        match &insts[ip] {
            Inst::LoadConst { dst, value } => {
                *get_reg_mut(&mut regs, *dst) = clone_value_hot(value);
            }
            Inst::LoadIdent { dst, name, location } => {
                if let Some(found) = env.variables.get(name).or_else(|| env.constants.get(name)) {
                    *get_reg_mut(&mut regs, *dst) = clone_value_hot(found);
                    ip += 1;
                    continue;
                }
                let found = env.lookup_ref(name).ok_or_else(|| {
                    ZekkenError::reference_with_span(
                        &format!("Variable '{}' not found", name),
                        "variable",
                        location.line,
                        location.column,
                        name.len().max(1),
                    )
                })?;
                *get_reg_mut(&mut regs, *dst) = clone_value_hot(found);
            }
            Inst::LoadIndex { dst, object, index, location } => {
                let obj = get_reg(&regs, *object);
                let idx_val = get_reg(&regs, *index);
                let value = match obj {
                    Value::Array(arr) => {
                        let idx = value_to_non_negative_index(idx_val).ok_or_else(|| {
                            ZekkenError::type_error(
                                "Invalid array index",
                                "non-negative int",
                                value_type_name(idx_val),
                                location.line,
                                location.column,
                            )
                        })?;
                        arr.get(idx).cloned().ok_or_else(|| {
                            ZekkenError::runtime(
                                &format!("Array index out of bounds: {}", idx),
                                location.line,
                                location.column,
                                None,
                            )
                        })?
                    }
                    Value::Object(map) => {
                        match idx_val {
                            Value::String(k) => map.get(k).cloned().ok_or_else(|| {
                                ZekkenError::runtime(
                                    &format!("Property '{}' not found", k),
                                    location.line,
                                    location.column,
                                    None,
                                )
                            })?,
                            Value::Int(i) => map.get(&i.to_string()).cloned().ok_or_else(|| {
                                ZekkenError::runtime(
                                    &format!("Property '{}' not found", i),
                                    location.line,
                                    location.column,
                                    None,
                                )
                            })?,
                            Value::Float(f) => map.get(&f.to_string()).cloned().ok_or_else(|| {
                                ZekkenError::runtime(
                                    &format!("Property '{}' not found", f),
                                    location.line,
                                    location.column,
                                    None,
                                )
                            })?,
                            _ => {
                                return Err(ZekkenError::type_error(
                                    "Invalid object key type",
                                    "string/int/float",
                                    value_type_name(idx_val),
                                    location.line,
                                    location.column,
                                ));
                            }
                        }
                    }
                    other => {
                        return Err(ZekkenError::type_error(
                            "Invalid member access",
                            "array/object",
                            value_type_name(other),
                            location.line,
                            location.column,
                        ));
                    }
                };
                *get_reg_mut(&mut regs, *dst) = value;
            }
            Inst::Binary { dst, left, right, op, location } => {
                let l = get_reg(&regs, *left);
                let r = get_reg(&regs, *right);
                let out = match (l, r, op) {
                    (Value::Int(li), Value::Int(ri), BinaryOpCode::Add) => Value::Int(li + ri),
                    (Value::Int(li), Value::Int(ri), BinaryOpCode::Sub) => Value::Int(li - ri),
                    (Value::Int(li), Value::Int(ri), BinaryOpCode::Mul) => Value::Int(li * ri),
                    (Value::Int(_), Value::Int(0), BinaryOpCode::Div) => {
                        return Err(ZekkenError::runtime(
                            "Division by zero",
                            location.line,
                            location.column,
                            Some("division by zero"),
                        ));
                    }
                    (Value::Int(li), Value::Int(ri), BinaryOpCode::Div) => Value::Int(li / ri),
                    (Value::Int(_), Value::Int(0), BinaryOpCode::Mod) => {
                        return Err(ZekkenError::runtime(
                            "Modulo by zero",
                            location.line,
                            location.column,
                            Some("modulo by zero"),
                        ));
                    }
                    (Value::Int(li), Value::Int(ri), BinaryOpCode::Mod) => Value::Int(li % ri),
                    (Value::Int(li), Value::Int(ri), BinaryOpCode::Eq) => Value::Boolean(li == ri),
                    (Value::Int(li), Value::Int(ri), BinaryOpCode::Ne) => Value::Boolean(li != ri),
                    (Value::Int(li), Value::Int(ri), BinaryOpCode::Lt) => Value::Boolean(li < ri),
                    (Value::Int(li), Value::Int(ri), BinaryOpCode::Gt) => Value::Boolean(li > ri),
                    (Value::Int(li), Value::Int(ri), BinaryOpCode::Le) => Value::Boolean(li <= ri),
                    (Value::Int(li), Value::Int(ri), BinaryOpCode::Ge) => Value::Boolean(li >= ri),

                    (Value::Float(lf), Value::Float(rf), BinaryOpCode::Add) => Value::Float(lf + rf),
                    (Value::Float(lf), Value::Float(rf), BinaryOpCode::Sub) => Value::Float(lf - rf),
                    (Value::Float(lf), Value::Float(rf), BinaryOpCode::Mul) => Value::Float(lf * rf),
                    (Value::Float(_), Value::Float(rf), BinaryOpCode::Div) if *rf == 0.0 => {
                        return Err(ZekkenError::runtime(
                            "Division by zero",
                            location.line,
                            location.column,
                            Some("division by zero"),
                        ));
                    }
                    (Value::Float(lf), Value::Float(rf), BinaryOpCode::Div) => Value::Float(lf / rf),
                    (Value::Float(lf), Value::Float(rf), BinaryOpCode::Eq) => Value::Boolean(lf == rf),
                    (Value::Float(lf), Value::Float(rf), BinaryOpCode::Ne) => Value::Boolean(lf != rf),
                    (Value::Float(lf), Value::Float(rf), BinaryOpCode::Lt) => Value::Boolean(lf < rf),
                    (Value::Float(lf), Value::Float(rf), BinaryOpCode::Gt) => Value::Boolean(lf > rf),
                    (Value::Float(lf), Value::Float(rf), BinaryOpCode::Le) => Value::Boolean(lf <= rf),
                    (Value::Float(lf), Value::Float(rf), BinaryOpCode::Ge) => Value::Boolean(lf >= rf),

                    (Value::Int(li), Value::Float(rf), BinaryOpCode::Add) => Value::Float(*li as f64 + rf),
                    (Value::Int(li), Value::Float(rf), BinaryOpCode::Sub) => Value::Float(*li as f64 - rf),
                    (Value::Int(li), Value::Float(rf), BinaryOpCode::Mul) => Value::Float(*li as f64 * rf),
                    (Value::Int(_), Value::Float(rf), BinaryOpCode::Div) if *rf == 0.0 => {
                        return Err(ZekkenError::runtime(
                            "Division by zero",
                            location.line,
                            location.column,
                            Some("division by zero"),
                        ));
                    }
                    (Value::Int(li), Value::Float(rf), BinaryOpCode::Div) => Value::Float(*li as f64 / rf),
                    (Value::Int(li), Value::Float(rf), BinaryOpCode::Eq) => Value::Boolean((*li as f64) == *rf),
                    (Value::Int(li), Value::Float(rf), BinaryOpCode::Ne) => Value::Boolean((*li as f64) != *rf),
                    (Value::Int(li), Value::Float(rf), BinaryOpCode::Lt) => Value::Boolean((*li as f64) < *rf),
                    (Value::Int(li), Value::Float(rf), BinaryOpCode::Gt) => Value::Boolean((*li as f64) > *rf),
                    (Value::Int(li), Value::Float(rf), BinaryOpCode::Le) => Value::Boolean((*li as f64) <= *rf),
                    (Value::Int(li), Value::Float(rf), BinaryOpCode::Ge) => Value::Boolean((*li as f64) >= *rf),

                    (Value::Float(lf), Value::Int(ri), BinaryOpCode::Add) => Value::Float(lf + *ri as f64),
                    (Value::Float(lf), Value::Int(ri), BinaryOpCode::Sub) => Value::Float(lf - *ri as f64),
                    (Value::Float(lf), Value::Int(ri), BinaryOpCode::Mul) => Value::Float(lf * *ri as f64),
                    (Value::Float(_), Value::Int(0), BinaryOpCode::Div) => {
                        return Err(ZekkenError::runtime(
                            "Division by zero",
                            location.line,
                            location.column,
                            Some("division by zero"),
                        ));
                    }
                    (Value::Float(lf), Value::Int(ri), BinaryOpCode::Div) => Value::Float(lf / *ri as f64),
                    (Value::Float(lf), Value::Int(ri), BinaryOpCode::Eq) => Value::Boolean(*lf == *ri as f64),
                    (Value::Float(lf), Value::Int(ri), BinaryOpCode::Ne) => Value::Boolean(*lf != *ri as f64),
                    (Value::Float(lf), Value::Int(ri), BinaryOpCode::Lt) => Value::Boolean(*lf < *ri as f64),
                    (Value::Float(lf), Value::Int(ri), BinaryOpCode::Gt) => Value::Boolean(*lf > *ri as f64),
                    (Value::Float(lf), Value::Int(ri), BinaryOpCode::Le) => Value::Boolean(*lf <= *ri as f64),
                    (Value::Float(lf), Value::Int(ri), BinaryOpCode::Ge) => Value::Boolean(*lf >= *ri as f64),

                    _ => eval_binary_opcode(l, r, *op, location)?,
                };
                *get_reg_mut(&mut regs, *dst) = out;
            }
            Inst::CallMath { dst, method, argc, args, location } => {
                let mut call_args = Vec::with_capacity(*argc as usize);
                for arg in args.iter().take(*argc as usize) {
                    call_args.push(clone_value_hot(get_reg(&regs, *arg)));
                }
                let out = method.eval(&call_args, location)?;
                *get_reg_mut(&mut regs, *dst) = out;
            }
            Inst::CallFs { dst, method, argc, args, location } => {
                let mut call_args = Vec::with_capacity(*argc as usize);
                for arg in args.iter().take(*argc as usize) {
                    call_args.push(clone_value_hot(get_reg(&regs, *arg)));
                }
                let out = method.eval(call_args, env, location)?;
                *get_reg_mut(&mut regs, *dst) = out;
            }
            Inst::CallOs { dst, method, argc, args, location } => {
                let mut call_args = Vec::with_capacity(*argc as usize);
                for arg in args.iter().take(*argc as usize) {
                    call_args.push(clone_value_hot(get_reg(&regs, *arg)));
                }
                let out = method.eval(call_args, env, location)?;
                *get_reg_mut(&mut regs, *dst) = out;
            }
            Inst::CallPath { dst, method, argc, args, location } => {
                let mut call_args = Vec::with_capacity(*argc as usize);
                for arg in args.iter().take(*argc as usize) {
                    call_args.push(clone_value_hot(get_reg(&regs, *arg)));
                }
                let out = method.eval(call_args, env, location)?;
                *get_reg_mut(&mut regs, *dst) = out;
            }
            Inst::CallEncoding { dst, method, argc, args, location } => {
                let mut call_args = Vec::with_capacity(*argc as usize);
                for arg in args.iter().take(*argc as usize) {
                    call_args.push(clone_value_hot(get_reg(&regs, *arg)));
                }
                let out = method.eval(call_args, env, location)?;
                *get_reg_mut(&mut regs, *dst) = out;
            }
            Inst::CallHttp { dst, method, argc, args, location } => {
                let mut call_args = Vec::with_capacity(*argc as usize);
                for arg in args.iter().take(*argc as usize) {
                    call_args.push(clone_value_hot(get_reg(&regs, *arg)));
                }
                let out = method.eval(call_args, env, location)?;
                *get_reg_mut(&mut regs, *dst) = out;
            }
            Inst::EvalExprNative { dst, expr } => {
                *get_reg_mut(&mut regs, *dst) = super::eval_expr_native(expr, env)?;
            }
            Inst::ExecStmtNative { stmt } => {
                if let Some(v) = super::eval_stmt_native(stmt, env)? {
                    last_value = Some(v);
                }
            }
            Inst::DeclareVar { name, ty, constant, src, location } => {
                let value = clone_value_hot(get_reg(&regs, *src));
                if !check_value_type(&value, ty) {
                    return Err(ZekkenError::type_error(
                        &format!("Type mismatch in variable declaration '{}'", name),
                        &format!("{:?}", ty),
                        value_type_name(&value),
                        location.line,
                        location.column,
                    ));
                }
                env.declare_ref_typed(name, value, *ty, *constant);
            }
            Inst::DeclareFunc { func } => {
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
            }
            Inst::DeclareLambda { lambda } => {
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
            }
            Inst::DeclareObject { object } => {
                let mut map = HashMap::new();
                let mut keys = Vec::with_capacity(object.properties.len());
                for prop in &object.properties {
                    let value = super::eval_expr_native(&prop.value, env)?;
                    keys.push(Value::String(prop.key.clone()));
                    map.insert(prop.key.clone(), value);
                }
                map.insert("__keys__".to_string(), Value::Array(keys));
                env.declare(object.ident.clone(), Value::Object(map), false);
            }
            Inst::AssignIdent { dst, name, src, location } => {
                let value = clone_value_hot(get_reg(&regs, *src));
                let expected = env.lookup_type(name).unwrap_or(DataType::Any);
                if expected != DataType::Any && !check_value_type(&value, &expected) {
                    return Err(ZekkenError::type_error(
                        &format!("Type mismatch in assignment to '{}'", name),
                        &format!("{:?}", expected),
                        value_type_name(&value),
                        location.line,
                        location.column,
                    ));
                }
                env.assign(name, value.clone()).map_err(|e| {
                    ZekkenError::runtime(&e, location.line, location.column, None)
                })?;
                *get_reg_mut(&mut regs, *dst) = value;
            }
            Inst::StoreIndexIdent { dst, name, index, src, location } => {
                let idx_value = clone_value_hot(get_reg(&regs, *index));
                let src_value = clone_value_hot(get_reg(&regs, *src));
                let slot = env.lookup_mut_assignable(name).map_err(|e| {
                    ZekkenError::runtime(&e, location.line, location.column, None)
                })?;
                match slot {
                    Value::Array(arr) => {
                        let idx = value_to_non_negative_index(&idx_value).ok_or_else(|| {
                            ZekkenError::type_error(
                                "Invalid array index",
                                "non-negative int",
                                value_type_name(&idx_value),
                                location.line,
                                location.column,
                            )
                        })?;
                        if idx >= arr.len() {
                            return Err(ZekkenError::runtime(
                                &format!("Array index out of bounds: {}", idx),
                                location.line,
                                location.column,
                                None,
                            ));
                        }
                        arr[idx] = src_value.clone();
                    }
                    Value::Object(map) => {
                        let key = match &idx_value {
                            Value::String(s) => s.clone(),
                            Value::Int(i) => i.to_string(),
                            Value::Float(f) => f.to_string(),
                            _ => {
                                return Err(ZekkenError::type_error(
                                    "Invalid object key type",
                                    "string/int/float",
                                    value_type_name(&idx_value),
                                    location.line,
                                    location.column,
                                ))
                            }
                        };
                        map.insert(key, src_value.clone());
                    }
                    other => {
                        return Err(ZekkenError::type_error(
                            "Invalid assignment target",
                            "array/object",
                            value_type_name(other),
                            location.line,
                            location.column,
                        ));
                    }
                }
                *get_reg_mut(&mut regs, *dst) = src_value;
            }
            Inst::Jump { target } => {
                ip = *target;
                continue;
            }
            Inst::JumpIfFalse { cond, target, location } => {
                match get_reg(&regs, *cond) {
                    Value::Boolean(false) => {
                        ip = *target;
                        continue;
                    }
                    Value::Boolean(true) => {}
                    other => {
                        return Err(ZekkenError::type_error(
                            "Condition must be a boolean",
                            "bool",
                            value_type_name(&other),
                            location.line,
                            location.column,
                        ))
                    }
                }
            }
            Inst::JumpIfCmpFalse { left, right, op, target, location } => {
                let cond_true = match op {
                    BinaryOpCode::Eq => compare_values(get_reg(&regs, *left), get_reg(&regs, *right)),
                    BinaryOpCode::Ne => !compare_values(get_reg(&regs, *left), get_reg(&regs, *right)),
                    BinaryOpCode::Lt | BinaryOpCode::Gt | BinaryOpCode::Le | BinaryOpCode::Ge => {
                        let l = match get_reg(&regs, *left) {
                            Value::Int(v) => *v as f64,
                            Value::Float(v) => *v,
                            other => {
                                return Err(ZekkenError::type_error(
                                    "Condition must be a boolean",
                                    "bool",
                                    value_type_name(other),
                                    location.line,
                                    location.column,
                                ))
                            }
                        };
                        let r = match get_reg(&regs, *right) {
                            Value::Int(v) => *v as f64,
                            Value::Float(v) => *v,
                            other => {
                                return Err(ZekkenError::type_error(
                                    "Condition must be a boolean",
                                    "bool",
                                    value_type_name(other),
                                    location.line,
                                    location.column,
                                ))
                            }
                        };
                        match op {
                            BinaryOpCode::Lt => l < r,
                            BinaryOpCode::Gt => l > r,
                            BinaryOpCode::Le => l <= r,
                            BinaryOpCode::Ge => l >= r,
                            _ => unreachable!(),
                        }
                    }
                    _ => unreachable!(),
                };

                if !cond_true {
                    ip = *target;
                    continue;
                }
            }
            Inst::SetLast { src } => {
                last_value = Some(clone_value_hot(get_reg(&regs, *src)));
            }
            Inst::Return { src } => {
                return Ok(Some(clone_value_hot(get_reg(&regs, *src))));
            }
            Inst::AddIntAssignIdent { dst, name, delta, location } => {
                let slot = env.lookup_mut_assignable(name).map_err(|e| {
                    ZekkenError::runtime(&e, location.line, location.column, None)
                })?;
                match slot {
                    Value::Int(i) => {
                        *i += *delta;
                        *get_reg_mut(&mut regs, *dst) = Value::Int(*i);
                    }
                    Value::Float(f) => {
                        *f += *delta as f64;
                        *get_reg_mut(&mut regs, *dst) = Value::Float(*f);
                    }
                    other => {
                        return Err(ZekkenError::type_error(
                            "Invalid assignment target for integer increment",
                            "int or float",
                            value_type_name(other),
                            location.line,
                            location.column,
                        ));
                    }
                }
            }
        }
        ip += 1;
    }

    Ok(last_value)
}
