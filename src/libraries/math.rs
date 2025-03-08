use crate::environment::{Environment, Value};

pub fn register(env: &mut Environment) -> Result<(), String> {
    // Constants
    env.declare("PI".to_string(), Value::Float(std::f64::consts::PI), true);
    env.declare("E".to_string(), Value::Float(std::f64::consts::E), true);
    env.declare("I".to_string(), Value::Complex { real: 0.0, imag: 1.0 }, true);

    // Square root function
    env.declare(
        "sqrt".to_string(),
        Value::NativeFunction(|args| {
            if args.len() != 1 {
                return Err(format!("sqrt error: expected one argument, received {}", args.len()));
            }
            let num = args[0].as_float().ok_or("sqrt error: argument must be numeric")?;
            if num < 0.0 {
                return Err("sqrt error: cannot compute square root of negative number".to_string());
            }
            Ok(Value::Float(num.sqrt()))
        }),
        true,
    );

    // Complex number functions
    env.declare(
        "complex".to_string(),
        Value::NativeFunction(|args| {
            if args.len() != 2 {
                return Err(format!("complex error: expected two arguments (real, imag), received {}", args.len()));
            }
            let real = args[0].as_float().ok_or("complex error: first argument must be numeric")?;
            let imag = args[1].as_float().ok_or("complex error: second argument must be numeric")?;
            Ok(Value::Complex { real, imag })
        }),
        true,
    );

    // Vector operations
    env.declare(
        "vector".to_string(),
        Value::NativeFunction(|args| {
            let vec: Vec<f64> = args.iter()
                .map(|v| v.as_float().ok_or("vector error: all vector elements must be numeric"))
                .collect::<Result<_, _>>()?;
            Ok(Value::Vector(vec))
        }),
        true,
    );

    // Matrix operations
    env.declare(
        "matrix".to_string(),
        Value::NativeFunction(|args| {
            if args.is_empty() {
                return Err("matrix error: expected at least one row".to_string());
            }
            let mut matrix = Vec::new();
            for arg in args {
                if let Value::Vector(row) = arg {
                    matrix.push(row.clone());
                } else {
                    return Err("matrix error: each row must be a vector".to_string());
                }
            }
            Ok(Value::Matrix(matrix))
        }),
        true,
    );

    // Basic arithmetic functions
    env.declare(
        "abs".to_string(),
        Value::NativeFunction(|args| {
            if args.len() != 1 {
                return Err(format!("abs error: expected one argument, received {}", args.len()));
            }
            let num = args[0].as_float().ok_or("abs error: argument must be numeric")?;
            Ok(Value::Float(num.abs()))
        }),
        true,
    );

    Ok(())
}