use std::fmt;
use std::error::Error;
use std::env;
use std::collections::HashSet;
use std::sync::Mutex;

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
        // WASM: avoid std::env and std::fs
        #[cfg(target_arch = "wasm32")]
        {
            Self::new("<web>".to_string(), line, column, "".to_string())
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let filename = env::var("ZEKKEN_CURRENT_FILE").unwrap_or_else(|_| "<unknown>".to_string());
            let line_content = if filename != "<unknown>" {
                std::fs::read_to_string(&filename)
                    .ok()
                    .and_then(|src| src.lines().nth(line.saturating_sub(1)).map(|l| l.trim_end().to_string()))
                    .unwrap_or("<line not found>".to_string())
            } else {
                "<line not found>".to_string()
            };
            let highlighted = highlight_zekken_line(&line_content);
            Self::new(filename, line, column, highlighted)
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
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
        // If expected is a type token, try to pretty it up
        let pretty_expected = if let Some(e) = expected {
            // If the string already looks like a full type list or hint, don't pretty-print
            if e.contains("a type (int, float, string, bool, obj, arr, fn)") {
                e
            } else {
                match e {
                    "DataType(Any)" => "a type (int, float, string, bool, obj, arr, fn)",
                    "DataType(Int)" => "Int Type (int)",
                    "DataType(Float)" => "Float Type (float)",
                    "DataType(String)" => "String Type (string)",
                    "DataType(Bool)" => "Bool Type (bool)",
                    "DataType(Object)" => "Object Type (obj)",
                    "DataType(Array)" => "Array Type (arr)",
                    "DataType(Fn)" => "Function Type (fn)",
                    _ => e,
                }
            }
        } else {
            ""
        };

        
        // Use colorize function for expected/found
        if let Some(_e) = expected {
            extra.push_str(&format!(
                "{} {}{}\n",
                colorize("  expected:", "\x1b[1;90m"),
                colorize(pretty_expected, "\x1b[1;32m"),
                colorize("", "\x1b[0m")
            ));
        }
        if let Some(f) = found {
            extra.push_str(&format!(
                "{} {}{}\n",
                colorize("  found:   ", "\x1b[1;90m"), 
                colorize(f, "\x1b[1;31m"), 
                colorize("", "\x1b[0m")
            ));
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
            "{} {}{}\n{} {}{}\n",
            colorize("  expected:", "\x1b[1;90m"),
            colorize(expected, "\x1b[1;32m"),
            colorize("", "\x1b[0m"),        
            colorize("  found:   ", "\x1b[1;90m"),
            colorize(found, "\x1b[1;31m"),   
            colorize("", "\x1b[0m")    
        );
        

        Self {
            kind: ErrorKind::Type,
            message: msg.to_string(),
            context: ctx,
            extra: Some(extra),
        }
    }
    pub fn reference(msg: &str, kind: &str, line: usize, column: usize) -> Self {
        let ctx = ErrorContext::from_env(line, column);
        
        let extra = format!(
            "{} {}{}\n",
            colorize("  kind:", "\x1b[1;90m"),
            colorize(kind, "\x1b[1;31m"),
            colorize("", "\x1b[0m")
        );
        
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

    /// Render a REPL-friendly error string (single-line, no file/line context)
    pub fn to_repl_string(&self) -> String {
        let kind = match self.kind {
            ErrorKind::Syntax => "Syntax Error",
            ErrorKind::Runtime => "Runtime Error",
            ErrorKind::Type => "Type Error",
            ErrorKind::Reference => "Reference Error",
            ErrorKind::Internal => "Internal Error",
        };
        let mut msg = format!("{}: {}", kind, self.message);
        if let Some(extra) = &self.extra {
            // Remove ANSI color codes for REPL and trim lines
            let plain = regex::Regex::new(r"\x1b\[[0-9;]*m").unwrap().replace_all(extra, "");
            for line in plain.lines() {
                msg.push_str(&format!("\n  {}", line.trim()));
            }
        }
        msg
    }
}

lazy_static::lazy_static! {
    // Store errors as (kind, line, column, message) to deduplicate
    static ref ERROR_SET: Mutex<HashSet<(String, usize, usize, String)>> = Mutex::new(HashSet::new());
    pub static ref ERROR_LIST: Mutex<Vec<ZekkenError>> = Mutex::new(Vec::new());
    static ref NO_COLOR: Mutex<bool> = Mutex::new(
        std::env::var("NO_COLOR").is_ok() ||
        std::env::var("TERM").map(|term| term == "dumb").unwrap_or(false)
    );
    pub static ref REPL_MODE: Mutex<bool> = Mutex::new(false);
}

// Helper function to conditionally apply color
fn colorize(text: &str, color_code: &str) -> String {
    if *NO_COLOR.lock().unwrap() {
        text.to_string()
    } else {
        format!("{}{}\x1b[0m", color_code, text)
    }
}

impl fmt::Display for ZekkenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if *REPL_MODE.lock().unwrap() {
            write!(f, "{}", self.to_repl_string())
        } else {
            let (kind, color) = match self.kind {
                ErrorKind::Syntax => ("Syntax Error", "\x1b[1;31m"),
                ErrorKind::Runtime => ("Runtime Error", "\x1b[1;35m"),
                ErrorKind::Type => ("Type Error", "\x1b[1;33m"),
                ErrorKind::Reference => ("Reference Error", "\x1b[1;34m"),
                ErrorKind::Internal => ("Internal Error", "\x1b[1;41m"),
            };

            let kind_str = colorize(kind, color);
            let location = format!("{} -> [Ln: {}, Col: {}]", 
                self.context.filename, self.context.line, self.context.column);
            let line_num = format!("{:>4}", self.context.line);
            
            write!(
                f,
                "\n{}: {}\n     | {}\n     |\n{} | {}\n     | {}\n{}",
                kind_str,
                self.message,
                colorize(&location, "\x1b[1;37m"),
                colorize(&line_num, "\x1b[1;90m"),
                self.context.line_content,
                colorize(&self.context.pointer, "\x1b[1;32m"),
                self.extra.clone().unwrap_or_default()
            )
        }
    }
}

impl Error for ZekkenError {}

// Add a global error collector using a Mutex-protected Vec

pub fn push_error(error: ZekkenError) {
    let key = (
        format!("{:?}", error.kind),
        error.context.line,
        error.context.column,
        error.message.clone(),
    );
    let mut set = ERROR_SET.lock().unwrap();
    if set.insert(key) {
        ERROR_LIST.lock().unwrap().push(error);
    }
}

// Print and clear all collected errors, returns true if any errors were printed
#[allow(dead_code)]
pub fn print_and_clear_errors() -> bool {
    let mut errors = ERROR_LIST.lock().unwrap();
    if !errors.is_empty() {
        for error in errors.iter() {
            eprintln!("{}", error);
        }
        errors.clear();
        ERROR_SET.lock().unwrap().clear();
        true
    } else {
        false
    }
}

fn highlight_zekken_line(line: &str) -> String {
    if *NO_COLOR.lock().unwrap() {
        return line.to_string();
    }
    use regex::Regex;
    struct Span {
        start: usize,
        end: usize,
        color: &'static str,
        bold: bool,
        italic: bool,
    }

    // Color mapping (ANSI 24-bit for best match)
    const COMMENT: &str = "\x1b[38;2;106;153;85m";
    const BUILTIN_FN: &str = "\x1b[38;2;86;156;214m";
    const FN_DECL: &str = "\x1b[38;2;220;220;170m";
    const FN_CALL: &str = "\x1b[38;2;220;220;170m";
    const KEYWORD_OTHER: &str = "\x1b[38;2;86;156;214m";
    const KEYWORD_CONTROL: &str = "\x1b[38;2;198;120;221m";
    const OPERATOR: &str = "\x1b[38;2;212;212;212m";
    const VARIABLE: &str = "\x1b[38;2;156;220;254m";
    const TYPE: &str = "\x1b[38;2;78;201;176m";
    const INT: &str = "\x1b[38;2;181;206;168m";
    const FLOAT: &str = "\x1b[38;2;181;206;168m";
    const BOOL: &str = "\x1b[38;2;86;156;214m";
    const ESCAPE: &str = "\x1b[38;2;215;186;125m\x1b[1m";
    const STRING: &str = "\x1b[38;2;206;145;120m";

    let patterns: &[(&str, &str, bool, bool)] = &[
        // Comments (line and block)
        (r"//.*", COMMENT, false, true),
        (r"/\*.*?\*/", COMMENT, false, true),
        // Strings (double and single)
        (r#""([^"\\]|\\.)*""#, STRING, false, false),
        (r#"'([^'\\]|\\.)*'"#, STRING, false, false),
        // String escapes (inside strings)
        (r#"\\[abfnrtv0'"\\]"#, ESCAPE, true, false),
        // Keywords (control)
        (r"\b(if|else|for|while|try|catch|return)\b", KEYWORD_CONTROL, false, false),
        // Keywords (other)
        (r"\b(use|include|export|func|let|const|from|in)\b", KEYWORD_OTHER, false, false),
        // Types
        (r"\b(int|float|bool|string|arr|obj|fn)\b", TYPE, false, false),
        // Boolean
        (r"\b(true|false)\b", BOOL, false, false),
        // Numbers (float and int)
        (r"\b\d+\.\d+\b", FLOAT, false, false),
        (r"\b\d+\b", INT, false, false),
        // Function declaration (func name)
        (r"\bfunc\s+([a-zA-Z_][a-zA-Z0-9_]*)", FN_DECL, false, false),
        // Operators
        (r"=>|->|[+\-*/%=]", OPERATOR, false, false),
        // Variables (fallback)
        (r"\b[a-zA-Z_][a-zA-Z0-9_]*\b", VARIABLE, false, false),
    ];

    let mut spans: Vec<Span> = Vec::new();

    // Highlight @ symbol and builtin function name separately
    {
        let re = Regex::new(r"@([a-zA-Z_][a-zA-Z0-9_]*)").unwrap();
        for m in re.captures_iter(line) {
            if let Some(mat) = m.get(0) {
                let at_pos = mat.start();
                let name_pos = at_pos + 1;
                let name_end = mat.end();
                spans.push(Span {
                    start: name_pos,
                    end: name_end,
                    color: BUILTIN_FN,
                    bold: false,
                    italic: false,
                });
            }
        }
    }

    for (pat, color, bold, italic) in patterns {
        let re = Regex::new(pat).unwrap();
        for m in re.find_iter(line) {
            spans.push(Span { start: m.start(), end: m.end(), color, bold: *bold, italic: *italic });
        }
    }

    // Highlight function calls: identifier immediately before '=>'
    // Only highlight as FN_CALL if not a builtin (not preceded by @)
    {
        let mut idx = 0;
        while let Some(pos) = line[idx..].find("=>") {
            let abs_pos = idx + pos;
            let before = &line[..abs_pos];
            // Find the identifier before =>
            if let Some(id_match) = Regex::new(r"[a-zA-Z_][a-zA-Z0-9_]*\s*$").unwrap().find(before) {
                // Check if this identifier is preceded by '@'
                let id_start = id_match.start();
                let is_builtin = id_start > 0 && &before[id_start - 1..id_start] == "@";
                if !is_builtin {
                    spans.push(Span {
                        start: id_match.start(),
                        end: id_match.end(),
                        color: FN_CALL,
                        bold: false,
                        italic: false,
                    });
                }
            }
            idx = abs_pos + 2;
        }
    }

    // Sort by start, then by longest match (descending)
    spans.sort_by(|a, b| a.start.cmp(&b.start).then(b.end.cmp(&a.end)));

    // Remove overlapping spans (keep outermost)
    let mut filtered: Vec<Span> = Vec::new();
    let mut last_end = 0;
    for s in spans {
        if s.start >= last_end {
            last_end = s.end;
            filtered.push(s);
        }
    }

    // Build highlighted line
    let mut result = String::new();
    let mut idx = 0;
    for s in filtered {
        if idx < s.start {
            result.push_str(&line[idx..s.start]);
        }
        result.push_str(s.color);
        if s.bold {
            result.push_str("\x1b[1m");
        }
        if s.italic {
            result.push_str("\x1b[3m");
        }
        result.push_str(&line[s.start..s.end]);
        result.push_str("\x1b[0m");
        idx = s.end;
    }
    if idx < line.len() {
        result.push_str(&line[idx..]);
    }
    result
}