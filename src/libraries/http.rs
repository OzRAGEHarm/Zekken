use crate::environment::{Environment, Value};
use hashbrown::HashMap;
use std::sync::Arc;

#[cfg(not(target_arch = "wasm32"))]
use std::io::Read;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Duration;

#[cfg(not(target_arch = "wasm32"))]
use tiny_http::{Header, Response, Server, StatusCode};

fn http_disabled_message() -> String {
    "HTTP is disabled in this runtime.".to_string()
}

fn http_allowed() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        false
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        !matches!(std::env::var("ZEKKEN_DISABLE_HTTP"), Ok(v) if v == "1" || v.eq_ignore_ascii_case("true"))
    }
}

fn obj_from_pairs(pairs: Vec<(String, String)>) -> Value {
    let mut obj = HashMap::with_capacity(pairs.len() + 1);
    let mut keys = Vec::with_capacity(pairs.len());
    for (k, v) in pairs {
        keys.push(Value::String(k.clone()));
        obj.insert(k, Value::String(v));
    }
    obj.insert("__keys__".to_string(), Value::Array(keys));
    Value::Object(obj)
}

fn obj_string_entries(v: &Value, name: &str) -> Result<Vec<(String, String)>, String> {
    match v {
        Value::Object(map) => {
            let mut out: Vec<(String, String)> = Vec::new();
            // Prefer stable ordering.
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

            // Fallback deterministic order.
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
        _ => Err(format!("{name} expects an object of string values")),
    }
}

fn response_obj(url: String, status: i64, headers: Vec<(String, String)>, body: String) -> Value {
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

fn url_decode_str(s: &str) -> Result<String, String> {
    let bytes = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                if i + 2 >= bytes.len() {
                    return Err("Invalid percent-encoding".to_string());
                }
                let hi = hex_val(bytes[i + 1]).ok_or_else(|| "Invalid percent-encoding".to_string())?;
                let lo = hex_val(bytes[i + 2]).ok_or_else(|| "Invalid percent-encoding".to_string())?;
                out.push((hi << 4) | lo);
                i += 3;
            }
            b'+' => {
                // Useful for query-string decoding.
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).map_err(|_| "Invalid UTF-8 in decoded string".to_string())
}

fn default_content_type_for_body(body: &str) -> &'static str {
    let t = body.trim_start();
    let lower_prefix = t
        .get(0..t.len().min(16))
        .unwrap_or("")
        .to_ascii_lowercase();

    if lower_prefix.starts_with("<!doctype") || lower_prefix.starts_with("<html") {
        "text/html; charset=utf-8"
    } else if t.starts_with('{') || t.starts_with('[') {
        "application/json; charset=utf-8"
    } else {
        "text/plain; charset=utf-8"
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn do_request(
    method: &str,
    url: &str,
    headers: Vec<(String, String)>,
    body: Option<String>,
    timeout_ms: Option<i64>,
) -> Result<Value, String> {
    if !http_allowed() {
        return Err(http_disabled_message());
    }

    let mut agent_builder = ureq::AgentBuilder::new();
    if let Some(ms) = timeout_ms {
        if ms < 0 {
            return Err("timeout_ms must be >= 0".to_string());
        }
        let d = Duration::from_millis(ms as u64);
        agent_builder = agent_builder.timeout_connect(d).timeout_read(d).timeout_write(d);
    }
    let agent = agent_builder.build();

    let mut req = agent.request(method, url);
    for (k, v) in headers {
        req = req.set(&k, &v);
    }

    let res = match body {
        Some(b) => req.send_string(&b),
        None => req.call(),
    };

    let response = match res {
        Ok(r) => r,
        Err(ureq::Error::Status(_code, r)) => r,
        Err(e) => return Err(format!("http request failed: {}", e)),
    };

    let status = response.status() as i64;

    let mut header_pairs: Vec<(String, String)> = Vec::new();
    for name in response.headers_names() {
        if let Some(v) = response.header(&name) {
            header_pairs.push((name.to_string(), v.to_string()));
        }
    }
    header_pairs.sort_by(|a, b| a.0.cmp(&b.0));

    let body_str = response.into_string().unwrap_or_default();
    Ok(response_obj(url.to_string(), status, header_pairs, body_str))
}

#[cfg(target_arch = "wasm32")]
fn do_request(
    _method: &str,
    _url: &str,
    _headers: Vec<(String, String)>,
    _body: Option<String>,
    _timeout_ms: Option<i64>,
) -> Result<Value, String> {
    Err("http.request is not available in WASM".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn do_serve(addr: &str, routes: HashMap<String, Route>) -> Result<Value, String> {
    if !http_allowed() {
        return Err(http_disabled_message());
    }

    let server = Server::http(addr).map_err(|e| format!("Failed to bind HTTP server: {}", e))?;
    for req in server.incoming_requests() {
        let method = req.method().as_str().to_string();
        let url = req.url().to_string();
        let path = url.split('?').next().unwrap_or(&url).to_string();

        let key_method_path = format!("{} {}", method, path);
        let route = routes
            .get(&key_method_path)
            .or_else(|| routes.get(&path))
            .or_else(|| routes.get("__default__"));

        let (status, headers, body) = match route {
            Some(Route::Plain(s)) => (
                200u16,
                vec![("content-type".to_string(), default_content_type_for_body(s).to_string())],
                s.clone(),
            ),
            Some(Route::Resp(r)) => (r.status, r.headers.clone(), r.body.clone()),
            None => (404u16, vec![("content-type".to_string(), "text/plain; charset=utf-8".to_string())], "Not Found".to_string()),
        };

        let mut response = Response::from_string(body).with_status_code(StatusCode(status));
        for (k, v) in headers {
            if let Ok(h) = Header::from_bytes(k.as_bytes(), v.as_bytes()) {
                response = response.with_header(h);
            }
        }

        let _ = req.respond(response);
    }

    Ok(Value::Void)
}

#[cfg(target_arch = "wasm32")]
fn do_serve(_addr: &str, _routes: HashMap<String, Route>) -> Result<Value, String> {
    Err("http.serve is not available in WASM".to_string())
}

#[derive(Clone)]
enum Route {
    Plain(String),
    Resp(RouteResp),
}

#[derive(Clone)]
struct RouteResp {
    status: u16,
    headers: Vec<(String, String)>,
    body: String,
}

fn parse_routes(v: &Value) -> Result<HashMap<String, Route>, String> {
    let map = match v {
        Value::Object(m) => m,
        _ => return Err("http.serve expects an object of routes".to_string()),
    };

    let mut out: HashMap<String, Route> = HashMap::new();
    for (k, val) in map {
        if k == "__keys__" {
            continue;
        }
        let route = match val {
            Value::String(s) => Route::Plain(s.clone()),
            Value::Object(obj) => {
                let status = match obj.get("status") {
                    Some(Value::Int(i)) => (*i).clamp(100, 599) as u16,
                    _ => 200u16,
                };
                let body = match obj.get("body") {
                    Some(Value::String(s)) => s.clone(),
                    Some(other) => other.to_string(),
                    None => String::new(),
                };
                let headers = match obj.get("headers") {
                    Some(h) => obj_string_entries(h, "http.serve routes.headers")?,
                    None => Vec::new(),
                };
                Route::Resp(RouteResp { status, headers, body })
            }
            other => Route::Plain(other.to_string()),
        };
        out.insert(k.clone(), route);
    }
    Ok(out)
}

#[cfg(not(target_arch = "wasm32"))]
fn request_obj(
    id: i64,
    method: String,
    url: String,
    headers: Vec<(String, String)>,
    body: String,
) -> Value {
    let mut obj = HashMap::with_capacity(8);
    let mut keys = Vec::with_capacity(7);

    let path = url.split('?').next().unwrap_or(&url).to_string();
    let query = url.splitn(2, '?').nth(1).unwrap_or("").to_string();

    keys.push(Value::String("id".to_string()));
    obj.insert("id".to_string(), Value::Int(id));

    keys.push(Value::String("method".to_string()));
    obj.insert("method".to_string(), Value::String(method));

    keys.push(Value::String("url".to_string()));
    obj.insert("url".to_string(), Value::String(url));

    keys.push(Value::String("path".to_string()));
    obj.insert("path".to_string(), Value::String(path));

    keys.push(Value::String("query".to_string()));
    obj.insert("query".to_string(), Value::String(query));

    keys.push(Value::String("headers".to_string()));
    obj.insert("headers".to_string(), obj_from_pairs(headers));

    keys.push(Value::String("body".to_string()));
    obj.insert("body".to_string(), Value::String(body));

    obj.insert("__keys__".to_string(), Value::Array(keys));
    Value::Object(obj)
}

#[cfg(not(target_arch = "wasm32"))]
struct ServerState {
    server: Server,
    next_id: i64,
    pending: HashMap<i64, tiny_http::Request>,
}

#[cfg(not(target_arch = "wasm32"))]
fn response_from_value(v: &Value) -> (u16, Vec<(String, String)>, String) {
    match v {
        Value::String(s) => (
            200u16,
            vec![("content-type".to_string(), default_content_type_for_body(s).to_string())],
            s.clone(),
        ),
        Value::Object(obj) => {
            let status = match obj.get("status") {
                Some(Value::Int(i)) => (*i).clamp(100, 599) as u16,
                _ => 200u16,
            };
            let body = match obj.get("body") {
                Some(Value::String(s)) => s.clone(),
                Some(other) => other.to_string(),
                None => String::new(),
            };
            let mut headers = match obj.get("headers") {
                Some(h) => obj_string_entries(h, "http response headers").unwrap_or_default(),
                None => Vec::new(),
            };

            let has_ct = headers.iter().any(|(k, _)| k.eq_ignore_ascii_case("content-type"));
            if !has_ct {
                headers.push(("content-type".to_string(), default_content_type_for_body(&body).to_string()));
            }

            (status, headers, body)
        }
        other => (
            200u16,
            vec![("content-type".to_string(), default_content_type_for_body(&other.to_string()).to_string())],
            other.to_string(),
        ),
    }
}

pub fn register(env: &mut Environment) -> Result<(), String> {
    let mut http_obj: HashMap<String, Value> = HashMap::new();

    http_obj.insert("build_query".to_string(), Value::NativeFunction(Arc::new(|args| {
        let entries = match args.get(0) {
            Some(v) => obj_string_entries(v, "http.build_query")?,
            None => return Err("http.build_query expects an object".to_string()),
        };
        let mut parts: Vec<String> = Vec::with_capacity(entries.len());
        for (k, v) in entries {
            parts.push(format!("{}={}", url_encode_str(&k), url_encode_str(&v)));
        }
        Ok(Value::String(parts.join("&")))
    })));

    http_obj.insert("parse_query".to_string(), Value::NativeFunction(Arc::new(|args| {
        let s = match args.get(0) {
            Some(Value::String(s)) => s.as_str(),
            _ => return Err("http.parse_query expects a string".to_string()),
        };
        let mut pairs: Vec<(String, String)> = Vec::new();
        for part in s.trim_start_matches('?').split('&') {
            if part.is_empty() {
                continue;
            }
            let mut it = part.splitn(2, '=');
            let k = it.next().unwrap_or("");
            let v = it.next().unwrap_or("");
            pairs.push((url_decode_str(k)?, url_decode_str(v)?));
        }
        Ok(obj_from_pairs(pairs))
    })));

    http_obj.insert("request".to_string(), Value::NativeFunction(Arc::new(|args| {
        let method = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("http.request expects method as string".to_string()),
        };
        let url = match args.get(1) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("http.request expects url as string".to_string()),
        };
        let mut headers: Vec<(String, String)> = Vec::new();
        let mut body: Option<String> = None;
        let mut timeout_ms: Option<i64> = None;

        if let Some(v) = args.get(2) {
            if !matches!(v, Value::Void) {
                headers = obj_string_entries(v, "http.request headers")?;
            }
        }
        if let Some(v) = args.get(3) {
            if !matches!(v, Value::Void) {
                body = Some(match v {
                    Value::String(s) => s.clone(),
                    other => other.to_string(),
                });
            }
        }
        if let Some(Value::Int(ms)) = args.get(4) {
            timeout_ms = Some(*ms);
        }

        do_request(&method, &url, headers, body, timeout_ms)
    })));

    http_obj.insert("get_json".to_string(), Value::NativeFunction(Arc::new(|args| {
        let url = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("http.get_json expects url as string".to_string()),
        };
        let headers = match args.get(1) {
            Some(Value::Object(_)) => obj_string_entries(&args[1], "http.get_json headers")?,
            _ => Vec::new(),
        };
        let timeout_ms = match args.get(2) {
            Some(Value::Int(ms)) => Some(*ms),
            _ => None,
        };

        let resp = do_request("GET", &url, headers, None, timeout_ms)?;
        let body = match &resp {
            Value::Object(obj) => match obj.get("body") {
                Some(Value::String(s)) => s.as_str(),
                _ => "",
            },
            _ => "",
        };

        match serde_json::from_str::<serde_json::Value>(body) {
            Ok(json) => Ok(crate::environment::json_to_zekken(&json)),
            Err(e) => Err(format!("JSON parse error: {}", e)),
        }
    })));

    http_obj.insert("get".to_string(), Value::NativeFunction(Arc::new(|args| {
        let url = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("http.get expects url as string".to_string()),
        };
        let headers = match args.get(1) {
            Some(Value::Object(_)) => obj_string_entries(&args[1], "http.get headers")?,
            _ => Vec::new(),
        };
        let timeout_ms = match args.get(2) {
            Some(Value::Int(ms)) => Some(*ms),
            _ => None,
        };
        do_request("GET", &url, headers, None, timeout_ms)
    })));

    http_obj.insert("post".to_string(), Value::NativeFunction(Arc::new(|args| {
        let url = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("http.post expects url as string".to_string()),
        };
        let body = match args.get(1) {
            Some(Value::String(s)) => Some(s.clone()),
            Some(Value::Void) | None => None,
            Some(other) => Some(other.to_string()),
        };
        let headers = match args.get(2) {
            Some(Value::Object(_)) => obj_string_entries(&args[2], "http.post headers")?,
            _ => Vec::new(),
        };
        let timeout_ms = match args.get(3) {
            Some(Value::Int(ms)) => Some(*ms),
            _ => None,
        };
        do_request("POST", &url, headers, body, timeout_ms)
    })));

    http_obj.insert("serve".to_string(), Value::NativeFunction(Arc::new(|args| {
        let addr = match args.get(0) {
            Some(Value::String(s)) => s.clone(),
            _ => return Err("http.serve expects address as string".to_string()),
        };
        let routes_val = args.get(1).ok_or_else(|| "http.serve expects routes object".to_string())?;
        let routes = parse_routes(routes_val)?;
        do_serve(&addr, routes)
    })));

    http_obj.insert("listen".to_string(), Value::NativeFunction(Arc::new(|args| {
        if !http_allowed() {
            return Err(http_disabled_message());
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = args;
            return Err("http.listen is not available in WASM".to_string());
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            let addr = match args.get(0) {
                Some(Value::String(s)) => s.as_str(),
                _ => return Err("http.listen expects address as string".to_string()),
            };
            let addr_str = addr.to_string();

            let server = Server::http(addr).map_err(|e| format!("Failed to bind HTTP server: {}", e))?;
            let state = Arc::new(std::sync::Mutex::new(ServerState {
                server,
                next_id: 1,
                pending: HashMap::new(),
            }));

            let mut obj: HashMap<String, Value> = HashMap::with_capacity(6);

            {
                let st = state.clone();
                obj.insert("accept".to_string(), Value::NativeFunction(Arc::new(move |args| {
                    let timeout_ms = match args.get(0) {
                        Some(Value::Int(ms)) => Some(*ms),
                        Some(Value::Void) | None => None,
                        Some(_) => return Err("http_server.accept timeout_ms must be int".to_string()),
                    };

                    let req_opt = {
                        let guard = st.lock().map_err(|_| "http server lock poisoned".to_string())?;
                        if let Some(ms) = timeout_ms {
                            if ms < 0 {
                                return Err("timeout_ms must be >= 0".to_string());
                            }
                            guard
                                .server
                                .recv_timeout(Duration::from_millis(ms as u64))
                                .map_err(|e| format!("accept failed: {}", e))?
                        } else {
                            Some(guard.server.recv().map_err(|e| format!("accept failed: {}", e))?)
                        }
                    };

                    let mut req = match req_opt {
                        Some(r) => r,
                        None => return Ok(Value::Void),
                    };

                    // Normalize for scripts (and to avoid tiny_http formatting differences).
                    let method_str = req.method().as_str().to_ascii_uppercase();
                    let url_str = req.url().to_string();

                    // Read body with a soft cap (default 1MB) to avoid OOM.
                    let cap = std::env::var("ZEKKEN_HTTP_MAX_BODY_BYTES")
                        .ok()
                        .and_then(|v| v.parse::<usize>().ok())
                        .unwrap_or(1024 * 1024);

                    let body = {
                        let reader = req.as_reader();
                        let mut buf = Vec::new();
                        reader
                            .take(cap as u64)
                            .read_to_end(&mut buf)
                            .map_err(|e| format!("Failed to read request body: {}", e))?;
                        String::from_utf8_lossy(&buf).to_string()
                    };

                    let headers: Vec<(String, String)> = req
                        .headers()
                        .iter()
                        .map(|h| (h.field.as_str().to_string(), h.value.as_str().to_string()))
                        .collect();

                    let id = {
                        let mut guard = st.lock().map_err(|_| "http server lock poisoned".to_string())?;
                        let id = guard.next_id;
                        guard.next_id += 1;
                        guard.pending.insert(id, req);
                        id
                    };

                    Ok(request_obj(
                        id,
                        method_str,
                        url_str,
                        headers,
                        body,
                    ))
                })));
            }

            {
                let st = state.clone();
                obj.insert("respond".to_string(), Value::NativeFunction(Arc::new(move |args| {
                    if args.len() != 2 {
                        return Err("http_server.respond expects (id, resp)".to_string());
                    }
                    let id = match &args[0] {
                        Value::Int(i) => *i,
                        _ => return Err("http_server.respond id must be int".to_string()),
                    };
                    let (status, headers, body) = response_from_value(&args[1]);

                    let req = {
                        let mut guard = st.lock().map_err(|_| "http server lock poisoned".to_string())?;
                        guard.pending.remove(&id)
                    };
                    let req = req.ok_or_else(|| format!("Unknown request id {}", id))?;

                    let mut response = Response::from_string(body).with_status_code(StatusCode(status));
                    for (k, v) in headers {
                        if let Ok(h) = Header::from_bytes(k.as_bytes(), v.as_bytes()) {
                            response = response.with_header(h);
                        }
                    }
                    req.respond(response).map_err(|e| format!("respond failed: {}", e))?;
                    Ok(Value::Void)
                })));
            }

            obj.insert("addr".to_string(), Value::NativeFunction(Arc::new(move |_args| {
                Ok(Value::String(addr_str.clone()))
            })));

            let mut keys: Vec<Value> = obj.keys().cloned().map(Value::String).collect();
            keys.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
            obj.insert("__keys__".to_string(), Value::Array(keys));
            Ok(Value::Object(obj))
        }
    })));

    let mut keys: Vec<Value> = http_obj.keys().cloned().map(Value::String).collect();
    keys.sort_by(|a, b| a.to_string().cmp(&b.to_string()));
    http_obj.insert("__keys__".to_string(), Value::Array(keys));

    env.declare("http".to_string(), Value::Object(http_obj), true);
    Ok(())
}
