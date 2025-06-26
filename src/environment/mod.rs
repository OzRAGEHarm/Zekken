#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Write;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use crate::ast::*;
use serde_json::Value as JsonValue;

pub enum Value {
  Int(i64),
  Float(f64),
  String(String),
  Boolean(bool),
  Array(Vec<Value>),
  Object(HashMap<String, Value>),
  Function(FunctionValue),
  NativeFunction(Arc<dyn Fn(Vec<Value>) -> Result<Value, String> + Send + Sync + 'static>),
  Complex { real: f64, imag: f64 },
  Vector(Vec<f64>),
  Matrix(Vec<Vec<f64>>),
  Void,
}

impl std::fmt::Debug for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Value::Int(i) => write!(f, "Int({})", i),
            Value::Float(fl) => write!(f, "Float({})", fl),
            Value::String(s) => write!(f, "String({:?})", s),
            Value::Boolean(b) => write!(f, "Boolean({})", b),
            Value::Array(arr) => write!(f, "Array({:?})", arr),
            Value::Object(obj) => write!(f, "Object({:?})", obj),
            Value::Function(_) => write!(f, "Function(...)"),
            Value::NativeFunction(_) => write!(f, "NativeFunction(...)"),
            Value::Complex { real, imag } => write!(f, "Complex {{ real: {}, imag: {} }}", real, imag),
            Value::Vector(v) => write!(f, "Vector({:?})", v),
            Value::Matrix(m) => write!(f, "Matrix({:?})", m),
            Value::Void => write!(f, "Void"),
        }
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        match self {
            Value::Int(i) => Value::Int(*i),
            Value::Float(f) => Value::Float(*f),
            Value::String(s) => Value::String(s.clone()),
            Value::Boolean(b) => Value::Boolean(*b),
            Value::Array(arr) => Value::Array(arr.clone()),
            Value::Object(obj) => Value::Object(obj.clone()),
            Value::Function(func) => Value::Function(func.clone()),
            Value::NativeFunction(f) => Value::NativeFunction(f.clone()),
            Value::Complex { real, imag } => Value::Complex { real: *real, imag: *imag },
            Value::Vector(v) => Value::Vector(v.clone()),
            Value::Matrix(m) => Value::Matrix(m.clone()),
            Value::Void => Value::Void,
        }
    }
}


impl Display for Value {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        self.fmt_with_indent(f, 0, false)
    }
}

impl Value {
    fn fmt_with_indent(&self, f: &mut Formatter, indent: usize, in_container: bool) -> fmt::Result {
        let indent_str = |n| "  ".repeat(n);
        match self {
            Value::Array(arr) => {
                if arr.is_empty() {
                    write!(f, "[]")
                } else {
                    writeln!(f, "[")?;
                    for (i, value) in arr.iter().enumerate() {
                        write!(f, "{}", indent_str(indent + 1))?;
                        value.fmt_with_indent(f, indent + 1, true)?;
                        if i < arr.len() - 1 {
                            writeln!(f, ",")?;
                        } else {
                            writeln!(f)?;
                        }
                    }
                    write!(f, "{}]", indent_str(indent))
                }
            },
            Value::Object(obj) => {
                let keys = obj.get("__keys__").and_then(|v| {
                    if let Value::Array(keys) = v {
                        Some(keys.clone())
                    } else {
                        None
                    }
                }).unwrap_or_else(Vec::new);

                if keys.is_empty() {
                    write!(f, "{{}}")
                } else {
                    writeln!(f, "{{")?;
                    let mut first = true;
                    for key_val in keys {
                        if let Value::String(key) = key_val {
                            if key == "__keys__" { continue; }
                            if let Some(value) = obj.get(&key) {
                                if !first {
                                    writeln!(f, ",")?;
                                }
                                write!(f, "{}\"{}\": ", indent_str(indent + 1), key)?;
                                value.fmt_with_indent(f, indent + 1, true)?;
                                first = false;
                            }
                        }
                    }
                    writeln!(f)?;
                    write!(f, "{}}}", indent_str(indent))
                }
            },
            Value::String(s) => {
                if in_container {
                    write!(f, "\"{}\"", s)
                } else {
                    write!(f, "{}", s)
                }
            },
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::Boolean(b) => write!(f, "{}", b),
            Value::Function(_) => write!(f, "<function>"),
            Value::NativeFunction(_) => write!(f, "<native function>"),
            Value::Complex { real, imag } => {
                if *imag >= 0.0 {
                    write!(f, "{} + {}i", real, imag)
                } else {
                    write!(f, "{} - {}i", real, imag.abs())
                }
            }
            Value::Vector(v) => {
                write!(f, "[")?;
                for (i, val) in v.iter().enumerate() {
                    if i > 0 { write!(f, ", ")? }
                    write!(f, "{}", val)?;
                }
                write!(f, "]")
            }
            Value::Matrix(m) => {
                writeln!(f, "[")?;
                for (i, row) in m.iter().enumerate() {
                    if i > 0 { write!(f, " ")? }
                    write!(f, "[")?;
                    for (j, val) in row.iter().enumerate() {
                        if j > 0 { write!(f, ", ")? }
                        write!(f, "{:>8.3}", val)?;
                    }
                    write!(f, "]")?;
                    if i < m.len() - 1 { writeln!(f)? }
                }
                write!(f, "\n{}]", indent_str(indent))
            }
            Value::Void => write!(f, "void"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct FunctionValue {
  pub params: Vec<Param>,
  pub body: Vec<Box<Content>>,
  //pub closure: Environment,
}

#[derive(Debug, Clone)]
pub struct Environment {
  pub parent: Option<Box<Environment>>,
  pub variables: HashMap<String, Value>,
  pub constants: HashMap<String, Value>,
}

pub fn json_to_zekken(val: &JsonValue) -> Value {
    match val {
        JsonValue::Null => Value::Void,
        JsonValue::Bool(b) => Value::Boolean(*b),
        JsonValue::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Int(i)
            } else if let Some(f) = n.as_f64() {
                Value::Float(f)
            } else {
                Value::Void
            }
        }
        JsonValue::String(s) => Value::String(s.clone()),
        JsonValue::Array(arr) => Value::Array(arr.iter().map(json_to_zekken).collect()),
        JsonValue::Object(obj) => {
            let mut map = std::collections::HashMap::new();
            let mut keys = Vec::new();
            for (k, v) in obj.iter() {
                keys.push(Value::String(k.clone()));
                map.insert(k.clone(), json_to_zekken(v));
            }
            map.insert("__keys__".to_string(), Value::Array(keys));
            Value::Object(map)
        }
    }
}

impl Environment {
  pub fn new() -> Self {
      let mut env = Environment {
          parent: None,
          variables: HashMap::new(),
          constants: HashMap::new(),
      };

      env.variables.insert(
        "println".to_string(),
        Value::NativeFunction(Arc::new(|args: Vec<Value>| -> Result<Value, String> {
            let mut stdout = std::io::stdout();

            if args.is_empty() {
                writeln!(stdout).map_err(|e| e.to_string())?;
                return Ok(Value::Void);
            }

            let return_value = args[0].clone();

            writeln!(stdout, "{}", return_value).map_err(|e| e.to_string())?;
            stdout.flush().map_err(|e| e.to_string())?;

            Ok(Value::Void)
        }))
      );

      env.declare(
        "input".to_string(), 
        Value::NativeFunction(Arc::new(|args| {
          use std::io::{Write, stdin, stdout};

          if args.is_empty() {
              return Err("Input requires a prompt string".to_string());
          }

          let mut stdout = stdout();

          write!(stdout, "{}", args[0]).map_err(|e| e.to_string())?;
          stdout.flush().map_err(|e| e.to_string())?;

          let mut input = String::new();
          stdin().read_line(&mut input).map_err(|e| e.to_string())?;

          let input = input.trim().to_string();

          Ok(Value::String(input))
      })), false);

      env.declare(
        "parse_json".to_string(),
        Value::NativeFunction(Arc::new(|args: Vec<Value>| -> Result<Value, String> {
            if let [Value::String(ref s)] = args.as_slice() {
                match serde_json::from_str::<JsonValue>(s) {
                    Ok(json) => Ok(json_to_zekken(&json)),
                    Err(e) => Err(format!("JSON parse error: {}", e)),
                }
            } else {
                Err("parse_json expects a single string argument".to_string())
            }
        })), true);

      env
  }

  pub fn new_with_parent(parent: Environment) -> Self {
      Environment {
          parent: Some(Box::new(parent)),
          variables: HashMap::new(),
          constants: HashMap::new(),
      }
  }

  pub fn declare(&mut self, name: String, value: Value, constant: bool) {
      if constant {
          self.constants.insert(name, value);
      } else {
          self.variables.insert(name, value);
      }
  }

  pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
      // First check if variable exists in current scope
      if self.variables.contains_key(name) {
          // Check if the variable is declared as constant
          if self.constants.contains_key(name) {
              return Err(format!("Cannot reassign constant '{}'", name));
          }
          self.variables.insert(name.to_string(), value);
          return Ok(());
      }

      // If not in current scope, try parent scope
      if let Some(ref mut parent) = self.parent {
          return parent.assign(name, value);
      }

      Err(format!("Variable '{}' not found", name))
  }

  pub fn lookup(&self, name: &str) -> Option<Value> {
      self.variables.get(name)
          .or_else(|| self.constants.get(name))
          .cloned()
          .or_else(|| self.parent.as_ref().and_then(|p| p.lookup(name)))
  }
}

impl From<IntLit> for Value {
  fn from(lit: IntLit) -> Self {
    Value::Int(lit.value)
  }
}

impl From<FloatLit> for Value {
  fn from(lit: FloatLit) -> Self {
    Value::Float(lit.value)
  }
}

impl From<StringLit> for Value {
  fn from(lit: StringLit) -> Self {
    Value::String(lit.value)
  }
}

impl From<BoolLit> for Value {
  fn from(lit: BoolLit) -> Self {
    Value::Boolean(lit.value)
  }
}

impl From<ArrayLit> for Value {
  fn from(lit: ArrayLit) -> Self {
    Value::Array(lit.elements.into_iter()
      .map(|e| match *e {
        Expr::IntLit(i) => Value::Int(i.value),
        Expr::FloatLit(f) => Value::Float(f.value),
        Expr::StringLit(s) => Value::String(s.value),
        Expr::BoolLit(b) => Value::Boolean(b.value),
        _ => Value::Void,
      })
      .collect())
  }
}

impl From<ComplexLit> for Value {
  fn from(lit: ComplexLit) -> Self {
      Value::Complex {
          real: lit.real,
          imag: lit.imag,
      }
  }
}

impl From<VectorLit> for Value {
  fn from(lit: VectorLit) -> Self {
      Value::Vector(lit.elements)
  }
}

impl From<MatrixLit> for Value {
  fn from(lit: MatrixLit) -> Self {
      Value::Matrix(lit.rows)
  }
}

impl Value {
    pub fn call_method(&self, method_name: &str, args: Vec<Value>, env: Option<&mut Environment>, variable_name: Option<&str>) -> Result<Value, String> {
        match self {
            Value::String(s) => Self::handle_string_method(s, method_name, args),
            Value::Array(arr) => Self::handle_array_method(arr, method_name, args, env, variable_name),
            Value::Object(obj) => {
                if let Some(Value::NativeFunction(func)) = obj.get(method_name) {
                    return (func)(args);
                }
                Self::handle_object_method(obj, method_name, args)
            }
            Value::Int(n) => Self::handle_int_method(*n, method_name, args),
            Value::Float(n) => Self::handle_float_method(*n, method_name, args),
            _ => Err(format!("Type '{}' does not support methods", self.type_name())),
        }
    }

    fn handle_array_method(arr: &Vec<Value>, method_name: &str, mut args: Vec<Value>, env: Option<&mut Environment>, variable_name: Option<&str>) -> Result<Value, String> {
        match method_name {
            "length" => Ok(Value::Int(arr.len() as i64)),
            "first" => {
                if let Some(first) = arr.first() {
                    Ok(first.clone())
                } else {
                    Err("Array is empty".to_string())
                }
            }
            "last" => {
                if let Some(last) = arr.last() {
                    Ok(last.clone())
                } else {
                    Err("Array is empty".to_string())
                }
            }
            "push" => {
                if args.len() != 1 {
                    return Err("push requires exactly one argument".to_string());
                }
                if let Some(env) = env {
                    if let Some(var_name) = variable_name {
                        let mut new_arr = arr.clone();
                        new_arr.push(args.remove(0));
                        env.assign(var_name, Value::Array(new_arr.clone()))
                            .map_err(|e| format!("Failed to update array: {}", e))?;
                        Ok(Value::Array(new_arr))
                    } else {
                        Err("push requires a variable name to update the original array".to_string())
                    }
                } else {
                    Err("push requires an environment to update the original array".to_string())
                }
            }
            "pop" => {
                let mut new_arr = arr.clone();
                if let Some(popped) = new_arr.pop() {
                    if let Some(env) = env {
                        if let Some(var_name) = variable_name {
                            env.assign(var_name, Value::Array(new_arr.clone()))
                                .map_err(|e| format!("Failed to update array: {}", e))?;
                        }
                    }
                    Ok(popped)
                } else {
                    Err("Array is empty".to_string())
                }
            }
            "join" => {
                if args.len() != 1 {
                    return Err("join requires one string argument".to_string());
                }
                let delim = match &args[0] {
                    Value::String(s) => s,
                    _ => return Err("join argument must be a string".to_string()),
                };
                let joined = arr.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(delim);
                Ok(Value::String(joined))
            }
            _ => Err(format!("Array method '{}' not supported", method_name)),
        }
    }

    fn handle_string_method(s: &String, method_name: &str, args: Vec<Value>) -> Result<Value, String> {
        match method_name {
            "length" => Ok(Value::Int(s.len() as i64)),
            "toUpper" => Ok(Value::String(s.to_uppercase())),
            "toLower" => Ok(Value::String(s.to_lowercase())),
            "trim" => Ok(Value::String(s.trim().to_string())),
            "split" => {
                if args.len() != 1 {
                    return Err("split requires one argument".to_string());
                }
                let delimiter = match &args[0] {
                    Value::String(delim) => delim,
                    _ => return Err("split argument must be a string".to_string()),
                };
                Ok(Value::Array(s.split(delimiter).map(|part| Value::String(part.to_string())).collect()))
            }
            _ => Err(format!("String method '{}' not supported", method_name)),
        }
    }

    fn handle_object_method(obj: &HashMap<String, Value>, method_name: &str, args: Vec<Value>) -> Result<Value, String> {
        match method_name {
            "keys" => {
                let keys_value = obj.get("__keys__").cloned();
                let keys = if let Some(Value::Array(keys)) = keys_value {
                    keys
                } else {
                    Vec::new()
                };
                
                let ordered_keys: Vec<Value> = keys.into_iter()
                    .filter_map(|key| {
                        if let Value::String(s) = key {
                            if s != "__keys__" {
                                Some(Value::String(s))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(Value::Array(ordered_keys))
            },
            "values" => {
                let keys_value = obj.get("__keys__").cloned();
                let keys = if let Some(Value::Array(keys)) = keys_value {
                    keys
                } else {
                    Vec::new()
                };

                let ordered_values: Vec<Value> = keys.into_iter()
                    .filter_map(|key| {
                        if let Value::String(s) = key {
                            if s != "__keys__" {
                                obj.get(&s).cloned()
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(Value::Array(ordered_values))
            },
            "entries" => {
                let keys_value = obj.get("__keys__").cloned();
                let keys = if let Some(Value::Array(keys)) = keys_value {
                    keys
                } else {
                    Vec::new()
                };

                let entries: Vec<Value> = keys.into_iter()
                    .filter_map(|key| {
                        if let Value::String(s) = key {
                            if s != "__keys__" {
                                obj.get(&s).map(|value| {
                                    Value::Array(vec![Value::String(s), value.clone()])
                                })
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    })
                    .collect();
                Ok(Value::Array(entries))
            },
            "hasKey" => {
                if args.len() != 1 {
                    return Err("hasKey requires one string argument".to_string());
                }
                if let Value::String(key) = &args[0] {
                    Ok(Value::Boolean(obj.contains_key(key)))
                } else {
                    Err("hasKey argument must be a string".to_string())
                }
            }
            "get" => {
                if args.len() != 2 {
                    return Err("get requires two arguments: key and default value".to_string());
                }
                if let Value::String(key) = &args[0] {
                    Ok(obj.get(key).cloned().unwrap_or_else(|| args[1].clone()))
                } else {
                    Err("get first argument must be a string".to_string())
                }
            }
            _ => Err(format!("Object method '{}' not supported", method_name)),
        }
    }
    
    fn handle_int_method(n: i64, method_name: &str, _args: Vec<Value>) -> Result<Value, String> {
        match method_name {
            "isEven" => Ok(Value::Boolean(n % 2 == 0)),
            "isOdd" => Ok(Value::Boolean(n % 2 != 0)),
            _ => Err(format!("Integer method '{}' not supported", method_name)),
        }
    }

    fn handle_float_method(n: f64, method_name: &str, _args: Vec<Value>) -> Result<Value, String> {
        match method_name {
            "round" => Ok(Value::Int(n.round() as i64)),
            "floor" => Ok(Value::Int(n.floor() as i64)),
            "ceil" => Ok(Value::Int(n.ceil() as i64)),
            "isEven" => Ok(Value::Boolean(n % 2.0 == 0.0)),
            "isOdd" => Ok(Value::Boolean(n % 2.0 != 0.0)),
            _ => Err(format!("Float method '{}' not supported", method_name)),
        }
    }

    fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Boolean(_) => "boolean",
            Value::Array(_) => "array",
            Value::Object(_) => "object",
            Value::NativeFunction(_) => "native function",
            Value::Function(_) => "function",
            Value::Complex { .. } => "complex",
            Value::Vector(_) => "vector",
            Value::Matrix(_) => "matrix",
            Value::Void => "void",
        }
    }
}