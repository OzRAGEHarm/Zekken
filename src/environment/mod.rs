use std::collections::HashMap;
use std::io::Write;

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
  fn format_value(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Int(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Array(arr) => {
            let elements: Vec<String> = arr.iter()
                .map(|v| Environment::format_value(v))
                .collect();
            format!("[{}]", elements.join(", "))
        },
        Value::Object(obj) => {
            let properties: Vec<String> = obj.iter()
                .map(|(k, v)| format!("{}: {}", k, Environment::format_value(v)))
                .collect();
            format!("{{{}}}", properties.join(", "))
        },
        Value::Function(_) => "<function>".to_string(),
        Value::NativeFunction(_) => "<native function>".to_string(),
        Value::Void => "void".to_string(),
    }
  }
  
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
              .map(|arg| Environment::format_value(arg))
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
