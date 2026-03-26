use crate::ast::Location;
use crate::environment::{Environment, Value};
use crate::errors::ZekkenError;

#[derive(Clone, Copy)]
pub enum PathOpCode {
    Join,
    Normalize,
    Resolve,
    Basename,
    Dirname,
    Extname,
    Stem,
    IsAbs,
    Relative,
}

impl PathOpCode {
    #[inline]
    pub fn from_method(name: &str) -> Option<Self> {
        match name {
            "join" => Some(Self::Join),
            "normalize" => Some(Self::Normalize),
            "resolve" => Some(Self::Resolve),
            "basename" => Some(Self::Basename),
            "dirname" => Some(Self::Dirname),
            "extname" => Some(Self::Extname),
            "stem" => Some(Self::Stem),
            "is_abs" => Some(Self::IsAbs),
            "relative" => Some(Self::Relative),
            _ => None,
        }
    }

    #[inline]
    fn method_name(self) -> &'static str {
        match self {
            Self::Join => "join",
            Self::Normalize => "normalize",
            Self::Resolve => "resolve",
            Self::Basename => "basename",
            Self::Dirname => "dirname",
            Self::Extname => "extname",
            Self::Stem => "stem",
            Self::IsAbs => "is_abs",
            Self::Relative => "relative",
        }
    }

    pub fn eval(self, args: Vec<Value>, env: &mut Environment, location: &Location) -> Result<Value, ZekkenError> {
        dispatch_library_native("path", self.method_name(), args, env, location)
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

