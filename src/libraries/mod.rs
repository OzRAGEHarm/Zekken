#![allow(dead_code)]

pub mod math;
pub mod fs;

use std::collections::HashMap;
use std::sync::OnceLock;
use crate::environment::Environment;
use crate::errors::ZekkenError;

/// Type alias for library registration functions
type LibraryFunction = fn(&mut Environment) -> Result<(), String>;

/// Global registry of available libraries
static LIBRARIES: OnceLock<HashMap<&'static str, LibraryFunction>> = OnceLock::new();

/// Initialize the standard library registry
fn init_libraries() -> HashMap<&'static str, LibraryFunction> {
    let mut map: HashMap<&'static str, LibraryFunction> = HashMap::new();
    
    // Register standard libraries
    map.insert("math", math::register);
    map.insert("fs", fs::register);
    // Add other standard libraries here...
    
    map
}

/// Load and initialize a library by name
pub fn load_library(library: &str, env: &mut Environment) -> Result<(), ZekkenError> {
    let registry = LIBRARIES.get_or_init(init_libraries);
    if let Some(register_fn) = registry.get(library) {
        register_fn(env).map_err(|e| ZekkenError::internal(&format!("Failed to load library '{}': {}", library, e)))
    } else {
        Err(ZekkenError::internal(&format!("Library '{}' not found", library)))
    }
}