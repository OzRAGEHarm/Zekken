use crate::ast::Location;
use crate::environment::{Environment, Value};
use crate::errors::ZekkenError;

#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use ureq;

#[derive(Clone, Copy)]
pub enum HttpOpCode {
    BuildQuery,
    ParseQuery,
    Get,
}

impl HttpOpCode {
    #[inline]
    pub fn from_method(name: &str) -> Option<Self> {
        match name {
            "build_query" => Some(Self::BuildQuery),
            "parse_query" => Some(Self::ParseQuery),
            "get" => Some(Self::Get),
            _ => None,
        }
    }

    pub fn eval(self, args: Vec<Value>, _env: &mut Environment, location: &Location) -> Result<Value, ZekkenError> {
        match self {
            Self::BuildQuery => {
                require_argc(&args, 1, location)?;
                let s = build_query(&args[0], location)?;
                Ok(Value::String(s))
            }
            Self::ParseQuery => {
                require_argc(&args, 1, location)?;
                let qs = match &args[0] {
                    Value::String(s) => s.as_str(),
                    _ => {
                        return Err(ZekkenError::type_error(
                            "http.parse_query expects a string",
                            "string",
                            value_type_name(&args[0]),
                            location.line,
                            location.column,
                        ))
                    }
                };
                parse_query(qs, location)
            }
            Self::Get => {
                // Signature: http.get(url: string, headers?: obj, timeout_ms?: int)
                if args.is_empty() || args.len() > 3 {
                    return Err(ZekkenError::runtime(
                        "http.get expects 1..=3 arguments",
                        location.line,
                        location.column,
                        Some("argument mismatch"),
                    ));
                }
                let url = match &args[0] {
                    Value::String(s) => s.clone(),
                    _ => {
                        return Err(ZekkenError::type_error(
                            "http.get expects url as string",
                            "string",
                            value_type_name(&args[0]),
                            location.line,
                            location.column,
                        ))
                    }
                };

                let headers = if args.len() >= 2 && !matches!(args[1], Value::Void) {
                    obj_string_entries(&args[1], location)?
                } else {
                    Vec::new()
                };

                let timeout_ms = if args.len() == 3 {
                    match &args[2] {
                        Value::Int(i) => Some(*i),
                        Value::Void => None,
                        _ => {
                            return Err(ZekkenError::type_error(
                                "http.get timeout_ms must be int",
                                "int",
                                value_type_name(&args[2]),
                                location.line,
                                location.column,
                            ))
                        }
                    }
                } else {
                    None
                };

                http_get(&url, headers, timeout_ms, location)
            }
        }
    }
}

#[inline]
fn require_argc(args: &[Value], expected: usize, location: &Location) -> Result<(), ZekkenError> {
    if args.len() == expected {
        return Ok(());
    }
    Err(ZekkenError::runtime(
        &format!("Expected {} argument{}", expected, if expected == 1 { "" } else { "s" }),
        location.line,
        location.column,
        Some("argument mismatch"),
    ))
}

#[inline]
fn value_type_name(value: &Value) -> &'static str {
    match value {
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Boolean(_) => "bool",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Function(_) => "function",
        Value::NativeFunction(_) => "native function",
        Value::Complex { .. } => "complex",
        Value::Vector(_) => "vector",
        Value::Matrix(_) => "matrix",
        Value::Void => "void",
    }
}

fn url_encode_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.as_bytes() {
        let ch = *b as char;
        let unreserved = ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' || ch == '.' || ch == '~';
        if unreserved {
            out.push(ch);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(10 + (b - b'a')),
        b'A'..=b'F' => Some(10 + (b - b'A')),
        _ => None,
    }
}

fn url_decode_str(s: &str, location: &Location) -> Result<String, ZekkenError> {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                if i + 2 >= bytes.len() {
                    return Err(ZekkenError::runtime(
                        "Invalid percent-encoding",
                        location.line,
                        location.column,
                        Some("decode"),
                    ));
                }
                let hi = hex_val(bytes[i + 1]).ok_or_else(|| {
                    ZekkenError::runtime("Invalid percent-encoding", location.line, location.column, Some("decode"))
                })?;
                let lo = hex_val(bytes[i + 2]).ok_or_else(|| {
                    ZekkenError::runtime("Invalid percent-encoding", location.line, location.column, Some("decode"))
                })?;
                out.push((hi << 4) | lo);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| {
        ZekkenError::runtime(
            "Invalid UTF-8 in decoded string",
            location.line,
            location.column,
            Some("decode"),
        )
    })
}

fn obj_string_entries(v: &Value, location: &Location) -> Result<Vec<(String, String)>, ZekkenError> {
    match v {
        Value::Object(map) => {
            let mut out: Vec<(String, String)> = Vec::new();
            if let Some(Value::Array(keys)) = map.get("__keys__") {
                for kv in keys {
                    if let Value::String(k) = kv {
                        if k == "__keys__" {
                            continue;
                        }
                        if let Some(val) = map.get(k) {
                            let s = match val {
                                Value::String(s) => s.clone(),
                                other => other.to_string(),
                            };
                            out.push((k.clone(), s));
                        }
                    }
                }
                return Ok(out);
            }
            let mut keys: Vec<&String> = map.keys().filter(|k| k.as_str() != "__keys__").collect();
            keys.sort_unstable();
            for k in keys {
                if let Some(val) = map.get(k) {
                    let s = match val {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    out.push((k.clone(), s));
                }
            }
            Ok(out)
        }
        _ => Err(ZekkenError::type_error(
            "Expected object",
            "obj",
            value_type_name(v),
            location.line,
            location.column,
        )),
    }
}

fn obj_from_pairs(pairs: Vec<(String, String)>) -> Value {
    use hashbrown::HashMap;
    let mut obj = HashMap::with_capacity(pairs.len() + 1);
    let mut keys = Vec::with_capacity(pairs.len());
    for (k, v) in pairs {
        keys.push(Value::String(k.clone()));
        obj.insert(k, Value::String(v));
    }
    obj.insert("__keys__".to_string(), Value::Array(keys));
    Value::Object(obj)
}

fn build_query(obj: &Value, location: &Location) -> Result<String, ZekkenError> {
    let entries = obj_string_entries(obj, location)?;
    let mut parts: Vec<String> = Vec::with_capacity(entries.len());
    for (k, v) in entries {
        parts.push(format!("{}={}", url_encode_str(&k), url_encode_str(&v)));
    }
    Ok(parts.join("&"))
}

fn parse_query(qs: &str, location: &Location) -> Result<Value, ZekkenError> {
    let mut pairs: Vec<(String, String)> = Vec::new();
    for part in qs.trim_start_matches('?').split('&') {
        if part.is_empty() {
            continue;
        }
        let mut it = part.splitn(2, '=');
        let k = it.next().unwrap_or("");
        let v = it.next().unwrap_or("");
        let dk = url_decode_str(k, location)?;
        let dv = url_decode_str(v, location)?;
        pairs.push((dk, dv));
    }
    // Stable order for printing/formatting.
    pairs.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(obj_from_pairs(pairs))
}

fn response_obj(url: String, status: i64, headers: Vec<(String, String)>, body: String) -> Value {
    use hashbrown::HashMap;
    let ok = status >= 200 && status < 300;
    let mut obj = HashMap::with_capacity(6);
    let mut keys = Vec::with_capacity(5);

    keys.push(Value::String("url".to_string()));
    obj.insert("url".to_string(), Value::String(url));

    keys.push(Value::String("status".to_string()));
    obj.insert("status".to_string(), Value::Int(status));

    keys.push(Value::String("ok".to_string()));
    obj.insert("ok".to_string(), Value::Boolean(ok));

    keys.push(Value::String("headers".to_string()));
    obj.insert("headers".to_string(), obj_from_pairs(headers));

    keys.push(Value::String("body".to_string()));
    obj.insert("body".to_string(), Value::String(body));

    obj.insert("__keys__".to_string(), Value::Array(keys));
    Value::Object(obj)
}

fn http_get(
    url: &str,
    headers: Vec<(String, String)>,
    timeout_ms: Option<i64>,
    location: &Location,
) -> Result<Value, ZekkenError> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = (url, headers, timeout_ms, location);
        return Err(ZekkenError::runtime(
            "http.get is not available in WASM",
            location.line,
            location.column,
            None,
        ));
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        let mut agent_builder = ureq::AgentBuilder::new();
        if let Some(ms) = timeout_ms {
            if ms < 0 {
                return Err(ZekkenError::runtime(
                    "timeout_ms must be >= 0",
                    location.line,
                    location.column,
                    None,
                ));
            }
            let d = Duration::from_millis(ms as u64);
            agent_builder = agent_builder.timeout_connect(d).timeout_read(d).timeout_write(d);
        }
        let agent = agent_builder.build();
        let mut req = agent.get(url);
        for (k, v) in headers {
            req = req.set(&k, &v);
        }
        let res = req.call();
        let response = match res {
            Ok(r) => r,
            Err(ureq::Error::Status(_code, r)) => r,
            Err(e) => {
                return Err(ZekkenError::runtime(
                    &format!("http request failed: {}", e),
                    location.line,
                    location.column,
                    None,
                ))
            }
        };
        let status = response.status() as i64;

        let mut header_pairs: Vec<(String, String)> = Vec::new();
        for name in response.headers_names() {
            if let Some(v) = response.header(&name) {
                header_pairs.push((name.to_string(), v.to_string()));
            }
        }
        header_pairs.sort_by(|a, b| a.0.cmp(&b.0));

        let body = response.into_string().unwrap_or_default();
        Ok(response_obj(url.to_string(), status, header_pairs, body))
    }
}
