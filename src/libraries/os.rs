use crate::environment::{Environment, Value};
use hashbrown::HashMap;
use std::env as std_env;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
#[cfg(not(target_arch = "wasm32"))]
use std::process::Command;

fn shell_disabled_message() -> String {
    "Shell execution is disabled in this runtime.".to_string()
}

fn shell_execution_allowed() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        false
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        !matches!(std_env::var("ZEKKEN_DISABLE_SHELL"), Ok(v) if v == "1" || v.eq_ignore_ascii_case("true"))
    }
}

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

    // Command line args
    os_obj.insert("args".to_string(), Value::NativeFunction(Arc::new(|_args| {
        let args: Vec<Value> = std_env::args().map(Value::String).collect();
        Ok(Value::Array(args))
    })));

    // Home directory
    os_obj.insert("home_dir".to_string(), Value::NativeFunction(Arc::new(|_args| {
        let home = std_env::var("HOME")
            .or_else(|_| std_env::var("USERPROFILE"))
            .unwrap_or_default();
        Ok(Value::String(home))
    })));

    // Temp directory
    os_obj.insert("temp_dir".to_string(), Value::NativeFunction(Arc::new(|_args| {
        Ok(Value::String(std_env::temp_dir().display().to_string()))
    })));

    // Hostname
    os_obj.insert("hostname".to_string(), Value::NativeFunction(Arc::new(|_args| {
        let host = std_env::var("HOSTNAME")
            .or_else(|_| std_env::var("COMPUTERNAME"))
            .unwrap_or_default();
        Ok(Value::String(host))
    })));

    // Username
    os_obj.insert("username".to_string(), Value::NativeFunction(Arc::new(|_args| {
        let user = std_env::var("USER")
            .or_else(|_| std_env::var("USERNAME"))
            .unwrap_or_default();
        Ok(Value::String(user))
    })));

    // CPU architecture
    os_obj.insert("arch".to_string(), Value::NativeFunction(Arc::new(|_args| {
        Ok(Value::String(std_env::consts::ARCH.to_string()))
    })));

    // Logical CPU count
    os_obj.insert("cpu_count".to_string(), Value::NativeFunction(Arc::new(|_args| {
        match std::thread::available_parallelism() {
            Ok(n) => Ok(Value::Int(n.get() as i64)),
            Err(e) => Err(format!("Failed to get CPU count: {}", e)),
        }
    })));

    // System uptime in milliseconds (Linux /proc/uptime support)
    os_obj.insert("uptime_ms".to_string(), Value::NativeFunction(Arc::new(|_args| {
        let content = fs::read_to_string("/proc/uptime")
            .map_err(|e| format!("Failed to read uptime: {}", e))?;
        let first = content
            .split_whitespace()
            .next()
            .ok_or_else(|| "Failed to parse uptime data".to_string())?;
        let secs = first
            .parse::<f64>()
            .map_err(|_| "Failed to parse uptime value".to_string())?;
        Ok(Value::Int((secs * 1000.0) as i64))
    })));

    // Resolve executable path by name (PATH search)
    os_obj.insert("which".to_string(), Value::NativeFunction(Arc::new(|args| {
        let cmd = match args.get(0) {
            Some(Value::String(s)) => s.as_str(),
            _ => return Err("which expects a command string".to_string()),
        };

        let contains_sep = cmd.contains('/') || cmd.contains('\\');
        if contains_sep {
            let p = std::path::Path::new(cmd);
            if p.exists() && p.is_file() {
                return Ok(Value::String(p.to_string_lossy().to_string()));
            }
            return Ok(Value::String(String::new()));
        }

        let path_env = std_env::var_os("PATH").unwrap_or_default();
        for dir in std_env::split_paths(&path_env) {
            let candidate: PathBuf = dir.join(cmd);
            if candidate.exists() && candidate.is_file() {
                return Ok(Value::String(candidate.to_string_lossy().to_string()));
            }
        }

        Ok(Value::String(String::new()))
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

    // Run command and capture output
    os_obj.insert("exec".to_string(), Value::NativeFunction(Arc::new(|args| {
        if !shell_execution_allowed() {
            return Err(shell_disabled_message());
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = args;
            return Err("os.exec is not available in WASM".to_string());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
        if args.is_empty() {
            return Err("exec expects command string and optional args array".to_string());
        }

        let command = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("exec expects first argument to be a command string".to_string()),
        };

        let cmd_args: Vec<String> = if let Some(Value::Array(values)) = args.get(1) {
            let mut out = Vec::with_capacity(values.len());
            for v in values {
                match v {
                    Value::String(s) => out.push(s.clone()),
                    _ => return Err("exec args array must contain only strings".to_string()),
                }
            }
            out
        } else {
            Vec::new()
        };

        let output = Command::new(&command).args(&cmd_args).output();
        match output {
            Ok(o) => {
                let mut result = HashMap::new();
                result.insert("status".to_string(), Value::Int(o.status.code().unwrap_or(-1) as i64));
                result.insert("stdout".to_string(), Value::String(String::from_utf8_lossy(&o.stdout).to_string()));
                result.insert("stderr".to_string(), Value::String(String::from_utf8_lossy(&o.stderr).to_string()));
                Ok(Value::Object(result))
            }
            Err(e) => Err(format!("exec failed for '{}': {}", command, e)),
        }
        }
    })));

    // Run command and inherit stdio (returns exit code)
    os_obj.insert("system".to_string(), Value::NativeFunction(Arc::new(|args| {
        if !shell_execution_allowed() {
            return Err(shell_disabled_message());
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = args;
            return Err("os.system is not available in WASM".to_string());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            if args.is_empty() {
                return Err("system expects command string and optional args array".to_string());
            }

            let command = match args.get(0) {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("system expects first argument to be a command string".to_string()),
            };

            let cmd_args: Vec<String> = if let Some(Value::Array(values)) = args.get(1) {
                let mut out = Vec::with_capacity(values.len());
                for v in values {
                    match v {
                        Value::String(s) => out.push(s.clone()),
                        _ => return Err("system args array must contain only strings".to_string()),
                    }
                }
                out
            } else {
                Vec::new()
            };

            match Command::new(&command).args(&cmd_args).status() {
                Ok(status) => Ok(Value::Int(status.code().unwrap_or(-1) as i64)),
                Err(e) => Err(format!("system failed for '{}': {}", command, e)),
            }
        }
    })));

    // Spawn command and return pid
    os_obj.insert("spawn".to_string(), Value::NativeFunction(Arc::new(|args| {
        if !shell_execution_allowed() {
            return Err(shell_disabled_message());
        }

        #[cfg(target_arch = "wasm32")]
        {
            let _ = args;
            return Err("os.spawn is not available in WASM".to_string());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
        if args.is_empty() {
            return Err("spawn expects command string and optional args array".to_string());
        }

        let command = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("spawn expects first argument to be a command string".to_string()),
        };

        let cmd_args: Vec<String> = if let Some(Value::Array(values)) = args.get(1) {
            let mut out = Vec::with_capacity(values.len());
            for v in values {
                match v {
                    Value::String(s) => out.push(s.clone()),
                    _ => return Err("spawn args array must contain only strings".to_string()),
                }
            }
            out
        } else {
            Vec::new()
        };

        match Command::new(&command).args(&cmd_args).spawn() {
            Ok(child) => Ok(Value::Int(child.id() as i64)),
            Err(e) => Err(format!("spawn failed for '{}': {}", command, e)),
        }
        }
    })));

    env.declare("os".to_string(), Value::Object(os_obj), true);
    Ok(())
}
