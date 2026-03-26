use crate::ast::Location;
use crate::environment::{Environment, Value};
use crate::errors::ZekkenError;

#[derive(Clone, Copy)]
pub enum OsOpCode {
    Cwd,
    Ls,
    Env,
    SetEnv,
    RemoveEnv,
    Platform,
    Args,
    HomeDir,
    TempDir,
    Hostname,
    Username,
    Arch,
    CpuCount,
    UptimeMs,
    Which,
    Exit,
    Pid,
    Sleep,
    Exec,
    System,
    Spawn,
}

impl OsOpCode {
    #[inline]
    pub fn from_method(name: &str) -> Option<Self> {
        match name {
            "cwd" => Some(Self::Cwd),
            "ls" => Some(Self::Ls),
            "env" => Some(Self::Env),
            "set_env" => Some(Self::SetEnv),
            "remove_env" => Some(Self::RemoveEnv),
            "platform" => Some(Self::Platform),
            "args" => Some(Self::Args),
            "home_dir" => Some(Self::HomeDir),
            "temp_dir" => Some(Self::TempDir),
            "hostname" => Some(Self::Hostname),
            "username" => Some(Self::Username),
            "arch" => Some(Self::Arch),
            "cpu_count" => Some(Self::CpuCount),
            "uptime_ms" => Some(Self::UptimeMs),
            "which" => Some(Self::Which),
            "exit" => Some(Self::Exit),
            "pid" => Some(Self::Pid),
            "sleep" => Some(Self::Sleep),
            "exec" => Some(Self::Exec),
            "system" => Some(Self::System),
            "spawn" => Some(Self::Spawn),
            _ => None,
        }
    }

    #[inline]
    fn method_name(self) -> &'static str {
        match self {
            Self::Cwd => "cwd",
            Self::Ls => "ls",
            Self::Env => "env",
            Self::SetEnv => "set_env",
            Self::RemoveEnv => "remove_env",
            Self::Platform => "platform",
            Self::Args => "args",
            Self::HomeDir => "home_dir",
            Self::TempDir => "temp_dir",
            Self::Hostname => "hostname",
            Self::Username => "username",
            Self::Arch => "arch",
            Self::CpuCount => "cpu_count",
            Self::UptimeMs => "uptime_ms",
            Self::Which => "which",
            Self::Exit => "exit",
            Self::Pid => "pid",
            Self::Sleep => "sleep",
            Self::Exec => "exec",
            Self::System => "system",
            Self::Spawn => "spawn",
        }
    }

    pub fn eval(self, args: Vec<Value>, env: &mut Environment, location: &Location) -> Result<Value, ZekkenError> {
        dispatch_library_native("os", self.method_name(), args, env, location)
    }
}

fn dispatch_library_native(
    lib_name: &str,
    method_name: &str,
    args: Vec<Value>,
    env: &mut Environment,
    location: &Location,
) -> Result<Value, ZekkenError> {
    let native = match env.lookup_ref(lib_name) {
        Some(Value::Object(map)) => match map.get(method_name) {
            Some(Value::NativeFunction(native)) => Some(native.clone()),
            _ => None,
        },
        _ => None,
    }
    .ok_or_else(|| {
        ZekkenError::runtime(
            &format!("Native method '{}.{}' not found", lib_name, method_name),
            location.line,
            location.column,
            None,
        )
    })?;

    native(args).map_err(|msg| ZekkenError::runtime(&msg, location.line, location.column, None))
}
