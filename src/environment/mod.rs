#![allow(dead_code)]

use std::collections::HashMap;
use std::io::Write;
use std::fmt::{self, Display, Formatter};

use crate::ast::*;

#[derive(Debug, Clone)]
pub enum Value {
  Int(i64),
  Float(f64),
  String(String),
  Boolean(bool),
  Array(Vec<Value>),
  Object(HashMap<String, Value>),
  Function(FunctionValue),
  NativeFunction(fn(Vec<Value>) -> Result<Value, String>),
  Complex { real: f64, imag: f64 },
  Vector(Vec<f64>),
  Matrix(Vec<Vec<f64>>),
  Void,
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
                  write!(f, "{}", value)?;
                  first = false;
              }
              write!(f, "]")
          },
          Value::Object(obj) => {
              write!(f, "{{")?;
              let mut first = true;
              for (key, value) in obj {
                  if !first {
                      write!(f, ", ")?;
                  }
                  write!(f, "{}: {}", key, value)?;
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
        Value::NativeFunction(|args: Vec<Value>| -> Result<Value, String> {
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
        })
      );

      env.declare("input".to_string(), Value::NativeFunction(|args| {
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
      }), false);

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
}