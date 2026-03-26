use crate::ast::Location;
use crate::environment::{Environment, Value};
use crate::errors::ZekkenError;

#[derive(Clone, Copy)]
pub enum EncodingOpCode {
    Base64Encode,
    Base64Decode,
    HexEncode,
    HexDecode,
    UrlEncode,
    UrlDecode,
}

impl EncodingOpCode {
    #[inline]
    pub fn from_method(name: &str) -> Option<Self> {
        match name {
            "base64_encode" => Some(Self::Base64Encode),
            "base64_decode" => Some(Self::Base64Decode),
            "hex_encode" => Some(Self::HexEncode),
            "hex_decode" => Some(Self::HexDecode),
            "url_encode" => Some(Self::UrlEncode),
            "url_decode" => Some(Self::UrlDecode),
            _ => None,
        }
    }

    #[inline]
    fn method_name(self) -> &'static str {
        match self {
            Self::Base64Encode => "base64_encode",
            Self::Base64Decode => "base64_decode",
            Self::HexEncode => "hex_encode",
            Self::HexDecode => "hex_decode",
            Self::UrlEncode => "url_encode",
            Self::UrlDecode => "url_decode",
        }
    }

    pub fn eval(self, args: Vec<Value>, env: &mut Environment, location: &Location) -> Result<Value, ZekkenError> {
        dispatch_library_native("encoding", self.method_name(), args, env, location)
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

