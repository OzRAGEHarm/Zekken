use crate::ast::*;
use crate::environment::{Environment, FunctionValue, Value};
use crate::errors::ZekkenError;
use crate::lexer::DataType;
use crate::libraries::load_library;
use hashbrown::HashMap;
use std::path::Path;
use std::sync::Arc;

#[inline]
fn builtin_requires_at(name: &str) -> bool {
    matches!(name, "println" | "input" | "parse_json" | "queue")
}

fn dummy_value_for_type(ty: &DataType) -> Value {
    match ty {
        DataType::Int => Value::Int(0),
        DataType::Float => Value::Float(0.0),
        DataType::String => Value::String(String::new()),
        DataType::Bool => Value::Boolean(false),
        DataType::Object => Value::Object(HashMap::new()),
        DataType::Array => Value::Array(Vec::new()),
        DataType::Fn => Value::Function(FunctionValue {
            params: Arc::new(Vec::new()),
            body: Arc::new(Vec::new()),
            return_type: None,
            needs_parent: false,
            captures: Arc::new(Vec::new()),
        }),
        DataType::Any => Value::Void,
    }
}

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
        Expr::Binary(binary) => {
            lint_expression(&binary.left, env)?;
            lint_expression(&binary.right, env)?;
        }
        Expr::Call(call) => {
            // Lint callee.
            if let Expr::Identifier(ident) = call.callee.as_ref() {
                let (val, kind) = env.lookup_with_kind(&ident.name);
                let is_callable = matches!(
                    val.as_ref(),
                    Some(Value::Function(_)) | Some(Value::NativeFunction(_))
                );

                if !is_callable {
                    if val.is_some() {
                        return Err(ZekkenError::type_error(
                            "Cannot call non-function value",
                            "function",
                            "non-function",
                            call.location.line,
                            call.location.column,
                        ));
                    } else {
                        return Err(ZekkenError::reference(
                            &format!("Function '{}' not found", &ident.name),
                            "function",
                            call.location.line,
                            call.location.column,
                        ));
                    }
                }

                // Enforce built-ins requiring '@' prefix.
                if builtin_requires_at(&ident.name) && !call.is_native {
                    return Err(ZekkenError::runtime(
                        &format!("{} is a built-in; call it with '@{} => |...|'", ident.name, ident.name),
                        call.location.line,
                        call.location.column,
                        None,
                    ));
                }
                let _ = kind; // reserved for future, more precise diagnostics
            } else {
                // Calling a member (e.g. `"x".cast => |"int"|`, `math.sqrt => |9|`):
                // lint the object, but treat the member name as a literal (not a variable).
                if let Expr::Member(member) = call.callee.as_ref() {
                    lint_expression(&member.object, env)?;
                } else {
                    lint_expression(&call.callee, env)?;
                }
            }

            // Always lint arguments.
            for arg in &call.args {
                lint_expression(arg, env)?;
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
                Expr::Member(ref member) => {
                    lint_expression(&member.object, env)?;
                }
                _ => {
                    return Err(ZekkenError::type_error(
                        "Invalid assignment target",
                        "identifier or member access",
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
            // Only bracket/computed member access should lint the property expression.
            // Dot member access (`obj.key`) treats the identifier as a literal key.
            if member.is_method {
                lint_expression(&member.property, env)?;
            }
        },
        Expr::ArrayLit(array) => {
            for el in &array.elements {
                lint_expression(el, env)?;
            }
        }
        Expr::ObjectLit(object) => {
            for prop in &object.properties {
                lint_expression(&prop.value, env)?;
            }
        }
        // Other expression types like literals don't need linting
        _ => {}
    }
    Ok(())
}

pub fn lint_statement(stmt: &Stmt, env: &Environment) -> Result<(), ZekkenError> {
    fn lint_contents_seq(contents: &[Box<Content>], env: &mut Environment) -> Result<(), ZekkenError> {
        for content in contents {
            match content.as_ref() {
                Content::Expression(expr) => lint_expression(expr, env)?,
                Content::Statement(stmt) => {
                    // Inside a function/lambda body, model sequential local bindings so
                    // later expressions can reference earlier `let` declarations.
                    if let Stmt::VarDecl(var_decl) = stmt.as_ref() {
                        if let Some(content) = &var_decl.value {
                            match content {
                                Content::Expression(expr) => lint_expression(expr, env)?,
                                Content::Statement(stmt) => lint_statement(stmt, env)?,
                            }
                        }
                        env.declare_ref_typed(
                            var_decl.ident.as_str(),
                            dummy_value_for_type(&var_decl.type_),
                            var_decl.type_,
                            var_decl.constant,
                        );
                    } else {
                        lint_statement(stmt, env)?;
                    }
                }
            }
        }
        Ok(())
    }

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
            // Lint function body in a dedicated scope that includes parameters.
            let mut fn_env = Environment::new_with_parent_capacity(env.clone(), func_decl.params.len() + 8);
            for param in func_decl.params.iter() {
                fn_env.declare_ref_typed(param.ident.as_str(), dummy_value_for_type(&param.type_), param.type_, false);
            }

            lint_contents_seq(&func_decl.body, &mut fn_env)?;
        },
        Stmt::Lambda(lambda) => {
            // Same parameter scoping rules as functions.
            let mut fn_env = Environment::new_with_parent_capacity(env.clone(), lambda.params.len() + 8);
            for param in lambda.params.iter() {
                fn_env.declare_ref_typed(param.ident.as_str(), dummy_value_for_type(&param.type_), param.type_, false);
            }
            lint_contents_seq(&lambda.body, &mut fn_env)?;
        }
        Stmt::IfStmt(if_stmt) => {
            lint_expression(&if_stmt.test, env)?;
            let mut body_env = Environment::new_with_parent_capacity(env.clone(), 8);
            lint_contents_seq(&if_stmt.body, &mut body_env)?;
            if let Some(alt) = &if_stmt.alt {
                let mut alt_env = Environment::new_with_parent_capacity(env.clone(), 8);
                lint_contents_seq(alt, &mut alt_env)?;
            }
        },
        Stmt::ForStmt(for_stmt) => {
            if let Some(init) = &for_stmt.init {
                // `for |...| in <collection> { ... }` is represented as a `ForStmt` whose `init`
                // is a VarDecl with `ident` containing one/two loop identifiers and `value`
                // containing the collection expression.
                if for_stmt.test.is_none() && for_stmt.update.is_none() {
                    if let Stmt::VarDecl(var_decl) = init.as_ref() {
                        if let Some(Content::Expression(expr)) = var_decl.value.as_ref() {
                            // Lint the collection in the outer env.
                            lint_expression(expr, env)?;

                            // Lint the body in a scope that contains the loop vars.
                            let idents: Vec<String> = var_decl
                                .ident
                                .split(',')
                                .map(|s| s.trim().to_string())
                                .filter(|s| !s.is_empty())
                                .collect();

                            let mut loop_env = Environment::new_with_parent_capacity(env.clone(), idents.len().max(1) + 8);
                            for ident in idents.iter() {
                                loop_env.declare_ref(ident.as_str(), Value::Void, false);
                            }
                            lint_contents_seq(&for_stmt.body, &mut loop_env)?;
                            return Ok(());
                        }
                    }
                }

                lint_statement(init, env)?;
            }
            if let Some(test) = &for_stmt.test {
                lint_expression(test, env)?;
            }
            if let Some(update) = &for_stmt.update {
                lint_expression(update, env)?;
            }
            let mut body_env = Environment::new_with_parent_capacity(env.clone(), 8);
            lint_contents_seq(&for_stmt.body, &mut body_env)?;
        },
        Stmt::WhileStmt(while_stmt) => {
            lint_expression(&while_stmt.test, env)?;
            let mut body_env = Environment::new_with_parent_capacity(env.clone(), 8);
            lint_contents_seq(&while_stmt.body, &mut body_env)?;
        },
        Stmt::TryCatchStmt(try_catch) => {
            let mut try_env = Environment::new_with_parent_capacity(env.clone(), 8);
            lint_contents_seq(&try_catch.try_block, &mut try_env)?;
            if let Some(catch_block) = &try_catch.catch_block {
                let mut catch_env = Environment::new_with_parent_capacity(env.clone(), 8);
                lint_contents_seq(catch_block, &mut catch_env)?;
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
        "math" | "fs" | "os" | "path" | "encoding" | "http" => {
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
