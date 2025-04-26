use crate::environment::{Environment, Value}; 
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub fn register(env: &mut Environment) -> Result<(), String> {
    let mut fs_obj = HashMap::new();

    // File Operations
    env.declare("read_file".to_string(), Value::NativeFunction(|args| {
        if let [Value::String(path)] = args.as_slice() {
            match fs::read_to_string(Path::new(path)) {
                Ok(content) => Ok(Value::String(content)),
                Err(e) => Err(format!("Failed to read file '{}': {}", path, e))
            }
        } else {
            Err("read_file expects a string path argument".to_string())
        }
    }), true);

    env.declare("write_file".to_string(), Value::NativeFunction(|args| {
        if let [Value::String(path), Value::String(content)] = args.as_slice() {
            match fs::write(Path::new(path), content) {
                Ok(_) => Ok(Value::Void),
                Err(e) => Err(format!("Failed to write file '{}': {}", path, e))
            }
        } else {
            Err("write_file expects path and content string arguments".to_string())
        }
    }), true);

    // Directory Operations
    env.declare("read_dir".to_string(), Value::NativeFunction(|args| {
        if let [Value::String(path)] = args.as_slice() {
            match fs::read_dir(Path::new(path)) {
                Ok(entries) => {
                    let files: Vec<Value> = entries
                        .filter_map(|entry| entry.ok())
                        .map(|entry| Value::String(entry.path().display().to_string()))
                        .collect();
                    Ok(Value::Array(files))
                },
                Err(e) => Err(format!("Failed to read directory '{}': {}", path, e))
            }
        } else {
            Err("read_dir expects a string path argument".to_string())
        }
    }), true);

    env.declare("create_dir".to_string(), Value::NativeFunction(|args| {
        if let [Value::String(path)] = args.as_slice() {
            match fs::create_dir_all(Path::new(path)) {
                Ok(_) => Ok(Value::Boolean(true)),
                Err(e) => Err(format!("Failed to create directory '{}': {}", path, e))
            }
        } else {
            Err("create_dir expects a string path argument".to_string())
        }
    }), true);

    env.declare("remove_dir".to_string(), Value::NativeFunction(|args| {
        if let [Value::String(path)] = args.as_slice() {
            match fs::remove_dir_all(Path::new(path)) {
                Ok(_) => Ok(Value::Boolean(true)),
                Err(e) => Err(format!("Failed to remove directory '{}': {}", path, e))
            }
        } else {
            Err("remove_dir expects a string path argument".to_string())
        }
    }), true);

    // Path Operations
    env.declare("exists".to_string(), Value::NativeFunction(|args| {
        if let [Value::String(path)] = args.as_slice() {
            Ok(Value::Boolean(Path::new(path).exists()))
        } else {
            Err("exists expects a string path argument".to_string())
        }
    }), true);

    env.declare("is_file".to_string(), Value::NativeFunction(|args| {
        if let [Value::String(path)] = args.as_slice() {
            Ok(Value::Boolean(Path::new(path).is_file()))
        } else {
            Err("is_file expects a string path argument".to_string())
        }
    }), true);

    env.declare("is_dir".to_string(), Value::NativeFunction(|args| {
        if let [Value::String(path)] = args.as_slice() {
            Ok(Value::Boolean(Path::new(path).is_dir()))
        } else {
            Err("is_dir expects a string path argument".to_string())
        }
    }), true);

    env.declare("remove_file".to_string(), Value::NativeFunction(|args| {
        if let [Value::String(path)] = args.as_slice() {
            match fs::remove_file(Path::new(path)) {
                Ok(_) => Ok(Value::Boolean(true)),
                Err(e) => Err(format!("Failed to remove file '{}': {}", path, e))
            }
        } else {
            Err("remove_file expects a string path argument".to_string())
        }
    }), true);

    // Always register the full module
    env.declare("fs".to_string(), Value::Object(fs_obj), true);

    Ok(())
}