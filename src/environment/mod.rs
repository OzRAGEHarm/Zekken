#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Write;
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use crate::ast::*;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Method {
    Length,
    ToUpper,
    ToLower,
    Trim,
    Split,
    Push,
    Pop,
    Join,
    First,
    Last,
    Keys,
    Values,
    Entries,
    Round,
    Floor,
    Ceil,
    ToString,
}

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
      match self {
          Value::Int(i) => write!(f, "{}", i),
          Value::Float(fl) => {
              let s = format!("{}", fl);
              if !s.contains('.') {
                  write!(f, "{}.0", fl)
              } else {
                  write!(f, "{}", fl)
              }
          },
          Value::String(s) => write!(f, "{}", s),
          Value::Boolean(b) => write!(f, "{}", b),
          Value::Array(arr) => {
              write!(f, "[")?;
              let mut first = true;
              for value in arr {
                  if !first {
                      write!(f, ", ")?;
                  }
                  match value {
                      Value::String(s) => write!(f, "\"{}\"", s)?,
                      _ => write!(f, "{}", value)?,
                  }
                  first = false;
              }
              write!(f, "]")
          },
          Value::Object(obj) => {
              write!(f, "{{")?;
              let mut first = true;

              // Get keys order from __keys__ property if present
              let keys_order = if let Some(Value::Array(keys)) = obj.get("__keys__") {
                  keys.iter().filter_map(|k| {
                      if let Value::String(s) = k {
                          Some(s)
                      } else {
                          None
                      }
                  }).collect::<Vec<&String>>()
              } else {
                  // Fallback to keys in arbitrary order
                  obj.keys().filter(|k| *k != "__keys__").collect()
              };

              for key in keys_order {
                  if !first {
                      write!(f, ", ")?;
                  }
                  if let Some(value) = obj.get(key) {
                      match value {
                          Value::String(s) => write!(f, "\"{}\": \"{}\"", key, s)?,
                          _ => write!(f, "\"{}\": {}", key, value)?,
                      }
                  }
                  first = false;
              }
              write!(f, "}}")
          },
          Value::Function(_) => write!(f, "<function>"),
          Value::NativeFunction(_) => write!(f, "<native function>"),
          Value::Complex { real, imag } => {
            if *imag >= 0.0 {
                write!(f, "{} + {}i", real, imag)
            } else {
                write!(f, "{} - {}i", real, imag.abs())
            }
          },
          Value::Vector(v) => {
              write!(f, "[")?;
              for (i, val) in v.iter().enumerate() {
                  if i > 0 { write!(f, ", ")? }
                  write!(f, "{}", val)?;
              }
              write!(f, "]")
          },
          Value::Matrix(m) => {
              writeln!(f, "[")?;
              for (i, row) in m.iter().enumerate() {
                  if i > 0 { write!(f, " ")? }
                  write!(f, " [")?;
                  for (j, val) in row.iter().enumerate() {
                      if j > 0 { write!(f, ", ")? }
                      write!(f, "{:>8.3}", val)?;
                  }
                  write!(f, "]")?;
                  if i < m.len() - 1 { writeln!(f)? }
              }
              write!(f, "\n]")
          },
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

            // Store the first arg as the return value
            let return_value = args[0].clone();

            // Print the value 
            writeln!(stdout, "{}", return_value).map_err(|e| e.to_string())?;
            stdout.flush().map_err(|e| e.to_string())?;

            // Return the actual value instead of Void
            Ok(return_value)
        }))
      );

      env.declare("input".to_string(), Value::NativeFunction(Arc::new(|args| {
          use std::io::{Write, stdin, stdout};

          if args.is_empty() {
              return Err("Input requires a prompt string".to_string());
          }

          let mut stdout = stdout();

          // Print the prompt
          write!(stdout, "{}", args[0]).map_err(|e| e.to_string())?;
          stdout.flush().map_err(|e| e.to_string())?;

          // Read user input
          let mut input = String::new();
          stdin().read_line(&mut input).map_err(|e| e.to_string())?;

          // Trim whitespace and newline
          let input = input.trim().to_string();

          Ok(Value::String(input))
      })), false);

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
  pub fn as_float(&self) -> Option<f64> {
      match self {
          Value::Int(i) => Some(*i as f64),
          Value::Float(f) => Some(*f),
          Value::Complex { real, imag: _ } => Some(*real),
          _ => None,
      }
  }

  pub fn as_int(&self) -> Option<i64> {
      match self {
          Value::Int(i) => Some(*i),
          Value::Float(f) => Some(*f as i64),
          _ => None,
      }
  }

  pub fn call_method(&self, method: Method, args: Vec<Value>) -> Result<Value, String> {
      match self {
          Value::String(s) => match method {
              Method::Length => Ok(Value::Int(s.len() as i64)),
              Method::ToUpper => Ok(Value::String(s.to_uppercase())),
              Method::ToLower => Ok(Value::String(s.to_lowercase())),
              Method::Trim => Ok(Value::String(s.trim().to_string())),
              Method::Split => {
                  if args.len() != 1 {
                      return Err("split requires one argument".to_string());
                  }
                  let delimiter = match &args[0] {
                      Value::String(s) => s,
                      _ => return Err("split argument must be a string".to_string()),
                  };
                  let parts: Vec<Value> = s.split(delimiter)
                      .map(|s| Value::String(s.to_string()))
                      .collect();
                  Ok(Value::Array(parts))
              },
              _ => Err(format!("Method '{}' not supported for string type", method_name(method))),
          },
          Value::Array(arr) => match method {
              Method::Length => Ok(Value::Int(arr.len() as i64)),
              Method::Push => {
                  if args.len() != 1 {
                      return Err("push requires one argument".to_string());
                  }
                  let mut new_arr = arr.clone();
                  new_arr.push(args[0].clone());
                  Ok(Value::Array(new_arr))
              }
              Method::Pop => {
                  let mut new_arr = arr.clone();
                  new_arr.pop().map_or(
                      Ok(Value::Void),
                      |v| Ok(v)
                  )
              }
              Method::Join => {
                  if args.len() != 1 {
                      return Err("join requires one argument".to_string());
                  }
                  let separator = args[0].to_string();
                  let strings: Result<Vec<String>, String> = arr.iter()
                      .map(|v| Ok(v.to_string()))
                      .collect::<Result<Vec<String>, String>>();
                  strings.map(|strs| Value::String(strs.join(&separator)))
              }
              Method::First => arr.first().map_or(
                  Ok(Value::Void),
                  |v| Ok(v.clone())
              ),
              Method::Last => arr.last().map_or(
                  Ok(Value::Void),
                  |v| Ok(v.clone())
              ),
              _ => Err(format!("Method '{}' not supported for array type", method_name(method))),
          },
          Value::Object(obj) => match method {
              Method::Keys => {
                  let keys: Vec<Value> = obj.keys()
                      .filter(|k| k != &"__keys__")
                      .map(|k| Value::String(k.clone()))
                      .collect();
                  Ok(Value::Array(keys))
              }
              Method::Values => {
                  let values: Vec<Value> = obj.iter()
                      .filter(|(k, _)| k != &"__keys__")
                      .map(|(_, v)| v.clone())
                      .collect();
                  Ok(Value::Array(values))
              }
              Method::Entries => {
                  let entries: Vec<Value> = obj.iter()
                      .filter(|(k, _)| k != &"__keys__")
                      .map(|(k, v)| {
                          Value::Array(vec![
                              Value::String(k.clone()),
                              v.clone()
                          ])
                      })
                      .collect();
                  Ok(Value::Array(entries))
              }
              _ => Err(format!("Method '{}' not supported for object type", method_name(method))),
          },
          Value::Int(n) => match method {
              Method::Round | Method::Floor | Method::Ceil => Ok(Value::Int(*n)),
              Method::ToString => Ok(Value::String(n.to_string())),
              _ => Err(format!("Method '{}' not supported for int type", method_name(method))),
          },
          Value::Float(n) => match method {
              Method::Round => Ok(Value::Int(n.round() as i64)),
              Method::Floor => Ok(Value::Int(n.floor() as i64)),
              Method::Ceil => Ok(Value::Int(n.ceil() as i64)),
              Method::ToString => Ok(Value::String(n.to_string())),
              _ => Err(format!("Method '{}' not supported for float type", method_name(method))),
          },
          _ => Err(format!("Type {} does not support methods", self.type_name())),
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
          Value::Function(_) => "function",
          Value::NativeFunction(_) => "native function",
          Value::Complex { .. } => "complex",
          Value::Vector(_) => "vector",
          Value::Matrix(_) => "matrix",
          Value::Void => "void",
      }
  }
}

fn method_name(method: Method) -> &'static str {
    match method {
        Method::Length => "length",
        Method::ToUpper => "toUpper",
        Method::ToLower => "toLower",
        Method::Trim => "trim",
        Method::Split => "split",
        Method::Push => "push",
        Method::Pop => "pop",
        Method::Join => "join",
        Method::First => "first", 
        Method::Last => "last",
        Method::Keys => "keys",
        Method::Values => "values",
        Method::Entries => "entries",
        Method::Round => "round",
        Method::Floor => "floor",
        Method::Ceil => "ceil",
        Method::ToString => "toString",
    }
}