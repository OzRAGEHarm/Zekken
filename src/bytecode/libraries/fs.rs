use crate::ast::Location;
use crate::environment::{Environment, Value};
use crate::errors::ZekkenError;

#[derive(Clone, Copy)]
pub enum FsOpCode {
    ReadFile,
    WriteFile,
    AppendFile,
    ReadDir,
    ReadLines,
    CreateDir,
    RemoveDir,
    Exists,
    IsFile,
    IsDir,
    RemoveFile,
    CopyFile,
    Rename,
    Stat,
}

impl FsOpCode {
    #[inline]
    pub fn from_method(name: &str) -> Option<Self> {
        match name {
            "read_file" => Some(Self::ReadFile),
            "write_file" => Some(Self::WriteFile),
            "append_file" => Some(Self::AppendFile),
            "read_dir" => Some(Self::ReadDir),
            "read_lines" => Some(Self::ReadLines),
            "create_dir" => Some(Self::CreateDir),
            "remove_dir" => Some(Self::RemoveDir),
            "exists" => Some(Self::Exists),
            "is_file" => Some(Self::IsFile),
            "is_dir" => Some(Self::IsDir),
            "remove_file" => Some(Self::RemoveFile),
            "copy_file" => Some(Self::CopyFile),
            "rename" => Some(Self::Rename),
            "stat" => Some(Self::Stat),
            _ => None,
        }
    }

    #[inline]
    fn method_name(self) -> &'static str {
        match self {
            Self::ReadFile => "read_file",
            Self::WriteFile => "write_file",
            Self::AppendFile => "append_file",
            Self::ReadDir => "read_dir",
            Self::ReadLines => "read_lines",
            Self::CreateDir => "create_dir",
            Self::RemoveDir => "remove_dir",
            Self::Exists => "exists",
            Self::IsFile => "is_file",
            Self::IsDir => "is_dir",
            Self::RemoveFile => "remove_file",
            Self::CopyFile => "copy_file",
            Self::Rename => "rename",
            Self::Stat => "stat",
        }
    }

    pub fn eval(self, args: Vec<Value>, env: &mut Environment, location: &Location) -> Result<Value, ZekkenError> {
        dispatch_library_native("fs", self.method_name(), args, env, location)
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
