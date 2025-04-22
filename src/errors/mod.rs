#![allow(dead_code)]

use std::fmt;
use std::error::Error;
use std::env;

#[derive(Debug)]
pub enum ZekkenError {
    SyntaxError {
        message: String,
        filename: String,
        line: usize,
        column: usize,
        line_content: String,
        pointer: String,
        expected: String,
        found: String,
    },
    
    RuntimeError {
        message: String,
        error_type: RuntimeErrorType,
        filename: Option<String>,
        line: Option<usize>,
        column: Option<usize>,
        line_content: Option<String>,
        pointer: Option<String>,
        expected: Option<String>,
        found: Option<String>,
    },

    TypeError {
        message: String,
        expected_type: String,
        found_type: String,
        location: Location,
    },

    ReferenceError {
        message: String,
        name: String,
        location: Location,
    },

    InternalError(String),
}

#[derive(Debug)]
pub enum RuntimeErrorType {
    DivisionByZero,
    IndexOutOfBounds,
    NullReference,
    StackOverflow,
    InvalidArgument,
    UndefinedVariable,
    TypeError,
    ReferenceError,
    SyntaxError,
    Other,
}

#[derive(Debug, Clone)]
pub struct Location {
    pub filename: String,
    pub line: usize,
    pub column: usize,
    pub line_content: String,
}

impl fmt::Display for ZekkenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZekkenError::SyntaxError { 
                message, 
                filename, 
                line, 
                column, 
                line_content, 
                pointer, 
                expected, 
                found 
            } => {
                write!(
                    f,
                    "\n\x1b[1;31mSyntax Error\x1b[0m: {}\n\
                     \x1b[1;90m  ┌─\x1b[0m \x1b[1;37m{}\x1b[0m\n\
                     \x1b[1;90m  ├─[\x1b[0m Line \x1b[1;37m{}\x1b[0m, Column \x1b[1;37m{}\x1b[0m \x1b[1;90m]\x1b[0m\n\
                     \x1b[1;90m  │\x1b[0m\n\
                     \x1b[1;90m  │\x1b[0m {}\n\
                     \x1b[1;90m  │\x1b[0m {}\n\
                     \x1b[1;90m  │\x1b[0m\n\
                     \x1b[1;90m  │\x1b[0m Expected: \x1b[1;32m{}\x1b[0m\n\
                     \x1b[1;90m  │\x1b[0m Found:    \x1b[1;31m{}\x1b[0m\n",
                    message, filename, line, column, line_content, pointer, expected, found
                )
            },
            ZekkenError::RuntimeError { 
                message,
                error_type,
                filename,
                line,
                column,
                line_content,
                pointer,
                expected: _expected,
                found: _found,
            } => {
                if let (Some(fname), Some(l), Some(c), Some(lc), Some(ptr)) = 
                    (filename, line, column, line_content, pointer) {
                    write!(
                        f,
                        "\n\x1b[1;31mRuntime Error\x1b[0m: {} ({:?})\n\
                         \x1b[1;90m  ┌─\x1b[0m \x1b[1;37m{}\x1b[0m\n\
                         \x1b[1;90m  ├─[\x1b[0m Line \x1b[1;37m{}\x1b[0m, Column \x1b[1;37m{}\x1b[0m \x1b[1;90m]\x1b[0m\n\
                         \x1b[1;90m  │\x1b[0m\n\
                         \x1b[1;90m  │\x1b[0m {}\n\
                         \x1b[1;90m  │\x1b[0m {}\n",
                        message, error_type, fname, l, c, lc, ptr
                    )
                } else {
                    write!(f, "\n\x1b[1;31mRuntime Error\x1b[0m: {} ({:?})", message, error_type)
                }
            },
            ZekkenError::TypeError { 
                message,
                expected_type,
                found_type,
                location
            } => {
                write!(
                    f,
                    "\n\x1b[1;31mType Error\x1b[0m: {}\n\
                     \x1b[1;90m  ┌─\x1b[0m \x1b[1;37m{}\x1b[0m\n\
                     \x1b[1;90m  ├─[\x1b[0m Line \x1b[1;37m{}\x1b[0m, Column \x1b[1;37m{}\x1b[0m \x1b[1;90m]\x1b[0m\n\
                     \x1b[1;90m  │\x1b[0m\n\
                     \x1b[1;90m  │\x1b[0m {}\n\
                     \x1b[1;90m  │\x1b[0m Expected type: \x1b[1;32m{}\x1b[0m\n\
                     \x1b[1;90m  │\x1b[0m Found type:    \x1b[1;31m{}\x1b[0m\n",
                    message,
                    location.filename,
                    location.line,
                    location.column,
                    location.line_content,
                    expected_type,
                    found_type
                )
            },
            ZekkenError::ReferenceError { 
                message,
                name,
                location
            } => {
                write!(
                    f,
                    "\n\x1b[1;31mReference Error\x1b[0m: {}\n\
                     \x1b[1;90m  ┌─\x1b[0m \x1b[1;37m{}\x1b[0m\n\
                     \x1b[1;90m  ├─[\x1b[0m Line \x1b[1;37m{}\x1b[0m, Column \x1b[1;37m{}\x1b[0m \x1b[1;90m]\x1b[0m\n\
                     \x1b[1;90m  │\x1b[0m\n\
                     \x1b[1;90m  │\x1b[0m {}\n\
                     \x1b[1;90m  │\x1b[0m Variable: \x1b[1;31m{}\x1b[0m\n",
                    message,
                    location.filename,
                    location.line,
                    location.column,
                    location.line_content,
                    name
                )
            },
            ZekkenError::InternalError(msg) => {
                write!(f, "\n\x1b[1;31mInternal Compiler Error\x1b[0m: {}", msg)
            }
        }
    }
}

impl Error for ZekkenError {}

// Helper functions for creating errors
pub fn syntax_error(
    message: &str,
    filename: &str,
    line: usize,
    column: usize,
    line_content: &str,
    expected: &str,
    found: &str
) -> ZekkenError {
    let pointer = " ".repeat(column - 1) + "^";
    ZekkenError::SyntaxError {
        message: message.to_string(),
        filename: filename.to_string(),
        line,
        column,
        line_content: line_content.to_string(),
        pointer,
        expected: expected.to_string(),
        found: found.to_string(),
    }
}

pub fn runtime_error(
    message: &str,
    error_type: RuntimeErrorType,
    line: usize,
    column: usize
) -> ZekkenError {
    let filename = env::var("ZEKKEN_CURRENT_FILE")
        .unwrap_or_else(|_| "<unknown>".to_string());
    let source = env::var("ZEKKEN_SOURCE_LINES")
        .unwrap_or_else(|_| "<source unavailable>".to_string());
    let line_content = source.lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("<line not found>")
        .to_string();
    let pointer = " ".repeat(column.saturating_sub(1)) + "^";

    ZekkenError::RuntimeError {
        message: message.to_string(),
        error_type,
        filename: Some(filename),
        line: Some(line),
        column: Some(column),
        line_content: Some(line_content),
        pointer: Some(pointer),
        expected: None,
        found: None,
    }
}

pub fn type_error(
    message: &str,
    expected_type: &str,
    found_type: &str,
    location: Location
) -> ZekkenError {
    ZekkenError::TypeError {
        message: message.to_string(),
        expected_type: expected_type.to_string(),
        found_type: found_type.to_string(),
        location,
    }
}

pub fn reference_error(
    message: &str,
    name: &str,
    location: Location
) -> ZekkenError {
    ZekkenError::ReferenceError {
        message: message.to_string(),
        name: name.to_string(),
        location,
    }
}

pub fn internal_error(message: &str) -> ZekkenError {
    ZekkenError::InternalError(message.to_string())
}