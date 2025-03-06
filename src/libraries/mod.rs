pub mod math;

use std::collections::HashMap;
use std::sync::OnceLock;
use crate::environment::Environment;
use crate::errors::ZekkenError;

type LibraryFunction = fn(&mut Environment) -> Result<(), String>;
static LIBRARIES: OnceLock<HashMap<&'static str, LibraryFunction>> = OnceLock::new();

pub fn load_library(library: &str, env: &mut Environment) -> Result<(), ZekkenError> {
    let libs = LIBRARIES.get_or_init(|| {
        let mut map: HashMap<&'static str, LibraryFunction> = HashMap::new();
        map.insert("math", math::register);
        map
    });

    if let Some(register_fn) = libs.get(library) {
        register_fn(env).map_err(|e| ZekkenError::RuntimeError {
            message: format!("Failed to load native library '{}': {}", library, e),
            filename: None,
            line: None,
            column: None,
            line_content: None,
            pointer: None,
            expected: None,
            found: None,
        })
    } else {
        Err(ZekkenError::RuntimeError {
            message: format!("Native library '{}' not found", library),
            filename: None,
            line: None,
            column: None,
            line_content: None,
            pointer: None,
            expected: None,
            found: None,
        })
    }
}