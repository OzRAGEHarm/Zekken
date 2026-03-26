use crate::environment::{Environment, Value}; 
use hashbrown::HashMap;
use std::fs;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Write};
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

    fs_obj.insert("append_file".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(path), Value::String(content)] = args.as_slice() {
            match OpenOptions::new().create(true).append(true).open(Path::new(path.as_str())) {
                Ok(mut file) => match file.write_all(content.as_bytes()) {
                    Ok(_) => Ok(Value::Void),
                    Err(e) => Err(format!("Failed to append to file '{}': {}", path, e)),
                },
                Err(e) => Err(format!("Failed to open file '{}' for append: {}", path, e)),
            }
        } else {
            Err("append_file expects path and content string arguments".to_string())
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

    fs_obj.insert("read_lines".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(path)] = args.as_slice() {
            match fs::File::open(Path::new(path.as_str())) {
                Ok(file) => {
                    let reader = BufReader::new(file);
                    let mut lines: Vec<Value> = Vec::new();
                    for line in reader.lines() {
                        match line {
                            Ok(text) => lines.push(Value::String(text)),
                            Err(e) => return Err(format!("Failed reading line from '{}': {}", path, e)),
                        }
                    }
                    Ok(Value::Array(lines))
                }
                Err(e) => Err(format!("Failed to open file '{}': {}", path, e)),
            }
        } else {
            Err("read_lines expects a string path argument".to_string())
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

    fs_obj.insert("copy_file".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(from), Value::String(to)] = args.as_slice() {
            match fs::copy(Path::new(from.as_str()), Path::new(to.as_str())) {
                Ok(bytes) => Ok(Value::Int(bytes as i64)),
                Err(e) => Err(format!("Failed to copy file '{}' -> '{}': {}", from, to, e)),
            }
        } else {
            Err("copy_file expects source and destination string paths".to_string())
        }
    })));

    fs_obj.insert("rename".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(from), Value::String(to)] = args.as_slice() {
            match fs::rename(Path::new(from.as_str()), Path::new(to.as_str())) {
                Ok(_) => Ok(Value::Void),
                Err(e) => Err(format!("Failed to rename '{}' -> '{}': {}", from, to, e)),
            }
        } else {
            Err("rename expects source and destination string paths".to_string())
        }
    })));

    fs_obj.insert("stat".to_string(), Value::NativeFunction(Arc::new(|args| {
        if let [Value::String(path)] = args.as_slice() {
            match fs::metadata(Path::new(path.as_str())) {
                Ok(meta) => {
                    let mut out = HashMap::new();
                    out.insert("path".to_string(), Value::String(path.clone()));
                    out.insert("size".to_string(), Value::Int(meta.len() as i64));
                    out.insert("is_file".to_string(), Value::Boolean(meta.is_file()));
                    out.insert("is_dir".to_string(), Value::Boolean(meta.is_dir()));
                    out.insert("readonly".to_string(), Value::Boolean(meta.permissions().readonly()));
                    let modified_epoch = meta
                        .modified()
                        .ok()
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_secs() as i64)
                        .unwrap_or(0);
                    out.insert("modified_unix".to_string(), Value::Int(modified_epoch));
                    Ok(Value::Object(out))
                }
                Err(e) => Err(format!("Failed to stat '{}': {}", path, e)),
            }
        } else {
            Err("stat expects a string path argument".to_string())
        }
    })));

    env.declare("fs".to_string(), Value::Object(fs_obj), true);

    Ok(())
}
