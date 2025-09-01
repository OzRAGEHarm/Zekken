use crate::ast::*;
use crate::environment::{Environment, Value};
use crate::errors::ZekkenError;
use crate::libraries::load_library;
use std::path::Path;

pub fn lint_expression(expr: &Expr, env: &Environment) -> Result<(), ZekkenError> {
    match expr {
        Expr::Identifier(ident) => {
            // Look up the identifier in the environment
            if env.lookup(&ident.name).is_none() {
                return Err(ZekkenError::reference(
                    &format!("Variable '{}' not found", &ident.name),
                    "variable",
                    ident.location.line,
                    ident.location.column,
                ));
            }
        },
        Expr::Call(call) => {
            match *call.callee {
                Expr::Identifier(ref ident) => {
                    match env.lookup(&ident.name) {
                        Some(Value::NativeFunction(_)) => {
                            // Special validation for @input
                            if ident.name == "@input" && call.args.len() != 1 {
                                return Err(ZekkenError::type_error(
                                    "@input expects exactly one argument (prompt string)",
                                    "one argument",
                                    &format!("{} arguments", call.args.len()),
                                    call.location.line,
                                    call.location.column,
                                ));
                            }
                            // Skip evaluating arguments for native functions during linting
                            return Ok(());
                        },
                        Some(Value::Function(_)) => {
                            // Regular function found, continue with argument linting
                        },
                        Some(_) => {
                            return Err(ZekkenError::type_error(
                                "Cannot call non-function value",
                                "function",
                                "non-function",
                                call.location.line,
                                call.location.column,
                            ));
                        },
                        None => {
                            // Check for native function with '@' prefix as fallback
                            let native_name = format!("@{}", &ident.name);
                            if let Some(Value::NativeFunction(_)) = env.lookup(&native_name) {
                                return Ok(());
                            }
                            return Err(ZekkenError::reference(
                                &format!("Function '{}' not found", &ident.name),
                                "function",
                                call.location.line,
                                call.location.column,
                            ));
                        }
                    }
                },
                _ => {
                    lint_expression(&call.callee, env)?;
                }
            }
            // Check arguments for non-native function calls
            if let Expr::Identifier(ref ident) = *call.callee {
                if !ident.name.starts_with('@') {
                    for arg in &call.args {
                        lint_expression(arg, env)?;
                    }
                }
            } else {
                for arg in &call.args {
                    lint_expression(arg, env)?;
                }
            }
        },
        Expr::Assign(assign) => {
            // Check if target is assignable
            match *assign.left {
                Expr::Identifier(ref ident) => {
                    let (_, kind) = env.lookup_with_kind(&ident.name);
                    if kind == Some("constant") {
                        return Err(ZekkenError::runtime(
                            &format!("Cannot assign to constant '{}'", &ident.name),
                            assign.location.line,
                            assign.location.column,
                            None,
                        ));
                    }
                },
                _ => {
                    return Err(ZekkenError::type_error(
                        "Invalid assignment target",
                        "identifier",
                        "other",
                        assign.location.line,
                        assign.location.column,
                    ));
                }
            }
            lint_expression(&assign.right, env)?;
        },
        Expr::Member(member) => {
            lint_expression(&member.object, env)?;
        },
        // Other expression types like literals don't need linting
        _ => {}
    }
    Ok(())
}

pub fn lint_statement(stmt: &Stmt, env: &Environment) -> Result<(), ZekkenError> {
    match stmt {
        Stmt::VarDecl(var_decl) => {
            if let Some(content) = &var_decl.value {
                match content {
                    Content::Expression(expr) => lint_expression(expr, env)?,
                    Content::Statement(stmt) => lint_statement(stmt, env)?,
                }
            }
        },
        Stmt::FuncDecl(func_decl) => {
            // Check function body for errors
            for content in &func_decl.body {
                match &**content {
                    Content::Expression(expr) => lint_expression(expr, env)?,
                    Content::Statement(stmt) => lint_statement(stmt, env)?,
                }
            }
        },
        Stmt::IfStmt(if_stmt) => {
            lint_expression(&if_stmt.test, env)?;
            for content in &if_stmt.body {
                match &**content {
                    Content::Expression(expr) => lint_expression(expr, env)?,
                    Content::Statement(stmt) => lint_statement(stmt, env)?,
                }
            }
            if let Some(alt) = &if_stmt.alt {
                for content in alt {
                    match &**content {
                        Content::Expression(expr) => lint_expression(expr, env)?,
                        Content::Statement(stmt) => lint_statement(stmt, env)?,
                    }
                }
            }
        },
        Stmt::ForStmt(for_stmt) => {
            if let Some(init) = &for_stmt.init {
                lint_statement(init, env)?;
            }
            for content in &for_stmt.body {
                match &**content {
                    Content::Expression(expr) => lint_expression(expr, env)?,
                    Content::Statement(stmt) => lint_statement(stmt, env)?,
                }
            }
        },
        Stmt::WhileStmt(while_stmt) => {
            lint_expression(&while_stmt.test, env)?;
            for content in &while_stmt.body {
                match &**content {
                    Content::Expression(expr) => lint_expression(expr, env)?,
                    Content::Statement(stmt) => lint_statement(stmt, env)?,
                }
            }
        },
        Stmt::TryCatchStmt(try_catch) => {
            for content in &try_catch.try_block {
                match &**content {
                    Content::Expression(expr) => lint_expression(expr, env)?,
                    Content::Statement(stmt) => lint_statement(stmt, env)?,
                }
            }
            if let Some(catch_block) = &try_catch.catch_block {
                for content in catch_block {
                    match &**content {
                        Content::Expression(expr) => lint_expression(expr, env)?,
                        Content::Statement(stmt) => lint_statement(stmt, env)?,
                    }
                }
            }
        },
        _ => {}
    }
    Ok(())
}

pub fn lint_include(include: &IncludeStmt) -> Result<(), ZekkenError> {
    // Get the directory of the current file being processed
    let current_file = std::env::var("ZEKKEN_CURRENT_FILE").unwrap_or_else(|_| "<unknown>".to_string());
    let current_dir = Path::new(&current_file)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    // Resolve path relative to current file's directory
    let mut full_path = std::path::PathBuf::from(&current_dir);
    full_path.push(&include.file_path);

    if !full_path.exists() {
        return Err(ZekkenError::runtime(
            &format!("File '{}' not found", include.file_path),
            include.location.line,
            include.location.column,
            None,
        ));
    }
    Ok(())
}

pub fn lint_use(use_stmt: &UseStmt) -> Result<(), ZekkenError> {
    // First check if library exists
    match use_stmt.module.as_str() {
        "math" | "fs" | "os" => {
            // If specific methods are requested, validate they exist in the library
            if let Some(methods) = &use_stmt.methods {
                // Create a temporary environment to load the library
                let mut temp_env = Environment::new();
                load_library(&use_stmt.module, &mut temp_env)?;
                
                // Look up the library object and check each method exists in it
                if let Some(Value::Object(lib_obj)) = temp_env.lookup(&use_stmt.module) {
                    for method in methods {
                        if !lib_obj.contains_key(method) {
                            return Err(ZekkenError::reference(
                                &format!("Method '{}' not found in library '{}'", method, use_stmt.module),
                                "function",
                                use_stmt.location.line,
                                use_stmt.location.column,
                            ));
                        }
                    }
                }
            }
            Ok(())
        },
        _ => Err(ZekkenError::runtime(
            &format!("Library '{}' not found", use_stmt.module),
            use_stmt.location.line,
            use_stmt.location.column,
            None,
        )),
    }
}
