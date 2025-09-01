use crate::environment::{Environment, Value}; 
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::Arc;

pub fn register(env: &mut Environment) -> Result<(), String> {
    // Create reusable function values
    let read_file_fn = Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(path)] = args.as_slice() {
            match fs::read_to_string(Path::new(path.as_str())) {
                Ok(content) => Ok(Value::String(content)),
                Err(e) => Err(format!("Failed to read file '{}': {}", path, e))
            }
        } else {
            Err("read_file expects a string path argument".to_string())
        }
    }));

    // For object-style access, we maintain an fs object
    let mut fs_obj = HashMap::new();

    // Add functions to the fs object
    fs_obj.insert("read_file".to_string(), read_file_fn.clone());

    fs_obj.insert("write_file".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(path), Value::String(content)] = args.as_slice() {
            match fs::write(Path::new(path.as_str()), content) {
                Ok(_) => Ok(Value::Void),
                Err(e) => Err(format!("Failed to write file '{}': {}", path, e))
            }
        } else {
            Err("write_file expects path and content string arguments".to_string())
        }
    })));

    // Directory Operations
    fs_obj.insert("read_dir".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(path)] = args.as_slice() {
            match fs::read_dir(Path::new(path.as_str())) {
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
    })));

    fs_obj.insert("create_dir".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(path)] = args.as_slice() {
            match fs::create_dir_all(Path::new(path.as_str())) {
                Ok(_) => Ok(Value::Boolean(true)),
                Err(e) => Err(format!("Failed to create directory '{}': {}", path, e))
            }
        } else {
            Err("create_dir expects a string path argument".to_string())
        }
    })));

    fs_obj.insert("remove_dir".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(path)] = args.as_slice() {
            match fs::remove_dir_all(Path::new(path.as_str())) {
                Ok(_) => Ok(Value::Boolean(true)),
                Err(e) => Err(format!("Failed to remove directory '{}': {}", path, e))
            }
        } else {
            Err("remove_dir expects a string path argument".to_string())
        }
    })));

    // Path Operations
    fs_obj.insert("exists".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(path)] = args.as_slice() {
            Ok(Value::Boolean(Path::new(path.as_str()).exists()))
        } else {
            Err("exists expects a string path argument".to_string())
        }
    })));

    fs_obj.insert("is_file".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(path)] = args.as_slice() {
            Ok(Value::Boolean(Path::new(path.as_str()).is_file()))
        } else {
            Err("is_file expects a string path argument".to_string())
        }
    })));

    fs_obj.insert("is_dir".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(path)] = args.as_slice() {
            Ok(Value::Boolean(Path::new(path.as_str()).is_dir()))
        } else {
            Err("is_dir expects a string path argument".to_string())
        }
    })));

    fs_obj.insert("remove_file".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(path)] = args.as_slice() {
            match fs::remove_file(Path::new(path.as_str())) {
                Ok(_) => Ok(Value::Boolean(true)),
                Err(e) => Err(format!("Failed to remove file '{}': {}", path, e))
            }
        } else {
            Err("remove_file expects a string path argument".to_string())
        }
    })));

    // Register our object in the environment
    env.declare("fs".to_string(), Value::Object(fs_obj), true);

    Ok(())
}