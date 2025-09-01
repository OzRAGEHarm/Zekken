use crate::environment::{Environment, Value};
use std::collections::HashMap;
use std::env as std_env;
use std::fs;
use std::sync::Arc;

pub fn register(env: &mut Environment) -> Result<(), String> {
    let mut os_obj = HashMap::new();

    // Get current working directory
    os_obj.insert("cwd".to_string(), Value::NativeFunction(Arc::new(|_args| {
        std_env::current_dir()
            .map(|p| Value::String(p.display().to_string()))
            .map_err(|e| format!("Failed to get current directory: {}", e))
    })));

    // List files in a directory
    os_obj.insert("ls".to_string(), Value::NativeFunction(Arc::new(|args| {
        let path = if let Some(Value::String(s)) = args.get(0) {
            s
        } else {
            "."
        };
        match fs::read_dir(path) {
            Ok(entries) => {
                let files: Vec<Value> = entries
                    .filter_map(|entry| entry.ok())
                    .map(|entry| Value::String(entry.file_name().to_string_lossy().to_string()))
                    .collect();
                Ok(Value::Array(files))
            }
            Err(e) => Err(format!("Failed to list directory '{}': {}", path, e)),
        }
    })));

    // Get environment variable
    os_obj.insert("env".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let Some(Value::String(key)) = args.get(0) {
            match std_env::var(key) {
                Ok(val) => Ok(Value::String(val)),
                Err(_) => Ok(Value::String(String::new())),
            }
        } else {
            Err("env expects a string key".to_string())
        }
    })));

    // Set environment variable
    os_obj.insert("set_env".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let (Some(Value::String(key)), Some(Value::String(val))) = (args.get(0), args.get(1)) {
            std_env::set_var(key, val);
            Ok(Value::Void)
        } else {
            Err("set_env expects two string arguments".to_string())
        }
    })));

    // Remove environment variable
    os_obj.insert("remove_env".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let Some(Value::String(key)) = args.get(0) {
            std_env::remove_var(key);
            Ok(Value::Void)
        } else {
            Err("remove_env expects a string key".to_string())
        }
    })));

    // Get platform/OS
    os_obj.insert("platform".to_string(), Value::NativeFunction(Arc::new(|_args| {
        Ok(Value::String(std_env::consts::OS.to_string()))
    })));

    // Exit process
    os_obj.insert("exit".to_string(), Value::NativeFunction(Arc::new(|args| {
        let code = if let Some(Value::Int(i)) = args.get(0) {
            *i as i32
        } else {
            0
        };
        // Return a special error string to signal exit
        Err(format!("ZK_EXIT_CODE: {}", code))
    })));

    // Get process ID
    os_obj.insert("pid".to_string(), Value::NativeFunction(Arc::new(|_args| {
        let pid = std::process::id();
        Ok(Value::Int(pid as i64))
    })));

    // Sleep for a given number of milliseconds
    os_obj.insert("sleep".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let Some(Value::Int(ms)) = args.get(0) {
            std::thread::sleep(std::time::Duration::from_millis(*ms as u64));
            Ok(Value::Void)
        } else {
            Err("sleep expects an integer (milliseconds)".to_string())
        }
    })));

    env.declare("os".to_string(), Value::Object(os_obj), true);
    Ok(())
}