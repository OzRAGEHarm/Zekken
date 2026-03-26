use crate::ast::Location;
use crate::environment::Value;
use crate::errors::ZekkenError;

#[derive(Clone, Copy)]
pub enum MathOpCode {
    Sin,
    Cos,
    Tan,
    Sqrt,
    Abs,
    Pow,
    Log,
    Exp,
    Floor,
    Ceil,
    Round,
    Min,
    Max,
    Clamp,
    Atan2,
}

impl MathOpCode {
    #[inline]
    pub fn from_method(name: &str) -> Option<Self> {
        match name {
            "sin" => Some(Self::Sin),
            "cos" => Some(Self::Cos),
            "tan" => Some(Self::Tan),
            "sqrt" => Some(Self::Sqrt),
            "abs" => Some(Self::Abs),
            "pow" => Some(Self::Pow),
            "log" => Some(Self::Log),
            "exp" => Some(Self::Exp),
            "floor" => Some(Self::Floor),
            "ceil" => Some(Self::Ceil),
            "round" => Some(Self::Round),
            "min" => Some(Self::Min),
            "max" => Some(Self::Max),
            "clamp" => Some(Self::Clamp),
            "atan2" => Some(Self::Atan2),
            _ => None,
        }
    }

    pub fn eval(self, args: &[Value], location: &Location) -> Result<Value, ZekkenError> {
        match self {
            Self::Sin => {
                require_argc(args, 1, location)?;
                Ok(Value::Float(as_num(&args[0], location)?.sin()))
            }
            Self::Cos => {
                require_argc(args, 1, location)?;
                Ok(Value::Float(as_num(&args[0], location)?.cos()))
            }
            Self::Tan => {
                require_argc(args, 1, location)?;
                Ok(Value::Float(as_num(&args[0], location)?.tan()))
            }
            Self::Sqrt => {
                require_argc(args, 1, location)?;
                Ok(Value::Float(as_num(&args[0], location)?.sqrt()))
            }
            Self::Abs => {
                require_argc(args, 1, location)?;
                Ok(Value::Float(as_num(&args[0], location)?.abs()))
            }
            Self::Pow => {
                require_argc(args, 2, location)?;
                Ok(Value::Float(as_num(&args[0], location)?.powf(as_num(&args[1], location)?)))
            }
            Self::Log => {
                if args.is_empty() || args.len() > 2 {
                    return Err(ZekkenError::runtime(
                        "Expected 1 or 2 arguments",
                        location.line,
                        location.column,
                        Some("argument mismatch"),
                    ));
                }
                let n = as_num(&args[0], location)?;
                if args.len() == 2 {
                    Ok(Value::Float(n.log(as_num(&args[1], location)?)))
                } else {
                    Ok(Value::Float(n.ln()))
                }
            }
            Self::Exp => {
                require_argc(args, 1, location)?;
                Ok(Value::Float(as_num(&args[0], location)?.exp()))
            }
            Self::Floor => {
                require_argc(args, 1, location)?;
                Ok(Value::Float(as_num(&args[0], location)?.floor()))
            }
            Self::Ceil => {
                require_argc(args, 1, location)?;
                Ok(Value::Float(as_num(&args[0], location)?.ceil()))
            }
            Self::Round => {
                require_argc(args, 1, location)?;
                Ok(Value::Float(as_num(&args[0], location)?.round()))
            }
            Self::Min => {
                require_argc(args, 2, location)?;
                Ok(Value::Float(as_num(&args[0], location)?.min(as_num(&args[1], location)?)))
            }
            Self::Max => {
                require_argc(args, 2, location)?;
                Ok(Value::Float(as_num(&args[0], location)?.max(as_num(&args[1], location)?)))
            }
            Self::Clamp => {
                require_argc(args, 3, location)?;
                let x = as_num(&args[0], location)?;
                let min = as_num(&args[1], location)?;
                let max = as_num(&args[2], location)?;
                Ok(Value::Float(x.max(min).min(max)))
            }
            Self::Atan2 => {
                require_argc(args, 2, location)?;
                let y = as_num(&args[0], location)?;
                let x = as_num(&args[1], location)?;
                Ok(Value::Float(y.atan2(x)))
            }
        }
    }
}

#[inline]
fn require_argc(args: &[Value], expected: usize, location: &Location) -> Result<(), ZekkenError> {
    if args.len() == expected {
        return Ok(());
    }
    Err(ZekkenError::runtime(
        &format!("Expected {} argument{}", expected, if expected == 1 { "" } else { "s" }),
        location.line,
        location.column,
        Some("argument mismatch"),
    ))
}

#[inline]
fn as_num(value: &Value, location: &Location) -> Result<f64, ZekkenError> {
    match value {
        Value::Int(i) => Ok(*i as f64),
        Value::Float(f) => Ok(*f),
        _ => Err(ZekkenError::type_error(
            "Expected number",
            "number",
            value_type_name(value),
            location.line,
            location.column,
        )),
    }
}

#[inline]
fn value_type_name(value: &Value) -> &'static str {
    match value {
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
