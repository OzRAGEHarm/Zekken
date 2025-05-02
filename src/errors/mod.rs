use std::fmt;
use std::error::Error;
use std::env;

#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub filename: String,
    pub line: usize,
    pub column: usize,
    pub line_content: String,
    pub pointer: String,
}

impl ErrorContext {
    pub fn new(filename: String, line: usize, column: usize, line_content: String) -> Self {
        let pointer = " ".repeat(column.saturating_sub(1)) + "^";
        Self { filename, line, column, line_content, pointer }
    }
    pub fn from_env(line: usize, column: usize) -> Self {
        let filename = env::var("ZEKKEN_CURRENT_FILE").unwrap_or_else(|_| "<unknown>".to_string());
        let source = env::var("ZEKKEN_SOURCE_LINES").unwrap_or_else(|_| "<source unavailable>".to_string());
        let line_content = source.lines().nth(line.saturating_sub(1)).unwrap_or("<line not found>").to_string();
        Self::new(filename, line, column, line_content)
    }
}

#[derive(Debug, Clone)]
pub enum ErrorKind {
    Syntax,
    Runtime,
    Type,
    Reference,
    Internal,
}

#[derive(Debug, Clone)]
pub struct ZekkenError {
    pub kind: ErrorKind,
    pub message: String,
    pub context: ErrorContext,
    pub extra: Option<String>, // For expected/found, etc.
}

impl ZekkenError {
    pub fn syntax(msg: &str, line: usize, column: usize, expected: Option<&str>, found: Option<&str>) -> Self {
        let ctx = ErrorContext::from_env(line, column);
        let mut extra = String::new();
        if let Some(e) = expected {
            extra.push_str(&format!("\x1b[1;90m  expected: \x1b[1;32m{}\x1b[0m\n", e));
        }
        if let Some(f) = found {
            extra.push_str(&format!("\x1b[1;90m  found:    \x1b[1;31m{}\x1b[0m\n", f));
        }
        Self {
            kind: ErrorKind::Syntax,
            message: msg.to_string(),
            context: ctx,
            extra: if extra.is_empty() { None } else { Some(extra) },
        }
    }
    pub fn runtime(msg: &str, line: usize, column: usize, details: Option<&str>) -> Self {
        let ctx = ErrorContext::from_env(line, column);
        Self {
            kind: ErrorKind::Runtime,
            message: msg.to_string(),
            context: ctx,
            extra: details.map(|d| d.to_string()),
        }
    }
    pub fn type_error(msg: &str, expected: &str, found: &str, line: usize, column: usize) -> Self {
        let ctx = ErrorContext::from_env(line, column);
        let extra = format!(
            "\x1b[1;90m  expected: \x1b[1;32m{}\x1b[0m\n\x1b[1;90m  found:    \x1b[1;31m{}\x1b[0m\n",
            expected, found
        );
        Self {
            kind: ErrorKind::Type,
            message: msg.to_string(),
            context: ctx,
            extra: Some(extra),
        }
    }
    pub fn reference(msg: &str, name: &str, line: usize, column: usize) -> Self {
        let ctx = ErrorContext::from_env(line, column);
        let extra = format!("\x1b[1;90m  variable: \x1b[1;31m{}\x1b[0m\n", name);
        Self {
            kind: ErrorKind::Reference,
            message: msg.to_string(),
            context: ctx,
            extra: Some(extra),
        }
    }
    pub fn internal(msg: &str) -> Self {
        Self {
            kind: ErrorKind::Internal,
            message: msg.to_string(),
            context: ErrorContext::new("<internal>".to_string(), 0, 0, "".to_string()),
            extra: None,
        }
    }
}

impl fmt::Display for ZekkenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (kind, color) = match self.kind {
            ErrorKind::Syntax => ("Syntax Error", "\x1b[1;31m"),
            ErrorKind::Runtime => ("Runtime Error", "\x1b[1;35m"),
            ErrorKind::Type => ("Type Error", "\x1b[1;33m"),
            ErrorKind::Reference => ("Reference Error", "\x1b[1;34m"),
            ErrorKind::Internal => ("Internal Error", "\x1b[1;41m"),
        };
        write!(
            f,
            "\n{color}{kind}\x1b[0m: {}\n\x1b[1;90m     |\x1b[0m \x1b[1;37m{} -> [Ln: {}, Col: {}]\x1b[0m\n\x1b[1;90m     |\x1b[0m\n\x1b[1;90m{:>4} |\x1b[0m {}\n\x1b[1;90m     |\x1b[0m \x1b[1;32m{}\x1b[0m\n{}",
            self.message,
            self.context.filename, self.context.line, self.context.column,
            self.context.line,
            self.context.line_content,
            self.context.pointer,
            self.extra.clone().unwrap_or_default()
        )
    }
}

impl Error for ZekkenError {}