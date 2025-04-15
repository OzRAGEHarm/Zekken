#![allow(dead_code)]

pub mod math;

use std::collections::HashMap;
use std::sync::OnceLock;
use crate::environment::Environment;
use crate::errors::{ZekkenError, RuntimeErrorType, runtime_error};
use std::env;

/// Type alias for library registration functions
type LibraryFunction = fn(&mut Environment) -> Result<(), String>;

/// Global registry of available libraries
static LIBRARIES: OnceLock<HashMap<&'static str, LibraryFunction>> = OnceLock::new();

/// Initialize the standard library registry
fn init_libraries() -> HashMap<&'static str, LibraryFunction> {
    let mut map: HashMap<&'static str, LibraryFunction> = HashMap::new();
    
    // Register standard libraries
    map.insert("math", math::register);
    // Add other standard libraries here...
    
    map
}

/// Load and initialize a library by name
pub fn load_library(library: &str, env: &mut Environment) -> Result<(), ZekkenError> {
    // Get current source location for error reporting
    let _filename = env::var("ZEKKEN_CURRENT_FILE").unwrap_or_default();
    let line: usize = env::var("ZEKKEN_CURRENT_LINE").unwrap_or_default().parse().unwrap_or(0);
    let column = env::var("ZEKKEN_CURRENT_COLUMN").unwrap_or_default().parse().unwrap_or(0);
    let _line_content = env::var("ZEKKEN_SOURCE_LINES")
        .unwrap_or_default()
        .lines()
        .nth(line.saturating_sub(1))
        .unwrap_or("")
        .to_string();

    // Get or initialize library registry
    let libs = LIBRARIES.get_or_init(init_libraries);

    // Attempt to load the library
    if let Some(register_fn) = libs.get(library) {
        register_fn(env).map_err(|e| runtime_error(
            &format!("Failed to load library '{}': {}", library, e),
            RuntimeErrorType::ReferenceError,
            line,
            column
        ))
    } else {
        Err(runtime_error(
            &format!("Library '{}' not found", library),
            RuntimeErrorType::ReferenceError,
            line,
            column
        ))
    }
}