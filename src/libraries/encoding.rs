use crate::environment::{Environment, Value};
use hashbrown::HashMap;
use std::sync::Arc;

const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn expect_string_arg(args: &[Value], fn_name: &str) -> Result<String, String> {
    if args.len() != 1 {
        return Err(format!("{} expects exactly one string argument", fn_name));
    }
    match &args[0] {
        Value::String(s) => Ok(s.clone()),
        _ => Err(format!("{} expects a string argument", fn_name)),
    }
}

fn base64_encode_bytes(input: &[u8]) -> String {
    let mut out = String::new();
    let mut i = 0usize;
    while i < input.len() {
        let b0 = input[i];
        let b1 = if i + 1 < input.len() { input[i + 1] } else { 0 };
        let b2 = if i + 2 < input.len() { input[i + 2] } else { 0 };

        let idx0 = (b0 >> 2) as usize;
        let idx1 = (((b0 & 0b0000_0011) << 4) | (b1 >> 4)) as usize;
        let idx2 = (((b1 & 0b0000_1111) << 2) | (b2 >> 6)) as usize;
        let idx3 = (b2 & 0b0011_1111) as usize;

        out.push(BASE64_TABLE[idx0] as char);
        out.push(BASE64_TABLE[idx1] as char);
        if i + 1 < input.len() {
            out.push(BASE64_TABLE[idx2] as char);
        } else {
            out.push('=');
        }
        if i + 2 < input.len() {
            out.push(BASE64_TABLE[idx3] as char);
        } else {
            out.push('=');
        }
        i += 3;
    }
    out
}

fn base64_value(c: u8) -> Option<u8> {
    match c {
        b'A'..=b'Z' => Some(c - b'A'),
        b'a'..=b'z' => Some(c - b'a' + 26),
        b'0'..=b'9' => Some(c - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn base64_decode_string(input: &str) -> Result<Vec<u8>, String> {
    let cleaned = input.trim().as_bytes();
    if cleaned.is_empty() {
        return Ok(Vec::new());
    }
    if cleaned.len() % 4 != 0 {
        return Err("Invalid base64 length".to_string());
    }

    let mut out = Vec::with_capacity((cleaned.len() / 4) * 3);
    let mut i = 0usize;
    while i < cleaned.len() {
        let c0 = cleaned[i];
        let c1 = cleaned[i + 1];
        let c2 = cleaned[i + 2];
        let c3 = cleaned[i + 3];

        let v0 = base64_value(c0).ok_or_else(|| "Invalid base64 character".to_string())?;
        let v1 = base64_value(c1).ok_or_else(|| "Invalid base64 character".to_string())?;
        let v2 = if c2 == b'=' {
            0
        } else {
            base64_value(c2).ok_or_else(|| "Invalid base64 character".to_string())?
        };
        let v3 = if c3 == b'=' {
            0
        } else {
            base64_value(c3).ok_or_else(|| "Invalid base64 character".to_string())?
        };

        out.push((v0 << 2) | (v1 >> 4));
        if c2 != b'=' {
            out.push(((v1 & 0b0000_1111) << 4) | (v2 >> 2));
        }
        if c3 != b'=' {
            out.push(((v2 & 0b0000_0011) << 6) | v3);
        }
        i += 4;
    }

    Ok(out)
}

fn is_unreserved_url_byte(b: u8) -> bool {
    b.is_ascii_uppercase()
        || b.is_ascii_lowercase()
        || b.is_ascii_digit()
        || matches!(b, b'-' | b'_' | b'.' | b'~')
}

fn hex_digit(n: u8) -> char {
    match n {
        0..=9 => (b'0' + n) as char,
        _ => (b'A' + (n - 10)) as char,
    }
}

fn parse_hex_nibble(c: u8) -> Option<u8> {
    match c {
        b'0'..=b'9' => Some(c - b'0'),
        b'a'..=b'f' => Some(c - b'a' + 10),
        b'A'..=b'F' => Some(c - b'A' + 10),
        _ => None,
    }
}

pub fn register(env: &mut Environment) -> Result<(), String> {
    let mut encoding_obj = HashMap::new();

    encoding_obj.insert(
        "base64_encode".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            let input = expect_string_arg(&args, "encoding.base64_encode")?;
            Ok(Value::String(base64_encode_bytes(input.as_bytes())))
        })),
    );

    encoding_obj.insert(
        "base64_decode".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            let input = expect_string_arg(&args, "encoding.base64_decode")?;
            let bytes = base64_decode_string(input.as_str())?;
            let decoded = String::from_utf8(bytes).map_err(|_| "Decoded base64 is not valid UTF-8".to_string())?;
            Ok(Value::String(decoded))
        })),
    );

    encoding_obj.insert(
        "hex_encode".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            let input = expect_string_arg(&args, "encoding.hex_encode")?;
            let mut out = String::with_capacity(input.len() * 2);
            for b in input.as_bytes() {
                out.push(hex_digit((b >> 4) & 0x0F));
                out.push(hex_digit(b & 0x0F));
            }
            Ok(Value::String(out))
        })),
    );

    encoding_obj.insert(
        "hex_decode".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            let input = expect_string_arg(&args, "encoding.hex_decode")?;
            let bytes = input.as_bytes();
            if bytes.len() % 2 != 0 {
                return Err("Invalid hex length".to_string());
            }
            let mut out = Vec::with_capacity(bytes.len() / 2);
            let mut i = 0usize;
            while i < bytes.len() {
                let hi = parse_hex_nibble(bytes[i]).ok_or_else(|| "Invalid hex character".to_string())?;
                let lo = parse_hex_nibble(bytes[i + 1]).ok_or_else(|| "Invalid hex character".to_string())?;
                out.push((hi << 4) | lo);
                i += 2;
            }
            let decoded = String::from_utf8(out).map_err(|_| "Decoded hex is not valid UTF-8".to_string())?;
            Ok(Value::String(decoded))
        })),
    );

    encoding_obj.insert(
        "url_encode".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            let input = expect_string_arg(&args, "encoding.url_encode")?;
            let mut out = String::new();
            for b in input.as_bytes() {
                if is_unreserved_url_byte(*b) {
                    out.push(*b as char);
                } else {
                    out.push('%');
                    out.push(hex_digit((b >> 4) & 0x0F));
                    out.push(hex_digit(b & 0x0F));
                }
            }
            Ok(Value::String(out))
        })),
    );

    encoding_obj.insert(
        "url_decode".to_string(),
        Value::NativeFunction(Arc::new(|args| {
            let input = expect_string_arg(&args, "encoding.url_decode")?;
            let bytes = input.as_bytes();
            let mut out = Vec::with_capacity(bytes.len());
            let mut i = 0usize;
            while i < bytes.len() {
                if bytes[i] == b'%' {
                    if i + 2 >= bytes.len() {
                        return Err("Invalid percent-encoding sequence".to_string());
                    }
                    let hi = parse_hex_nibble(bytes[i + 1]).ok_or_else(|| "Invalid percent-encoding sequence".to_string())?;
                    let lo = parse_hex_nibble(bytes[i + 2]).ok_or_else(|| "Invalid percent-encoding sequence".to_string())?;
                    out.push((hi << 4) | lo);
                    i += 3;
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            let decoded = String::from_utf8(out).map_err(|_| "Decoded URL data is not valid UTF-8".to_string())?;
            Ok(Value::String(decoded))
        })),
    );

    env.declare("encoding".to_string(), Value::Object(encoding_obj), true);
    Ok(())
}
