use std::fmt;
use std::error::Error;
use std::fs;

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
        filename: Option<String>,
        line: Option<usize>,
        column: Option<usize>,
        line_content: Option<String>,
        pointer: Option<String>,
        expected: Option<String>,
        found: Option<String>,
    },
    InternalError(String),
}

impl fmt::Display for ZekkenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ZekkenError::SyntaxError { message, filename, line, column, line_content, pointer, expected, found } => write!(
                f,
                "\x1b[1;31mSyntax Error\x1b[0m: {}\n\
                 \x1b[1;90m  ┌─\x1b[0m \x1b[1;37m{}\x1b[0m\n\
                 \x1b[1;90m  ├─[\x1b[0m Line \x1b[1;37m{}\x1b[0m, Column \x1b[1;37m{}\x1b[0m \x1b[1;90m]\x1b[0m\n\
                 \x1b[1;90m  │\x1b[0m\n\
                 \x1b[1;90m{:>4} │\x1b[0m {}\n\
                 \x1b[1;90m     │\x1b[0m {}\n\
                 \x1b[1;90m     │\x1b[0m\n\
                 \x1b[1;90m     │\x1b[0m Expected: \x1b[1;32m{}\x1b[0m\n\
                 \x1b[1;90m     │\x1b[0m Found:    \x1b[1;31m{}\x1b[0m\n",
                message, filename, line, column, line, line_content, pointer, expected, found
            ),
            ZekkenError::RuntimeError { message, filename, line, column, line_content, pointer, expected, found } => {
                if let (Some(fname), Some(l), Some(c), Some(lc), Some(ptr), Some(exp), Some(fnd)) =
                    (filename, line, column, line_content, pointer, expected, found)
                {
                    write!(
                        f,
                        "\x1b[1;31mRuntime Error\x1b[0m: {}\n\
                         \x1b[1;90m  ┌─\x1b[0m \x1b[1;37m{}\x1b[0m\n\
                         \x1b[1;90m  ├─[\x1b[0m Line \x1b[1;37m{}\x1b[0m, Column \x1b[1;37m{}\x1b[0m \x1b[1;90m]\x1b[0m\n\
                         \x1b[1;90m  │\x1b[0m\n\
                         \x1b[1;90m{:>4} │\x1b[0m {}\n\
                         \x1b[1;90m     │\x1b[0m {}\n\
                         \x1b[1;90m     │\x1b[0m\n\
                         \x1b[1;90m     │\x1b[0m Expected: \x1b[1;32m{}\x1b[0m\n\
                         \x1b[1;90m     │\x1b[0m Found:    \x1b[1;31m{}\x1b[0m\n",
                        message, fname, l, c, l, lc, ptr, exp, fnd
                    )
                } else {
                    write!(f, "\x1b[1;31mRuntime Error\x1b[0m: {}", message)
                }
            }
            ZekkenError::InternalError(msg) => write!(f, "\x1b[1;31mInternal Error\x1b[0m: {}", msg),
        }
    }
}

impl Error for ZekkenError {}

pub fn runtime_error(message: &str, line: usize, column: usize) -> ZekkenError {
    let filename = std::env::var("ZEKKEN_CURRENT_FILE").unwrap_or_else(|_| "<unknown>".into());
    let file_content = fs::read_to_string(&filename).unwrap_or_else(|_| "<unknown>".into());
    // Get the specific line from the file; line numbers are 1-indexed.
    let line_content = file_content.lines().nth(line.wrapping_sub(1)).unwrap_or("<unknown>").to_string();
    let pointer = " ".repeat(column.saturating_sub(1)) + "\x1b[1;31m^\x1b[0m";

    ZekkenError::RuntimeError {
        message: message.to_string(),
        filename: Some(filename),
        line: Some(line),
        column: Some(column),
        line_content: Some(line_content),
        pointer: Some(pointer),
        expected: None,
        found: None,
    }
}

/// A helper function to output errors.
pub fn handle_error(err: &ZekkenError) {
    eprintln!("{}", err);
}