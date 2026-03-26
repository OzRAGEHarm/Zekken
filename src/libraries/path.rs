use crate::environment::{Environment, Value};
use hashbrown::HashMap;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;

fn value_to_string_arg(value: &Value, fn_name: &str) -> Result<String, String> {
    match value {
        Value::String(s) => Ok(s.clone()),
        _ => Err(format!("{} expects string path arguments", fn_name)),
    }
}

fn collect_path_parts(args: &[Value], fn_name: &str) -> Result<Vec<String>, String> {
    if args.is_empty() {
        return Err(format!("{} expects at least one path argument", fn_name));
    }

    if let [Value::Array(items)] = args {
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            out.push(value_to_string_arg(item, fn_name)?);
        }
        if out.is_empty() {
            return Err(format!("{} expects at least one path argument", fn_name));
        }
        return Ok(out);
    }

    let mut out = Vec::with_capacity(args.len());
    for arg in args {
        out.push(value_to_string_arg(arg, fn_name)?);
    }
    Ok(out)
}

fn normalize_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    normalized
}

fn make_absolute(path: &Path) -> Result<PathBuf, String> {
    if path.is_absolute() {
        Ok(normalize_path(path))
    } else {
        let cwd = std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
        Ok(normalize_path(&cwd.join(path)))
    }
}

fn relative_between(from: &Path, to: &Path) -> Result<PathBuf, String> {
    let from_abs = make_absolute(from)?;
    let to_abs = make_absolute(to)?;

    let from_parts: Vec<String> = from_abs
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().to_string()),
            _ => None,
        })
        .collect();
    let to_parts: Vec<String> = to_abs
        .components()
        .filter_map(|c| match c {
            Component::Normal(s) => Some(s.to_string_lossy().to_string()),
            _ => None,
        })
        .collect();

    let mut common = 0usize;
    while common < from_parts.len() && common < to_parts.len() && from_parts[common] == to_parts[common] {
        common += 1;
    }

    let mut rel = PathBuf::new();
    for _ in common..from_parts.len() {
        rel.push("..");
    }
    for part in &to_parts[common..] {
        rel.push(part);
    }

    if rel.as_os_str().is_empty() {
        Ok(PathBuf::from("."))
    } else {
        Ok(rel)
    }
}

pub fn register(env: &mut Environment) -> Result<(), String> {
    let mut path_obj = HashMap::new();

    path_obj.insert(
        "join".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            let parts = collect_path_parts(&args, "path.join")?;
            let mut out = PathBuf::new();
            for part in parts {
                out.push(part);
            }
            Ok(Value::String(out.to_string_lossy().to_string()))
        })),
    );

    path_obj.insert(
        "normalize".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            if args.len() != 1 {
                return Err("path.normalize expects exactly one path string".to_string());
            }
            let raw = value_to_string_arg(&args[0], "path.normalize")?;
            Ok(Value::String(normalize_path(Path::new(raw.as_str())).to_string_lossy().to_string()))
        })),
    );

    path_obj.insert(
        "resolve".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            if args.is_empty() {
                let cwd = std::env::current_dir().map_err(|e| format!("Failed to get current directory: {}", e))?;
                return Ok(Value::String(cwd.to_string_lossy().to_string()));
            }

            let parts = collect_path_parts(&args, "path.resolve")?;
            let mut current = PathBuf::new();
            for part in parts {
                let p = Path::new(part.as_str());
                if p.is_absolute() {
                    current = PathBuf::from(p);
                } else {
                    current.push(p);
                }
            }

            let abs = make_absolute(&current)?;
            Ok(Value::String(abs.to_string_lossy().to_string()))
        })),
    );

    path_obj.insert(
        "basename".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            if args.len() != 1 {
                return Err("path.basename expects exactly one path string".to_string());
            }
            let raw = value_to_string_arg(&args[0], "path.basename")?;
            let name = Path::new(raw.as_str())
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(name))
        })),
    );

    path_obj.insert(
        "dirname".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            if args.len() != 1 {
                return Err("path.dirname expects exactly one path string".to_string());
            }
            let raw = value_to_string_arg(&args[0], "path.dirname")?;
            let p = Path::new(raw.as_str());
            let dir = match p.parent() {
                Some(parent) if !parent.as_os_str().is_empty() => parent.to_string_lossy().to_string(),
                Some(_) => ".".to_string(),
                None => ".".to_string(),
            };
            Ok(Value::String(dir))
        })),
    );

    path_obj.insert(
        "extname".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            if args.len() != 1 {
                return Err("path.extname expects exactly one path string".to_string());
            }
            let raw = value_to_string_arg(&args[0], "path.extname")?;
            let ext = Path::new(raw.as_str())
                .extension()
                .map(|s| format!(".{}", s.to_string_lossy()))
                .unwrap_or_default();
            Ok(Value::String(ext))
        })),
    );

    path_obj.insert(
        "stem".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            if args.len() != 1 {
                return Err("path.stem expects exactly one path string".to_string());
            }
            let raw = value_to_string_arg(&args[0], "path.stem")?;
            let stem = Path::new(raw.as_str())
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_default();
            Ok(Value::String(stem))
        })),
    );

    path_obj.insert(
        "is_abs".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            if args.len() != 1 {
                return Err("path.is_abs expects exactly one path string".to_string());
            }
            let raw = value_to_string_arg(&args[0], "path.is_abs")?;
            Ok(Value::Boolean(Path::new(raw.as_str()).is_absolute()))
        })),
    );

    path_obj.insert(
        "relative".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            if args.len() != 2 {
                return Err("path.relative expects exactly two path strings (from, to)".to_string());
            }
            let from = value_to_string_arg(&args[0], "path.relative")?;
            let to = value_to_string_arg(&args[1], "path.relative")?;
            let rel = relative_between(Path::new(from.as_str()), Path::new(to.as_str()))?;
            Ok(Value::String(rel.to_string_lossy().to_string()))
        })),
    );

    env.declare("path".to_string(), Value::Object(path_obj), true);
    Ok(())
}
