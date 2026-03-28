use crate::environment::{Environment, Value};
use crate::ast::{*};
use crate::lexer::{*};
use hashbrown::HashMap;
use std::f64::consts::{PI, E};
#[cfg(not(target_arch = "wasm32"))]
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_arch = "wasm32")]
fn random_unit() -> Result<f64, String> {
    // WASM does not reliably support SystemTime/OS entropy in all environments.
    // Use the host-provided JS RNG for the demo/runtime.
    Ok(js_sys::Math::random())
}

#[cfg(not(target_arch = "wasm32"))]
fn random_unit() -> Result<f64, String> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("random time error: {}", e))?;
    let nanos = now.subsec_nanos() as f64;
    Ok(nanos / 1_000_000_000.0)
}

pub fn register(env: &mut Environment) -> Result<(), String> {
    let mut math_obj = HashMap::new();

    fn create_param(name: &str, type_: DataType) -> Param {
        Param {
            ident: name.to_string(),
            type_,
            default_value: None,
            location: Location { line: 0, column: 0 }
        }
    }

    // Constants
    math_obj.insert("PI".to_string(), Value::Float(PI));
    math_obj.insert("E".to_string(), Value::Float(E));
    math_obj.insert("I".to_string(), Value::Complex { real: 0.0, imag: 1.0 });

    // Basic Math Functions
    use std::sync::Arc;
    math_obj.insert("sqrt".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 1 {
            return Err("sqrt expects exactly one argument".to_string());
        }
        match &args[0] {
            Value::Int(x) => Ok(Value::Float(((*x) as f64).sqrt())),
            Value::Float(x) => Ok(Value::Float((*x).sqrt())),
            _ => Err("sqrt expects a numeric argument".to_string()),
        }
    })));

    math_obj.insert("pow".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 2 {
            return Err("pow expects exactly two arguments".to_string());
        }
        let base = match &args[0] {
            Value::Int(x) => (*x) as f64,
            Value::Float(x) => *x,
            _ => return Err("pow expects numeric arguments".to_string()),
        };
        let exp = match &args[1] {
            Value::Int(x) => (*x) as f64,
            Value::Float(x) => *x,
            _ => return Err("pow expects numeric arguments".to_string()),
        };
        Ok(Value::Float(base.powf(exp)))
    })));

    math_obj.insert("abs".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 1 {
            return Err("abs expects exactly one argument".to_string());
        }
        match &args[0] {
            Value::Int(x) => Ok(Value::Int((*x).abs())),
            Value::Float(x) => Ok(Value::Float((*x).abs())),
            _ => Err("abs expects a numeric argument".to_string()),
        }
    })));

    // Trigonometric Functions
    math_obj.insert("sin".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 1 {
            return Err("sin expects exactly one argument".to_string());
        }
        match &args[0] {
            Value::Int(x) => Ok(Value::Float(((*x) as f64).sin())),
            Value::Float(x) => Ok(Value::Float((*x).sin())),
            _ => Err("sin expects a numeric argument".to_string()),
        }
    })));

    math_obj.insert("cos".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 1 {
            return Err("cos expects exactly one argument".to_string());
        }
        match &args[0] {
            Value::Int(x) => Ok(Value::Float(((*x) as f64).cos())),
            Value::Float(x) => Ok(Value::Float((*x).cos())),
            _ => Err("cos expects a numeric argument".to_string()),
        }
    })));

    math_obj.insert("tan".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 1 {
            return Err("tan expects exactly one argument".to_string());
        }
        match &args[0] {
            Value::Int(x) => Ok(Value::Float(((*x) as f64).tan())),
            Value::Float(x) => Ok(Value::Float((*x).tan())),
            _ => Err("tan expects a numeric argument".to_string()),
        }
    })));

    math_obj.insert("log".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.is_empty() || args.len() > 2 {
            return Err("log expects one or two numeric arguments".to_string());
        }
        let x = match &args[0] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("log expects numeric arguments".to_string()),
        };
        if x <= 0.0 {
            return Err("log input must be greater than 0".to_string());
        }
        if args.len() == 1 {
            return Ok(Value::Float(x.ln()));
        }
        let base = match &args[1] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("log expects numeric arguments".to_string()),
        };
        if base <= 0.0 || (base - 1.0).abs() < f64::EPSILON {
            return Err("log base must be > 0 and != 1".to_string());
        }
        Ok(Value::Float(x.log(base)))
    })));

    math_obj.insert("exp".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 1 {
            return Err("exp expects exactly one numeric argument".to_string());
        }
        let x = match &args[0] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("exp expects a numeric argument".to_string()),
        };
        Ok(Value::Float(x.exp()))
    })));

    math_obj.insert("floor".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 1 {
            return Err("floor expects exactly one numeric argument".to_string());
        }
        let x = match &args[0] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("floor expects a numeric argument".to_string()),
        };
        Ok(Value::Float(x.floor()))
    })));

    math_obj.insert("ceil".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 1 {
            return Err("ceil expects exactly one numeric argument".to_string());
        }
        let x = match &args[0] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("ceil expects a numeric argument".to_string()),
        };
        Ok(Value::Float(x.ceil()))
    })));

    math_obj.insert("round".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 1 {
            return Err("round expects exactly one numeric argument".to_string());
        }
        let x = match &args[0] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("round expects a numeric argument".to_string()),
        };
        Ok(Value::Float(x.round()))
    })));

    math_obj.insert("min".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 2 {
            return Err("min expects exactly two numeric arguments".to_string());
        }
        let a = match &args[0] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("min expects numeric arguments".to_string()),
        };
        let b = match &args[1] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("min expects numeric arguments".to_string()),
        };
        Ok(Value::Float(a.min(b)))
    })));

    math_obj.insert("max".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 2 {
            return Err("max expects exactly two numeric arguments".to_string());
        }
        let a = match &args[0] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("max expects numeric arguments".to_string()),
        };
        let b = match &args[1] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("max expects numeric arguments".to_string()),
        };
        Ok(Value::Float(a.max(b)))
    })));

    math_obj.insert("clamp".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 3 {
            return Err("clamp expects exactly three numeric arguments (x, min, max)".to_string());
        }
        let x = match &args[0] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("clamp expects numeric arguments".to_string()),
        };
        let min_v = match &args[1] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("clamp expects numeric arguments".to_string()),
        };
        let max_v = match &args[2] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("clamp expects numeric arguments".to_string()),
        };
        if min_v > max_v {
            return Err("clamp expects min <= max".to_string());
        }
        Ok(Value::Float(x.clamp(min_v, max_v)))
    })));

    math_obj.insert("random".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if !args.is_empty() {
            return Err("random expects no arguments".to_string());
        }
        Ok(Value::Float(random_unit()?))
    })));

    math_obj.insert("rand_int".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 2 {
            return Err("rand_int expects exactly two integer arguments (min, max)".to_string());
        }
        let min = match &args[0] {
            Value::Int(v) => *v,
            _ => return Err("rand_int expects integer arguments".to_string()),
        };
        let max = match &args[1] {
            Value::Int(v) => *v,
            _ => return Err("rand_int expects integer arguments".to_string()),
        };
        if min > max {
            return Err("rand_int expects min <= max".to_string());
        }
        let range = (max - min + 1) as f64;
        let n = (random_unit()? * range).floor() as i64 + min;
        Ok(Value::Int(n.clamp(min, max)))
    })));

    math_obj.insert("rand_choice".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 1 {
            return Err("rand_choice expects exactly one array argument".to_string());
        }
        let items = match &args[0] {
            Value::Array(v) => v,
            _ => return Err("rand_choice expects an array argument".to_string()),
        };
        if items.is_empty() {
            return Err("rand_choice expects a non-empty array".to_string());
        }
        let idx = (random_unit()? * items.len() as f64).floor() as usize;
        let safe_idx = idx.min(items.len() - 1);
        Ok(items[safe_idx].clone())
    })));

    math_obj.insert("shuffle".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 1 {
            return Err("shuffle expects exactly one array argument".to_string());
        }
        let mut items = match &args[0] {
            Value::Array(v) => v.clone(),
            _ => return Err("shuffle expects an array argument".to_string()),
        };
        if items.len() <= 1 {
            return Ok(Value::Array(items));
        }
        for i in (1..items.len()).rev() {
            let j = (random_unit()? * ((i + 1) as f64)).floor() as usize;
            items.swap(i, j.min(i));
        }
        Ok(Value::Array(items))
    })));

    math_obj.insert("atan2".to_string(), Value::NativeFunction(Arc::new(|args: Vec<Value>| {
        if args.len() != 2 {
            return Err("atan2 expects exactly two numeric arguments".to_string());
        }
        let y = match &args[0] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("atan2 expects numeric arguments".to_string()),
        };
        let x = match &args[1] {
            Value::Int(v) => *v as f64,
            Value::Float(v) => *v,
            _ => return Err("atan2 expects numeric arguments".to_string()),
        };
        Ok(Value::Float(y.atan2(x)))
    })));

    // Vector Operations
    math_obj.insert("vector".to_string(), Value::NativeFunction(Arc::new(|args| {
        if args.len() != 1 {
            return Err("vector expects exactly one argument".to_string());
        }
        match &args[0] {
            Value::Array(arr) => {
                let mut vec_f64 = Vec::with_capacity(arr.len());
                for v in arr {
                    match v {
                        Value::Int(i) => vec_f64.push(*i as f64),
                        Value::Float(f) => vec_f64.push(*f),
                        _ => return Err("vector expects array elements to be numbers".to_string()),
                    }
                }
                Ok(Value::Array(vec_f64.into_iter().map(Value::Float).collect()))
            },
            _ => Err("vector expects an array argument".to_string()),
        }
    })));

    math_obj.insert("dot".to_string(), Value::NativeFunction(Arc::new(|args| {
        if args.len() != 2 {
            return Err("dot expects exactly two arguments".to_string());
        }
        let v1: Vec<f64> = match &args[0] {
            Value::Vector(v) => v.clone(),
            Value::Array(v) => {
                let mut vec_f64 = Vec::with_capacity(v.len());
                for val in v {
                    match val {
                        Value::Int(i) => vec_f64.push(*i as f64),
                        Value::Float(f) => vec_f64.push(*f),
                        _ => return Err("dot: array elements must be numbers".to_string()),
                    }
                }
                vec_f64
            },
            _ => return Err("dot expects two vectors or arrays".to_string()),
        };
        let v2: Vec<f64> = match &args[1] {
            Value::Vector(v) => v.clone(),
            Value::Array(v) => {
                let mut vec_f64 = Vec::with_capacity(v.len());
                for val in v {
                    match val {
                        Value::Int(i) => vec_f64.push(*i as f64),
                        Value::Float(f) => vec_f64.push(*f),
                        _ => return Err("dot: array elements must be numbers".to_string()),
                    }
                }
                vec_f64
            },
            _ => return Err("dot expects two vectors or arrays".to_string()),
        };
        if v1.len() != v2.len() {
            return Err("dot: vectors must be the same length".to_string());
        }
        let mut sum = 0.0;
        for (a, b) in v1.iter().zip(v2.iter()) {
            sum += a * b;
        }
        Ok(Value::Float(sum))
    })));

    math_obj.insert("matrix".to_string(), Value::NativeFunction(Arc::new(|args| {
        if args.len() != 1 {
            return Err("matrix expects exactly one argument".to_string());
        }
        match &args[0] {
            Value::Array(rows) => {
                for row in rows {
                    match row {
                        Value::Array(cols) => {
                            for v in cols {
                                match v {
                                    Value::Int(_) | Value::Float(_) => {},
                                    _ => return Err("matrix expects all elements to be numbers".to_string()),
                                }
                            }
                        }
                        _ => return Err("matrix expects an array of arrays".to_string()),
                    }
                }
                Ok(Value::Array(rows.clone()))
            }
            _ => Err("matrix expects an array of arrays".to_string()),
        }
    })));
    
    // Matrix multiplication: matmul(a, b)
    math_obj.insert("matmul".to_string(), Value::NativeFunction(Arc::new(|args| {
        if args.len() != 2 {
            return Err("matmul expects exactly two arguments".to_string());
        }
        let a = match &args[0] {
            Value::Array(rows) => rows,
            _ => return Err("matmul expects both arguments to be matrices (array of arrays)".to_string()),
        };
        let b = match &args[1] {
            Value::Array(rows) => rows,
            _ => return Err("matmul expects both arguments to be matrices (array of arrays)".to_string()),
        };
    
        // Check dimensions
        let a_rows = a.len();
        let a_cols = match a.get(0) {
            Some(Value::Array(cols)) => cols.len(),
            _ => return Err("matmul: first matrix is empty or not a matrix".to_string()),
        };
        let b_rows = b.len();
        let b_cols = match b.get(0) {
            Some(Value::Array(cols)) => cols.len(),
            _ => return Err("matmul: second matrix is empty or not a matrix".to_string()),
        };
    
        if a_cols != b_rows {
            return Err("matmul: number of columns in first matrix must equal number of rows in second matrix".to_string());
        }
    
        // Perform multiplication
        let mut result = Vec::with_capacity(a_rows);
        for i in 0..a_rows {
            let mut row = Vec::with_capacity(b_cols);
            let a_row = match &a[i] {
                Value::Array(cols) => cols,
                _ => return Err("matmul: first matrix is not well-formed".to_string()),
            };
            for j in 0..b_cols {
                let mut sum = 0.0;
                for k in 0..a_cols {
                    let a_val = match &a_row[k] {
                        Value::Int(x) => *x as f64,
                        Value::Float(x) => *x,
                        _ => return Err("matmul: matrix elements must be numbers".to_string()),
                    };
                    let b_col = match &b[k] {
                        Value::Array(cols) => cols,
                        _ => return Err("matmul: second matrix is not well-formed".to_string()),
                    };
                    let b_val = match &b_col[j] {
                        Value::Int(x) => *x as f64,
                        Value::Float(x) => *x,
                        _ => return Err("matmul: matrix elements must be numbers".to_string()),
                    };
                    sum += a_val * b_val;
                }
                row.push(Value::Float(sum));
            }
            result.push(Value::Array(row));
        }
        Ok(Value::Array(result))
    })));

    // Register either full module or specific imports
    if let Some(Value::Array(methods)) = env.lookup("__IMPORT_METHODS__") {
        // Specific imports
        for method in methods {
            if let Value::String(name) = method {
                if let Some(value) = math_obj.get(&name) {
                    env.declare(name, value.clone(), true);
                } else {
                    return Err(format!("Math module error: '{}' not found", name));
                }
            }
        }
    } else {
        // Full module import
        env.declare("math".to_string(), Value::Object(math_obj), true);
    }

    Ok(())
}
