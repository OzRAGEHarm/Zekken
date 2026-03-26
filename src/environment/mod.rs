#![allow(dead_code)]

use hashbrown::HashMap;
use std::cell::RefCell;
use std::collections::VecDeque;
use std::io::Write;
use std::fmt::{self, Display, Formatter};
use std::sync::{Arc, Mutex};
use crate::ast::*;
use crate::lexer::DataType;
use serde_json::Value as JsonValue;

thread_local! {
    static SCOPE_POOL: RefCell<Vec<Environment>> = const { RefCell::new(Vec::new()) };
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
        self.fmt_compact(f, false)
    }
}

impl Value {
    fn write_escaped_string(f: &mut Formatter, s: &str) -> fmt::Result {
        for ch in s.chars() {
            match ch {
                '\\' => write!(f, "\\\\")?,
                '"' => write!(f, "\\\"")?,
                '\n' => write!(f, "\\n")?,
                '\r' => write!(f, "\\r")?,
                '\t' => write!(f, "\\t")?,
                _ => write!(f, "{}", ch)?,
            }
        }
        Ok(())
    }

    fn fmt_compact(&self, f: &mut Formatter, in_container: bool) -> fmt::Result {
        match self {
            Value::Array(arr) => {
                if arr.is_empty() {
                    write!(f, "[]")
                } else {
                    write!(f, "[")?;
                    for (i, value) in arr.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        value.fmt_compact(f, true)?;
                    }
                    write!(f, "]")
                }
            }
            Value::Object(obj) => {
                // If this object is an error (has __zekken_error__), print the pretty error string
                if let Some(Value::String(pretty)) = obj.get("__zekken_error__") {
                    write!(f, "{}", pretty)
                } else {
                    write!(f, "{{")?;
                    let mut first = true;

                    // Prefer insertion-order key list when available.
                    if let Some(Value::Array(keys)) = obj.get("__keys__") {
                        for key_val in keys {
                            if let Value::String(k) = key_val {
                                if k == "__keys__" || k == "__zekken_error__" {
                                    continue;
                                }
                                if let Some(v) = obj.get(k) {
                                    if !first { write!(f, ", ")?; }
                                    write!(f, "{}: ", k)?;
                                    v.fmt_compact(f, true)?;
                                    first = false;
                                }
                            }
                        }
                    } else {
                        // Deterministic fallback order for objects without __keys__.
                        let mut keys: Vec<&String> = obj
                            .keys()
                            .filter(|k| k.as_str() != "__keys__" && k.as_str() != "__zekken_error__")
                            .collect();
                        keys.sort_unstable();
                        for k in keys {
                            if let Some(v) = obj.get(k) {
                                if !first { write!(f, ", ")?; }
                                write!(f, "{}: ", k)?;
                                v.fmt_compact(f, true)?;
                                first = false;
                            }
                        }
                    }
                    write!(f, "}}")
                }
            }
            Value::String(s) => {
                if in_container {
                    write!(f, "\"")?;
                    Self::write_escaped_string(f, s)?;
                    write!(f, "\"")
                } else {
                    write!(f, "{}", s)
                }
            }
            Value::Int(i) => write!(f, "{}", i),
            Value::Float(fl) => {
                if fl.fract() == 0.0 {
                    write!(f, "{:.1}", fl)
                } else {
                    write!(f, "{}", fl)
                }
            },
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
                // Keep matrix compact(ish) in default printing.
                write!(f, "[")?;
                for (i, row) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "[")?;
                    for (j, val) in row.iter().enumerate() {
                        if j > 0 { write!(f, ", ")? }
                        write!(f, "{:>8.3}", val)?;
                    }
                    write!(f, "]")?;
                }
                write!(f, "]")
            }
            Value::Void => write!(f, "void"),
        }
    }

    fn fmt_pretty(&self, f: &mut Formatter, indent: usize, in_container: bool) -> fmt::Result {
        let indent_str = |n| "  ".repeat(n);
        match self {
            Value::Array(arr) => {
                if arr.is_empty() {
                    write!(f, "[]")
                } else {
                    writeln!(f, "[")?;
                    for (i, value) in arr.iter().enumerate() {
                        write!(f, "{}", indent_str(indent + 1))?;
                        value.fmt_pretty(f, indent + 1, true)?;
                        if i < arr.len() - 1 {
                            writeln!(f, ",")?;
                        } else {
                            writeln!(f)?;
                        }
                    }
                    write!(f, "{}]", indent_str(indent))
                }
            }
            Value::Object(obj) => {
                if let Some(Value::String(pretty)) = obj.get("__zekken_error__") {
                    write!(f, "{}", pretty)
                } else {
                    // Collect keys in deterministic order (prefer __keys__ insertion order).
                    let mut ordered: Vec<&String> = Vec::new();
                    if let Some(Value::Array(keys)) = obj.get("__keys__") {
                        for key_val in keys {
                            if let Value::String(k) = key_val {
                                if k == "__keys__" || k == "__zekken_error__" {
                                    continue;
                                }
                                if obj.contains_key(k) {
                                    ordered.push(k);
                                }
                            }
                        }
                    } else {
                        ordered = obj
                            .keys()
                            .filter(|k| k.as_str() != "__keys__" && k.as_str() != "__zekken_error__")
                            .collect();
                        ordered.sort_unstable();
                    }

                    if ordered.is_empty() {
                        return write!(f, "{{}}");
                    }

                    writeln!(f, "{{")?;
                    for (i, k) in ordered.iter().enumerate() {
                        if let Some(v) = obj.get(*k) {
                            write!(f, "{}{}: ", indent_str(indent + 1), k)?;
                            v.fmt_pretty(f, indent + 1, true)?;
                            if i < ordered.len() - 1 {
                                writeln!(f, ",")?;
                            } else {
                                writeln!(f)?;
                            }
                        }
                    }
                    write!(f, "{}}}", indent_str(indent))
                }
            }
            Value::String(s) => {
                if in_container {
                    write!(f, "\"")?;
                    Self::write_escaped_string(f, s)?;
                    write!(f, "\"")
                } else {
                    write!(f, "{}", s)
                }
            }
            _ => self.fmt_compact(f, in_container),
        }
    }

    pub fn to_pretty_string(&self) -> String {
        struct PrettyValue<'a>(&'a Value);
        impl<'a> Display for PrettyValue<'a> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                self.0.fmt_pretty(f, 0, false)
            }
        }
        PrettyValue(self).to_string()
    }
}

pub fn interpolate_named_placeholders<'a, F>(template: &str, mut lookup: F) -> String
where
    F: FnMut(&str) -> Option<&'a Value>,
{
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

        let key = &template[i + 1..j];
        let key_trimmed = key.trim();
        let is_ident = !key_trimmed.is_empty()
            && key_trimmed
                .chars()
                .enumerate()
                .all(|(idx, c)| if idx == 0 { c.is_ascii_alphabetic() || c == '_' } else { c.is_ascii_alphanumeric() || c == '_' });

        if is_ident {
            out.push_str(&template[segment_start..i]);
            if let Some(v) = lookup(key_trimmed) {
                out.push_str(&v.to_string());
            } else {
                out.push('{');
                out.push_str(key);
                out.push('}');
            }
            i = j + 1;
            segment_start = i;
        } else {
            i += 1;
        }
    }

    out.push_str(&template[segment_start..]);
    out
}

fn interpolate_positional_placeholders(template: &str, args: &[Value]) -> (String, usize) {
    let mut out = String::with_capacity(template.len());
    let bytes = template.as_bytes();
    let mut i = 0usize;
    let mut segment_start = 0usize;
    let mut used = 0usize;

    while i + 1 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'}' {
            out.push_str(&template[segment_start..i]);
            if let Some(val) = args.get(used) {
                out.push_str(&val.to_string());
                used += 1;
            } else {
                out.push_str("{}");
            }
            i += 2;
            segment_start = i;
            continue;
        }
        i += 1;
    }

    out.push_str(&template[segment_start..]);
    (out, used)
}

pub fn format_print_values(args: &[Value]) -> String {
    if args.is_empty() {
        return String::new();
    }

    if let Value::String(template) = &args[0] {
        let (mut out, used) = interpolate_positional_placeholders(template, &args[1..]);
        if args.len() > used + 1 {
            for val in &args[(used + 1)..] {
                out.push(' ');
                out.push_str(&val.to_string());
            }
        }
        out
    } else {
        let mut out = String::new();
        for (idx, v) in args.iter().enumerate() {
            if idx > 0 {
                out.push(' ');
            }
            out.push_str(&v.to_string());
        }
        out
    }
}

#[derive(Debug, Clone)]
pub struct FunctionValue {
  pub params: Arc<Vec<Param>>,
  pub body: Arc<Vec<Box<Content>>>,
  pub return_type: Option<DataType>,
  pub needs_parent: bool,
  pub captures: Arc<Vec<String>>,
  //pub closure: Environment,
}

#[derive(Debug, Clone)]
pub struct Environment {
  pub parent: Option<Box<Environment>>,
  pub variables: HashMap<String, Value>,
  pub constants: HashMap<String, Value>,
  pub types: HashMap<String, DataType>,
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
            let mut map = HashMap::new();
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
  pub fn new_scope_with_capacity(var_capacity: usize) -> Self {
      Environment {
          parent: None,
          variables: HashMap::with_capacity(var_capacity.max(4)),
          constants: HashMap::with_capacity(0),
          types: HashMap::with_capacity(var_capacity.max(4)),
      }
  }

  pub fn new_scope() -> Self {
      Self::new_scope_with_capacity(16)
  }

  pub fn take_pooled_scope(var_capacity: usize) -> Self {
      let mut reused = None;
      SCOPE_POOL.with(|pool| {
          reused = pool.borrow_mut().pop();
      });

      if let Some(mut env) = reused {
          env.parent = None;
          env.variables.clear();
          env.constants.clear();
          env.types.clear();
          env.variables.reserve(var_capacity.max(4));
          env.types.reserve(var_capacity.max(4));
          return env;
      }

      Self::new_scope_with_capacity(var_capacity)
  }

  pub fn return_pooled_scope(mut env: Environment) {
      if env.parent.is_some() {
          return;
      }
      env.variables.clear();
      env.constants.clear();
      env.types.clear();
      SCOPE_POOL.with(|pool| {
          pool.borrow_mut().push(env);
      });
  }

  pub fn new() -> Self {
      let mut env = Environment {
          parent: None,
          variables: HashMap::with_capacity(64),
          constants: HashMap::with_capacity(16),
          types: HashMap::with_capacity(64),
      };

      let disable_print = match std::env::var("ZEKKEN_DISABLE_PRINT") {
          Ok(val) => val == "1" || val.eq_ignore_ascii_case("true"),
          Err(_) => false,
      };

      env.constants.insert(
        "println".to_string(),
        Value::NativeFunction(Arc::new(move |args: Vec<Value>| -> Result<Value, String> {
            if disable_print {
                return Ok(Value::Void);
            }

            let mut stdout = std::io::stdout();

            if args.is_empty() {
                writeln!(stdout).map_err(|e| e.to_string())?;
                return Ok(Value::Void);
            }

            let line = format_print_values(&args);
            writeln!(stdout, "{}", line).map_err(|e| e.to_string())?;

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

      env.declare(
        "queue".to_string(),
        Value::NativeFunction(Arc::new(|args: Vec<Value>| -> Result<Value, String> {
            if !args.is_empty() {
                return Err("queue expects no arguments".to_string());
            }

            let queue = Arc::new(Mutex::new(VecDeque::<Value>::new()));
            let mut obj = HashMap::with_capacity(6);

            {
                let q = queue.clone();
                obj.insert(
                    "enqueue".to_string(),
                    Value::NativeFunction(Arc::new(move |mut a: Vec<Value>| -> Result<Value, String> {
                        if a.len() != 1 {
                            return Err("enqueue requires exactly one argument".to_string());
                        }
                        let mut guard = q.lock().map_err(|_| "queue lock poisoned".to_string())?;
                        guard.push_back(a.remove(0));
                        Ok(Value::Void)
                    })),
                );
            }

            {
                let q = queue.clone();
                obj.insert(
                    "dequeue".to_string(),
                    Value::NativeFunction(Arc::new(move |a: Vec<Value>| -> Result<Value, String> {
                        if !a.is_empty() {
                            return Err("dequeue requires no arguments".to_string());
                        }
                        let mut guard = q.lock().map_err(|_| "queue lock poisoned".to_string())?;
                        Ok(guard.pop_front().unwrap_or(Value::Void))
                    })),
                );
            }

            {
                let q = queue.clone();
                obj.insert(
                    "peek".to_string(),
                    Value::NativeFunction(Arc::new(move |a: Vec<Value>| -> Result<Value, String> {
                        if !a.is_empty() {
                            return Err("peek requires no arguments".to_string());
                        }
                        let guard = q.lock().map_err(|_| "queue lock poisoned".to_string())?;
                        Ok(guard.front().cloned().unwrap_or(Value::Void))
                    })),
                );
            }

            {
                let q = queue.clone();
                obj.insert(
                    "length".to_string(),
                    Value::NativeFunction(Arc::new(move |a: Vec<Value>| -> Result<Value, String> {
                        if !a.is_empty() {
                            return Err("length requires no arguments".to_string());
                        }
                        let guard = q.lock().map_err(|_| "queue lock poisoned".to_string())?;
                        Ok(Value::Int(guard.len() as i64))
                    })),
                );
            }

            {
                let q = queue.clone();
                obj.insert(
                    "is_empty".to_string(),
                    Value::NativeFunction(Arc::new(move |a: Vec<Value>| -> Result<Value, String> {
                        if !a.is_empty() {
                            return Err("is_empty requires no arguments".to_string());
                        }
                        let guard = q.lock().map_err(|_| "queue lock poisoned".to_string())?;
                        Ok(Value::Boolean(guard.is_empty()))
                    })),
                );
            }

            {
                let q = queue.clone();
                obj.insert(
                    "clear".to_string(),
                    Value::NativeFunction(Arc::new(move |a: Vec<Value>| -> Result<Value, String> {
                        if !a.is_empty() {
                            return Err("clear requires no arguments".to_string());
                        }
                        let mut guard = q.lock().map_err(|_| "queue lock poisoned".to_string())?;
                        guard.clear();
                        Ok(Value::Void)
                    })),
                );
            }

            Ok(Value::Object(obj))
        })),
        true,
      );

      env
  }

  pub fn new_with_parent(parent: Environment) -> Self {
      Environment {
          parent: Some(Box::new(parent)),
          variables: HashMap::with_capacity(16),
          constants: HashMap::with_capacity(8),
          types: HashMap::with_capacity(16),
      }
  }

  pub fn new_with_parent_capacity(parent: Environment, var_capacity: usize) -> Self {
      Environment {
          parent: Some(Box::new(parent)),
          variables: HashMap::with_capacity(var_capacity.max(4)),
          constants: HashMap::with_capacity(0),
          types: HashMap::with_capacity(var_capacity.max(4)),
      }
  }

  pub fn declare(&mut self, name: String, value: Value, constant: bool) {
      let type_key = name.clone();
      if constant {
          self.constants.insert(name, value);
      } else {
          self.variables.insert(name, value);
      }
      self.types.entry(type_key).or_insert(DataType::Any);
  }

  #[inline]
  pub fn declare_ref(&mut self, name: &str, value: Value, constant: bool) {
      if constant {
          if let Some(slot) = self.constants.get_mut(name) {
              *slot = value;
          } else {
              self.constants.insert(name.to_string(), value);
          }
      } else if let Some(slot) = self.variables.get_mut(name) {
          *slot = value;
      } else {
          self.variables.insert(name.to_string(), value);
      }
      self.types.entry(name.to_string()).or_insert(DataType::Any);
  }

  #[inline]
  pub fn declare_ref_typed(&mut self, name: &str, value: Value, ty: DataType, constant: bool) {
      self.declare_ref(name, value, constant);
      self.types.insert(name.to_string(), ty);
  }

  #[inline]
  pub fn lookup_type(&self, name: &str) -> Option<DataType> {
      let mut env = self;
      loop {
          if let Some(t) = env.types.get(name) {
              return Some(*t);
          }
          if let Some(parent) = env.parent.as_ref() {
              env = parent;
          } else {
              return None;
          }
      }
  }

  #[inline]
  fn value_label(value: &Value) -> &'static str {
      match value {
          Value::Int(_) => "int",
          Value::Float(_) => "float",
          Value::String(_) => "string",
          Value::Boolean(_) => "bool",
          Value::Array(_) => "arr",
          Value::Object(_) => "obj",
          Value::Function(_) | Value::NativeFunction(_) => "fn",
          _ => "other",
      }
  }

  #[inline]
  fn datatype_label(ty: &DataType) -> &'static str {
      match ty {
          DataType::Int => "int",
          DataType::Float => "float",
          DataType::String => "string",
          DataType::Bool => "bool",
          DataType::Object => "obj",
          DataType::Array => "arr",
          DataType::Fn => "fn",
          DataType::Any => "any",
      }
  }

  #[inline]
  fn value_matches_datatype(value: &Value, expected: &DataType) -> bool {
      match expected {
          DataType::Any => true,
          DataType::Int => matches!(value, Value::Int(_)),
          DataType::Float => matches!(value, Value::Float(_)),
          DataType::String => matches!(value, Value::String(_)),
          DataType::Bool => matches!(value, Value::Boolean(_)),
          DataType::Object => matches!(value, Value::Object(_)),
          DataType::Array => matches!(value, Value::Array(_)),
          DataType::Fn => matches!(value, Value::Function(_) | Value::NativeFunction(_)),
      }
  }


  #[inline]
  pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
      // First check if variable exists in current scope.
      if let Some(slot) = self.variables.get_mut(name) {
          if let Some(expected) = self.types.get(name) {
              if !Self::value_matches_datatype(&value, expected) {
                  return Err(format!(
                      "Type mismatch in assignment to '{}': expected '{}', got '{}'",
                      name,
                      Self::datatype_label(expected),
                      Self::value_label(&value),
                  ));
              }
          }
          *slot = value;
          return Ok(());
      }

      // Constants cannot be reassigned in their declaring scope.
      if self.constants.contains_key(name) {
          return Err(format!("Cannot reassign constant '{}'", name));
      }

      // If not in current scope, try parent scope
      if let Some(ref mut parent) = self.parent {
          return parent.assign(name, value);
      }

      Err(format!("Variable '{}' not found", name))
  }

  #[inline]
  pub fn lookup(&self, name: &str) -> Option<Value> {
      let mut env = self;
      loop {
          if let Some(v) = env.variables.get(name) {
              return Some(match v {
                  Value::Int(i) => Value::Int(*i),
                  Value::Float(f) => Value::Float(*f),
                  Value::Boolean(b) => Value::Boolean(*b),
                  _ => v.clone(),
              });
          }
          if let Some(v) = env.constants.get(name) {
              return Some(match v {
                  Value::Int(i) => Value::Int(*i),
                  Value::Float(f) => Value::Float(*f),
                  Value::Boolean(b) => Value::Boolean(*b),
                  _ => v.clone(),
              });
          }
          if let Some(parent) = env.parent.as_ref() {
              env = parent;
          } else {
              return None;
          }
      }
  }

  #[inline]
  pub fn lookup_ref(&self, name: &str) -> Option<&Value> {
      let mut env = self;
      loop {
          if let Some(v) = env.variables.get(name) {
              return Some(v);
          }
          if let Some(v) = env.constants.get(name) {
              return Some(v);
          }
          if let Some(parent) = env.parent.as_ref() {
              env = parent;
          } else {
              return None;
          }
      }
  }

  #[inline]
  pub fn lookup_mut_assignable(&mut self, name: &str) -> Result<&mut Value, String> {
      if let Some(v) = self.variables.get_mut(name) {
          return Ok(v);
      }

      if self.constants.contains_key(name) {
          return Err(format!("Cannot reassign constant '{}'", name));
      }

      if let Some(parent) = self.parent.as_mut() {
          return parent.lookup_mut_assignable(name);
      }

      Err(format!("Variable '{}' not found", name))
  }

  /// Lookup a name and return (Option<Value>, Option<&'static str> kind)
  pub fn lookup_with_kind(&self, name: &str) -> (Option<Value>, Option<&'static str>) {
      let mut env = self;
      loop {
          if let Some(val) = env.variables.get(name) {
              let kind = match val {
                  Value::Function(_) => "function",
                  Value::NativeFunction(_) => "native function",
                  _ => "variable",
              };
              return (Some(val.clone()), Some(kind));
          }
          if let Some(val) = env.constants.get(name) {
              let kind = match val {
                  Value::Function(_) => "function",
                  Value::NativeFunction(_) => "native function",
                  _ => "constant",
              };
              return (Some(val.clone()), Some(kind));
          }
          if let Some(parent) = env.parent.as_ref() {
              env = parent;
          } else {
              return (None, None);
          }
      }
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
        if method_name == "format" {
            if !args.is_empty() {
                return Err("format takes no arguments".to_string());
            }
            return Ok(Value::String(self.to_pretty_string()));
        }
        if method_name == "cast" {
            return self.handle_cast(args);
        }

        match self {
            Value::String(s) => Self::handle_string_method(s, method_name, args),
            Value::Array(arr) => Self::handle_array_method(arr, method_name, args, env, variable_name),
            Value::Object(obj) => {
                // First check if the object has the method as a native function
                if let Some(Value::NativeFunction(func)) = obj.get(method_name) {
                    // Execute the native function directly
                    return (func)(args);
                }
                
                // If we didn't find a native function but found something, treat it as a regular value
                if let Some(value) = obj.get(method_name) {
                    // If it's a function, call it with the arguments
                    if let Value::Function(_) = value {
                        return value.call_method("__call__", args, env, None);
                    }
                    // If it's a native function, call it with the arguments
                    if let Value::NativeFunction(func) = value {
                        return (func)(args);
                    }
                    // For other values, just return them if no args were provided
                    if args.is_empty() {
                        return Ok(value.clone());
                    }
                }
                
                // If nothing else matched, try standard object methods
                Self::handle_object_method(obj, method_name, args)
            }
            Value::Int(n) => Self::handle_int_method(*n, method_name, args),
            Value::Float(n) => Self::handle_float_method(*n, method_name, args),
            _ => Err(format!("Type '{}' does not support methods", self.type_name())),
        }
    }

    fn handle_cast(&self, args: Vec<Value>) -> Result<Value, String> {
        if args.len() != 1 {
            return Err("cast requires one string argument (target type)".to_string());
        }

        let target = match &args[0] {
            Value::String(s) => s.trim().to_ascii_lowercase(),
            _ => return Err("cast target type must be a string".to_string()),
        };

        match target.as_str() {
            "string" => Ok(Value::String(self.to_string())),
            "str" => Err("Unsupported cast target 'str'. Use 'string'.".to_string()),
            "int" => match self {
                Value::Int(i) => Ok(Value::Int(*i)),
                Value::Float(f) => Ok(Value::Int(*f as i64)),
                Value::Boolean(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                Value::String(s) => s
                    .trim()
                    .parse::<i64>()
                    .map(Value::Int)
                    .map_err(|_| format!("Cannot cast string '{}' to int", s)),
                _ => Err(format!("Cannot cast type '{}' to int", self.type_name())),
            },
            "float" => match self {
                Value::Float(f) => Ok(Value::Float(*f)),
                Value::Int(i) => Ok(Value::Float(*i as f64)),
                Value::Boolean(b) => Ok(Value::Float(if *b { 1.0 } else { 0.0 })),
                Value::String(s) => s
                    .trim()
                    .parse::<f64>()
                    .map(Value::Float)
                    .map_err(|_| format!("Cannot cast string '{}' to float", s)),
                _ => Err(format!("Cannot cast type '{}' to float", self.type_name())),
            },
            "bool" => match self {
                Value::Boolean(b) => Ok(Value::Boolean(*b)),
                Value::Int(i) => Ok(Value::Boolean(*i != 0)),
                Value::Float(f) => Ok(Value::Boolean(*f != 0.0)),
                Value::String(s) => {
                    let lower = s.trim().to_ascii_lowercase();
                    match lower.as_str() {
                        "true" | "1" => Ok(Value::Boolean(true)),
                        "false" | "0" => Ok(Value::Boolean(false)),
                        _ => Err(format!("Cannot cast string '{}' to bool", s)),
                    }
                }
                _ => Err(format!("Cannot cast type '{}' to bool", self.type_name())),
            },
            _ => Err(format!("Unsupported cast target '{}'", target)),
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
            "shift" => {
                let mut new_arr = arr.clone();
                if new_arr.is_empty() {
                    return Err("Array is empty".to_string());
                }
                let shifted = new_arr.remove(0);
                if let Some(env) = env {
                    if let Some(var_name) = variable_name {
                        env.assign(var_name, Value::Array(new_arr))
                            .map_err(|e| format!("Failed to update array: {}", e))?;
                    }
                }
                Ok(shifted)
            }
            "unshift" => {
                if args.len() != 1 {
                    return Err("unshift requires exactly one argument".to_string());
                }
                if let Some(env) = env {
                    if let Some(var_name) = variable_name {
                        let mut new_arr = arr.clone();
                        new_arr.insert(0, args.remove(0));
                        env.assign(var_name, Value::Array(new_arr.clone()))
                            .map_err(|e| format!("Failed to update array: {}", e))?;
                        Ok(Value::Array(new_arr))
                    } else {
                        Err("unshift requires a variable name to update the original array".to_string())
                    }
                } else {
                    Err("unshift requires an environment to update the original array".to_string())
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
        // First check if it's a NativeFunction
        if let Some(Value::NativeFunction(func)) = obj.get(method_name) {
            return (func)(args);
        }

        // If not a native function, try standard object methods
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
