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
          let output: Vec<String> = args.iter()
              .map(|v| format!("{}", v))  // This uses the Display implementation
              .collect();
          
          println!("{}", output.join(" "));
          std::io::stdout().flush().unwrap();
          Ok(Value::Void)
      })
    );

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
    if self.constants.contains_key(name) {
      return Err(format!("Cannot reassign constant '{}'", name));
    }

    if self.variables.contains_key(name) {
      self.variables.insert(name.to_string(), value);
      return Ok(());
    }

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
