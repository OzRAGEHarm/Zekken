use crate::ast::*;
use crate::environment::{Environment, Value, FunctionValue};
use crate::parser::Parser;
use super::expression::{evaluate_assignment_discard, evaluate_expression};
use crate::errors::{ZekkenError, ErrorKind, push_error};
use crate::libraries::load_library;
use crate::lexer::DataType;
use hashbrown::HashMap;
use std::collections::HashSet;
use std::path::Path;
use std::sync::Arc;
// use std::process;
use super::lint::{lint_statement, lint_expression, lint_include, lint_use};

// Check if the value type matches the expected type
fn check_value_type(value: &Value, expected: &DataType) -> bool {
    match (value, expected) {
        (_, DataType::Any) => true,
        (Value::Int(_), DataType::Int) => true,
        (Value::Float(_), DataType::Float) => true,
        (Value::String(_), DataType::String) => true,
        (Value::Boolean(_), DataType::Bool) => true,
        (Value::Array(_), DataType::Array) => true,
        (Value::Object(_), DataType::Object) => true,
        (Value::Function(_), DataType::Fn) => true,
        _ => false,
    }
}

// Helper function to create a dummy value based on type
fn create_dummy_value(data_type: &DataType) -> Value {
    match data_type {
        DataType::String => Value::String(String::new()),
        DataType::Int => Value::Int(0),
        DataType::Float => Value::Float(0.0),
        DataType::Bool => Value::Boolean(false),
        DataType::Array => Value::Array(vec![]),
        DataType::Object => Value::Object(HashMap::new()),
        DataType::Fn => Value::Function(FunctionValue { 
            params: Arc::new(vec![]), 
            body: Arc::new(vec![]),
            return_type: None,
            needs_parent: false,
            captures: Arc::new(vec![]),
        }),
        _ => Value::Void,
    }
}

#[derive(Default)]
struct ParentUsage {
    requires_parent_clone: bool,
    captures: HashSet<String>,
}

fn analyze_function_parent_usage(params: &[Param], body: &[Box<Content>]) -> ParentUsage {
    let mut locals = HashSet::new();
    for p in params {
        locals.insert(p.ident.clone());
    }

    let mut usage = ParentUsage::default();
    analyze_contents_parent_usage(body, &mut locals, &mut usage);
    usage
}

fn analyze_contents_parent_usage(
    contents: &[Box<Content>],
    locals: &mut HashSet<String>,
    usage: &mut ParentUsage,
) {
    for content in contents {
        match content.as_ref() {
            Content::Statement(stmt) => analyze_stmt_parent_usage(stmt, locals, usage),
            Content::Expression(expr) => analyze_expr_parent_usage(expr, locals, usage),
        }
        if usage.requires_parent_clone {
            return;
        }
    }
}

fn analyze_stmt_parent_usage(stmt: &Stmt, locals: &mut HashSet<String>, usage: &mut ParentUsage) {
    match stmt {
        Stmt::VarDecl(v) => {
            if let Some(value) = &v.value {
                match value {
                    Content::Statement(s) => analyze_stmt_parent_usage(s.as_ref(), locals, usage),
                    Content::Expression(e) => analyze_expr_parent_usage(e.as_ref(), locals, usage),
                }
            }
            locals.insert(v.ident.clone());
        }
        Stmt::FuncDecl(f) => {
            // Nested function semantics are complicated for capture-only mode; keep safe path.
            usage.requires_parent_clone = true;
            locals.insert(f.ident.clone());
        }
        Stmt::Lambda(l) => {
            usage.requires_parent_clone = true;
            locals.insert(l.ident.clone());
        }
        Stmt::ObjectDecl(obj) => {
            for p in &obj.properties {
                analyze_expr_parent_usage(&p.value, locals, usage);
                if usage.requires_parent_clone {
                    return;
                }
            }
        }
        Stmt::IfStmt(i) => {
            analyze_expr_parent_usage(&i.test, locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            let mut then_locals = locals.clone();
            analyze_contents_parent_usage(&i.body, &mut then_locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            if let Some(alt) = &i.alt {
                let mut alt_locals = locals.clone();
                analyze_contents_parent_usage(alt, &mut alt_locals, usage);
            }
        }
        Stmt::ForStmt(f) => {
            usage.requires_parent_clone = true;
            if let Some(init) = &f.init {
                analyze_stmt_parent_usage(init, locals, usage);
            }
        }
        Stmt::WhileStmt(w) => {
            analyze_expr_parent_usage(&w.test, locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            let mut body_locals = locals.clone();
            analyze_contents_parent_usage(&w.body, &mut body_locals, usage);
        }
        Stmt::TryCatchStmt(_) => {
            usage.requires_parent_clone = true;
        }
        Stmt::BlockStmt(b) => {
            let mut body_locals = locals.clone();
            analyze_contents_parent_usage(&b.body, &mut body_locals, usage);
        }
        Stmt::Return(r) => {
            if let Some(v) = &r.value {
                match v.as_ref() {
                    Content::Statement(s) => analyze_stmt_parent_usage(s.as_ref(), locals, usage),
                    Content::Expression(e) => analyze_expr_parent_usage(e.as_ref(), locals, usage),
                }
            }
        }
        Stmt::Use(_) | Stmt::Include(_) | Stmt::Export(_) => {
            usage.requires_parent_clone = true;
        }
        Stmt::Program(_) => {}
    }
}

fn analyze_expr_parent_usage(expr: &Expr, locals: &HashSet<String>, usage: &mut ParentUsage) {
    match expr {
        Expr::Identifier(i) => {
            if !locals.contains(&i.name) {
                usage.captures.insert(i.name.clone());
            }
        }
        Expr::Binary(b) => {
            analyze_expr_parent_usage(&b.left, locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            analyze_expr_parent_usage(&b.right, locals, usage);
        }
        Expr::Call(c) => {
            analyze_expr_parent_usage(&c.callee, locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            for a in &c.args {
                analyze_expr_parent_usage(a, locals, usage);
                if usage.requires_parent_clone {
                    return;
                }
            }
        }
        Expr::Member(m) => {
            analyze_expr_parent_usage(&m.object, locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            match m.property.as_ref() {
                Expr::Identifier(_) | Expr::Property(_) => {}
                other => analyze_expr_parent_usage(other, locals, usage),
            }
        }
        Expr::Assign(a) => {
            if let Expr::Identifier(id) = a.left.as_ref() {
                if !locals.contains(&id.name) {
                    usage.requires_parent_clone = true;
                    return;
                }
            }
            analyze_expr_parent_usage(&a.left, locals, usage);
            if usage.requires_parent_clone {
                return;
            }
            analyze_expr_parent_usage(&a.right, locals, usage);
        }
        Expr::ArrayLit(a) => {
            for e in &a.elements {
                analyze_expr_parent_usage(e, locals, usage);
                if usage.requires_parent_clone {
                    return;
                }
            }
        }
        Expr::ObjectLit(o) => {
            for p in &o.properties {
                analyze_expr_parent_usage(&p.value, locals, usage);
                if usage.requires_parent_clone {
                    return;
                }
            }
        }
        Expr::IntLit(_)
        | Expr::FloatLit(_)
        | Expr::StringLit(_)
        | Expr::BoolLit(_)
        | Expr::Property(_) => {}
    }
}

// Helper function to process a statement for declarations
fn process_statement_scope(stmt: &Stmt, env: &mut Environment) {
    match stmt {
        Stmt::Lambda(lambda) => {
            // Register the lambda function in the environment during the first pass
            let function_value = FunctionValue {
                params: Arc::new(lambda.params.clone()),
                body: Arc::new(lambda.body.clone()),
                return_type: lambda.return_type,
                needs_parent: true,
                captures: Arc::new(vec![]),
            };
            env.declare(lambda.ident.clone(), Value::Function(function_value), lambda.constant);
        },
        Stmt::VarDecl(var_decl) => {
            // Skip type checking for object iteration patterns in for loops
            if var_decl.ident.contains(", ") {
                // This is likely a for-loop pattern, we'll validate types during evaluation
                env.declare(var_decl.ident.clone(), Value::Void, false);
                return;
            }

            // First-pass scope processing should only register declaration shapes.
            // Avoid evaluating expressions here to keep lint preprocessing O(AST size)
            // instead of re-executing value logic.
            let dummy_val = if var_decl.type_ == DataType::Any {
                Value::Void
            } else {
                create_dummy_value(&var_decl.type_)
            };
            env.declare(var_decl.ident.clone(), dummy_val, var_decl.constant);
        },
        Stmt::FuncDecl(func_decl) => {
            // First, register the function itself in the environment
            let function_value = FunctionValue {
                params: Arc::new(func_decl.params.clone()),
                body: Arc::new(func_decl.body.clone()),
                return_type: func_decl.return_type,
                needs_parent: true,
                captures: Arc::new(vec![]),
            };
            env.declare(func_decl.ident.clone(), Value::Function(function_value), false);
        },
        Stmt::BlockStmt(block) => {
            // Process block contents in the current environment
            for content in &block.body {
                if let Content::Statement(stmt) = &**content {
                    process_statement_scope(stmt, env);
                }
            }
        },
        Stmt::ForStmt(for_stmt) => {
            // Process initializer if it exists
            if let Some(init) = &for_stmt.init {
                process_statement_scope(init, env);
            }
            
            // Process the loop body
            for content in &for_stmt.body {
                if let Content::Statement(stmt) = &**content {
                    process_statement_scope(stmt, env);
                }
            }
        },
        Stmt::IfStmt(if_stmt) => {
            for content in &if_stmt.body {
                if let Content::Statement(stmt) = &**content {
                    process_statement_scope(stmt, env);
                }
            }
            if let Some(alt) = &if_stmt.alt {
                for content in alt {
                    if let Content::Statement(stmt) = &**content {
                        process_statement_scope(stmt, env);
                    }
                }
            }
        },
        Stmt::WhileStmt(while_stmt) => {
            for content in &while_stmt.body {
                if let Content::Statement(stmt) = &**content {
                    process_statement_scope(stmt, env);
                }
            }
        },
        Stmt::TryCatchStmt(try_catch) => {
            for content in &try_catch.try_block {
                if let Content::Statement(stmt) = &**content {
                    process_statement_scope(stmt, env);
                }
            }
            if let Some(catch_block) = &try_catch.catch_block {
                for content in catch_block {
                    if let Content::Statement(stmt) = &**content {
                        process_statement_scope(stmt, env);
                    }
                }
            }
        },
        _ => {}
    }
}

// Helper to get a string name for Value type
fn value_type_name(val: &Value) -> &'static str {
    match val {
        Value::Int(_) => "int",
        Value::Float(_) => "float",
        Value::String(_) => "string",
        Value::Boolean(_) => "bool",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
        Value::Function(_) => "",
        Value::NativeFunction(_) => "",
        Value::Void => "void",
        _ => "unknown",
    }
}

fn expr_location(expr: &Expr) -> Location {
    match expr {
        Expr::Assign(e) => e.location.clone(),
        Expr::Member(e) => e.location.clone(),
        Expr::Call(e) => e.location.clone(),
        Expr::Binary(e) => e.location.clone(),
        Expr::Identifier(e) => e.location.clone(),
        Expr::Property(e) => e.location.clone(),
        Expr::IntLit(e) => e.location.clone(),
        Expr::FloatLit(e) => e.location.clone(),
        Expr::StringLit(e) => e.location.clone(),
        Expr::BoolLit(e) => e.location.clone(),
        Expr::ArrayLit(e) => e.location.clone(),
        Expr::ObjectLit(e) => e.location.clone(),
    }
}

// Evaluate a statement and return the result
pub fn evaluate_statement(stmt: &Stmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match stmt {
        Stmt::Program(program) => evaluate_program(program, env),
        Stmt::VarDecl(var_decl) => evaluate_var_declaration(var_decl, env),
        Stmt::FuncDecl(func_decl) => evaluate_function_declaration(func_decl, env),
        Stmt::ObjectDecl(obj_decl) => evaluate_object_declaration(obj_decl, env),
        Stmt::IfStmt(if_stmt) => evaluate_if_statement(if_stmt, env),
        Stmt::ForStmt(for_stmt) => evaluate_for_statement(for_stmt, env),
        Stmt::WhileStmt(while_stmt) => evaluate_while_statement(while_stmt, env),
        Stmt::TryCatchStmt(try_catch) => evaluate_try_catch(try_catch, env),
        Stmt::BlockStmt(block) => evaluate_block(block, env),
        Stmt::Return(ret) => evaluate_return(ret, env),
        Stmt::Lambda(lambda) => evaluate_lambda(lambda, env),
        Stmt::Use(use_stmt) => evaluate_use(use_stmt, env),
        Stmt::Include(include) => evaluate_include(include, env),
        Stmt::Export(exports) => evaluate_export(exports, env),
    }
}

// Evaluate the entire program
fn evaluate_program(program: &Program, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let mut errors = Vec::new();

    // First pass: Process imports and declarations
    for import in &program.imports {
        if let Content::Statement(stmt) = &*import {
            match **stmt {
                Stmt::Include(ref include) => {
                    // First check if the file exists
                    if let Err(e) = lint_include(include) {
                        errors.push(e);
                        continue;
                    }
                    // If file exists, evaluate it to set up the environment
                    if let Err(e) = evaluate_include(include, env) {
                        errors.push(e);
                    }
                },
                Stmt::Use(ref use_stmt) => {
                    // First check if the library is valid
                    if let Err(e) = lint_use(use_stmt) {
                        errors.push(e);
                        continue;
                    }
                    // If library is valid, load it to set up the environment
                    if let Err(e) = evaluate_use(use_stmt, env) {
                        errors.push(e);
                    }
                },
                _ => errors.push(ZekkenError::syntax(
                    "Invalid import statement",
                    0,
                    0,
                    None,
                    None,
                ))
            }
        }
    }

    // If there were import errors, report them and stop (except internal errors)
    if !errors.is_empty() {
        for error in errors {
            if error.kind == ErrorKind::Internal {
                continue; // Skip internal errors
            }
            push_error(error.clone());
        }
        // Just return an error to stop execution, but don't log it
        return Err(ZekkenError::internal("Import errors found"));
    }

    // Process top-level declarations using same environment
    let mut lint_errors = Vec::new();

    // Process all top-level statements in the same environment
    for content in &program.content {
        if let Content::Statement(stmt) = &**content {
            process_statement_scope(stmt, env);
        }
    }

    // Second pass: Now lint everything with the complete environment
    for content in &program.content {
        match &**content {
            Content::Statement(stmt) => {
                if let Err(e) = lint_statement(stmt, env) {
                    lint_errors.push(e);
                }
            },
            Content::Expression(expr) => {
                if let Err(e) = lint_expression(expr, env) {
                    lint_errors.push(e);
                }
            }
        }
    }

    // Report lint errors and stop before execution.
    if !lint_errors.is_empty() {
        for error in lint_errors {
            if error.kind == ErrorKind::Internal {
                continue; // Skip internal errors
            }
            push_error(error.clone());
        }
        return Err(ZekkenError::internal("Linting errors found"));
    }
    
    // Imports/declarations were already applied directly to env during lint setup.
    // Execute only top-level content to avoid duplicate import work and side effects.
    evaluate_block_content(&program.content, env)
}

// Handle variable declarations
fn evaluate_var_declaration(decl: &VarDecl, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let value = match &decl.value {
        Some(content) => match content {
            Content::Expression(expr) => {
                let val = evaluate_expression(expr, env)?;
                if !check_value_type(&val, &decl.type_) {
                    let loc = expr_location(expr);
                    return Err(ZekkenError::type_error(
                        &format!("Type mismatch in variable declaration '{}'", decl.ident),
                        &format!("{:?}", decl.type_),
                        value_type_name(&val),
                        loc.line,
                        loc.column
                    ));
                }
                val
            },
            Content::Statement(stmt) => {
                if let Some(val) = evaluate_statement(stmt, env)? {
                    val
                } else {
                    Value::Void
                }
            },
        },
        None => Value::Void,
    };

    env.declare_ref_typed(&decl.ident, value, decl.type_, decl.constant);
    Ok(None)
}

// Handle function declarations
fn evaluate_function_declaration(func: &FuncDecl, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let usage = analyze_function_parent_usage(&func.params, &func.body);
    let captures = if usage.requires_parent_clone {
        vec![]
    } else {
        let mut v: Vec<String> = usage.captures.into_iter().collect();
        v.sort_unstable();
        v
    };
    let function_value = FunctionValue {
        params: Arc::new(func.params.clone()),
        body: Arc::new(func.body.clone()),
        return_type: func.return_type,
        needs_parent: usage.requires_parent_clone,
        captures: Arc::new(captures),
    };

    env.declare(func.ident.clone(), Value::Function(function_value), false);
    Ok(None)
}

// Handle object declarations
fn evaluate_object_declaration(obj: &ObjectDecl, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let mut object_map = HashMap::new();
    let mut keys = Vec::new();
    for property in &obj.properties {
        let value = evaluate_expression(&property.value, env)
            .map_err(|e| ZekkenError::type_error(
                &format!("Failed to evaluate property '{}': {}", property.key, e),
                "object",
                "property evaluation failed",
                obj.location.line,
                obj.location.column
            ))?;
        keys.push(property.key.clone());
        object_map.insert(property.key.clone(), value);
    }
    object_map.insert("__keys__".to_string(), Value::Array(keys.iter().map(|k| Value::String(k.clone())).collect()));
    env.declare(obj.ident.clone(), Value::Object(object_map), false);
    Ok(None)
}

// Handle if statements
fn evaluate_if_statement(if_stmt: &IfStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    #[inline]
    fn eval_single_content_or_block(
        content: &[Box<Content>],
        env: &mut Environment,
    ) -> Result<Option<Value>, ZekkenError> {
        if content.len() == 1 {
            match content[0].as_ref() {
                Content::Statement(stmt) => evaluate_statement(stmt, env),
                Content::Expression(expr) => Ok(Some(evaluate_expression(expr, env)?)),
            }
        } else {
            evaluate_block_content(content, env)
        }
    }

    fn try_eval_fast_if_test(test: &Expr, env: &Environment, line: usize, column: usize) -> Result<Option<bool>, ZekkenError> {
        #[inline]
        fn lookup_local_or_chain<'a>(env: &'a Environment, name: &str) -> Option<&'a Value> {
            env.variables
                .get(name)
                .or_else(|| env.constants.get(name))
                .or_else(|| env.lookup_ref(name))
        }

        #[inline]
        fn string_tag(v: &str) -> Option<u8> {
            if v.len() == 1 {
                Some(v.as_bytes()[0])
            } else {
                None
            }
        }

        #[inline]
        fn expr_string_tag(expr: &Expr, env: &Environment) -> Option<u8> {
            match expr {
                Expr::StringLit(s) => string_tag(&s.value),
                Expr::Identifier(id) => match lookup_local_or_chain(env, &id.name) {
                    Some(Value::String(v)) => string_tag(v),
                    _ => None,
                },
                _ => None,
            }
        }

        fn as_num(v: &Value) -> Option<f64> {
            match v {
                Value::Int(i) => Some(*i as f64),
                Value::Float(f) => Some(*f),
                _ => None,
            }
        }
        fn eval_num_expr(expr: &Expr, env: &Environment) -> Option<f64> {
            match expr {
                Expr::IntLit(i) => Some(i.value as f64),
                Expr::FloatLit(f) => Some(f.value),
                Expr::Identifier(id) => lookup_local_or_chain(env, &id.name).and_then(as_num),
                Expr::Binary(bin) => {
                    let l = eval_num_expr(&bin.left, env)?;
                    let r = eval_num_expr(&bin.right, env)?;
                    match bin.operator.as_str() {
                        "+" => Some(l + r),
                        "-" => Some(l - r),
                        "*" => Some(l * r),
                        "/" => {
                            if r == 0.0 { None } else { Some(l / r) }
                        }
                        "%" => {
                            if r == 0.0 || l.fract() != 0.0 || r.fract() != 0.0 {
                                None
                            } else {
                                Some((l as i64 % r as i64) as f64)
                            }
                        }
                        _ => None,
                    }
                }
                _ => None,
            }
        }
        fn eval_int_mod_compare_side<'a>(
            expr: &'a Expr,
            env: &Environment,
        ) -> Option<(i64, &'a str)> {
            match expr {
                Expr::Binary(bin) if bin.operator == "%" => {
                    let l = match bin.left.as_ref() {
                        Expr::Identifier(id) => match lookup_local_or_chain(env, &id.name) {
                            Some(Value::Int(v)) => *v,
                            _ => return None,
                        },
                        Expr::IntLit(i) => i.value,
                        _ => return None,
                    };
                    let r = match bin.right.as_ref() {
                        Expr::IntLit(i) if i.value != 0 => i.value,
                        Expr::Identifier(id) => match lookup_local_or_chain(env, &id.name) {
                            Some(Value::Int(v)) if *v != 0 => *v,
                            _ => return None,
                        },
                        _ => return None,
                    };
                    Some((l % r, "%"))
                }
                Expr::Identifier(id) => match lookup_local_or_chain(env, &id.name) {
                    Some(Value::Int(v)) => Some((*v, "id")),
                    _ => None,
                },
                Expr::IntLit(i) => Some((i.value, "lit")),
                _ => None,
            }
        }

        #[inline]
        fn int_val(expr: &Expr, env: &Environment) -> Option<i64> {
            match expr {
                Expr::IntLit(i) => Some(i.value),
                Expr::Identifier(id) => {
                    if let Some(v) = env.variables.get(&id.name).or_else(|| env.constants.get(&id.name)) {
                        return match v {
                            Value::Int(i) => Some(*i),
                            _ => None,
                        };
                    }
                    match env.lookup_ref(&id.name) {
                        Some(Value::Int(i)) => Some(*i),
                        _ => None,
                    }
                }
                _ => None,
            }
        }

        #[inline]
        fn mod_val(expr: &Expr, env: &Environment) -> Option<i64> {
            match expr {
                Expr::Binary(bin) if bin.operator == "%" => {
                    let l = int_val(bin.left.as_ref(), env)?;
                    let r = int_val(bin.right.as_ref(), env)?;
                    if r == 0 { None } else { Some(l % r) }
                }
                _ => int_val(expr, env),
            }
        }

        match test {
            Expr::Identifier(id) => {
                return match lookup_local_or_chain(env, &id.name) {
                    Some(Value::Boolean(b)) => Ok(Some(*b)),
                    Some(other) => Err(ZekkenError::type_error(
                        "If statement condition must evaluate to a boolean",
                        "bool",
                        value_type_name(other),
                        line,
                        column,
                    )),
                    None => Err(ZekkenError::reference(
                        &format!("Variable '{}' not found", id.name),
                        "variable",
                        line,
                        column,
                    )),
                };
            }
            Expr::Binary(bin) => {
                let op = bin.operator.as_str();
                let left = bin.left.as_ref();
                let right = bin.right.as_ref();

                if matches!(op, "==" | "!=") {
                    if let (Some(lv), Some(rv)) = (mod_val(left, env), mod_val(right, env)) {
                        let eq = lv == rv;
                        return Ok(Some(if op == "==" { eq } else { !eq }));
                    }

                    if let (Some(lt), Some(rt)) = (expr_string_tag(left, env), expr_string_tag(right, env)) {
                        let eq = lt == rt;
                        return Ok(Some(if op == "==" { eq } else { !eq }));
                    }

                    // Super-hot fast path for integer modulo/equality tests:
                    // e.g. state % 4 == 0, state % 4 == 1, ...
                    if let (Some((lv, _)), Some((rv, _))) =
                        (eval_int_mod_compare_side(left, env), eval_int_mod_compare_side(right, env))
                    {
                        return Ok(Some(if op == "==" { lv == rv } else { lv != rv }));
                    }

                    let out = match (left, right) {
                        (Expr::Identifier(id), Expr::StringLit(s)) => match lookup_local_or_chain(env, &id.name) {
                            Some(Value::String(v)) => Some(v == &s.value),
                            _ => None,
                        },
                        (Expr::StringLit(s), Expr::Identifier(id)) => match lookup_local_or_chain(env, &id.name) {
                            Some(Value::String(v)) => Some(s.value == *v),
                            _ => None,
                        },
                        (Expr::Identifier(id), Expr::BoolLit(b)) => match lookup_local_or_chain(env, &id.name) {
                            Some(Value::Boolean(v)) => Some(*v == b.value),
                            _ => None,
                        },
                        (Expr::BoolLit(b), Expr::Identifier(id)) => match lookup_local_or_chain(env, &id.name) {
                            Some(Value::Boolean(v)) => Some(b.value == *v),
                            _ => None,
                        },
                        (Expr::Identifier(lid), Expr::Identifier(rid)) => match (
                            lookup_local_or_chain(env, &lid.name),
                            lookup_local_or_chain(env, &rid.name),
                        ) {
                            (Some(Value::String(lv)), Some(Value::String(rv))) => Some(lv == rv),
                            (Some(Value::Boolean(lv)), Some(Value::Boolean(rv))) => Some(lv == rv),
                            (Some(lv), Some(rv)) => match (as_num(lv), as_num(rv)) {
                                (Some(ln), Some(rn)) => Some(ln == rn),
                                _ => None,
                            },
                            _ => None,
                        },
                        _ => None,
                    };
                    if let Some(eq) = out {
                        return Ok(Some(if op == "==" { eq } else { !eq }));
                    }

                    if let (Some(l), Some(r)) = (eval_num_expr(left, env), eval_num_expr(right, env)) {
                        return Ok(Some(if op == "==" { l == r } else { l != r }));
                    }
                }

                if matches!(op, "<" | "<=" | ">" | ">=") {
                    let lnum = eval_num_expr(left, env);
                    let rnum = eval_num_expr(right, env);

                    if let (Some(l), Some(r)) = (lnum, rnum) {
                        let b = match op {
                            "<" => l < r,
                            "<=" => l <= r,
                            ">" => l > r,
                            _ => l >= r,
                        };
                        return Ok(Some(b));
                    }
                }
            }
            _ => {}
        }

        Ok(None)
    }

    #[derive(Clone, Copy)]
    enum ChainSpec<'a> {
        ModEq { ident: &'a str, divisor: i64 },
        StrEq { ident: &'a str },
    }

    #[derive(Clone, Copy)]
    enum ChainKey {
        Int(i64),
        Char(u8),
    }

    fn nested_else_if<'a>(alt: &'a Option<Vec<Box<Content>>>) -> Option<&'a IfStmt> {
        let block = alt.as_ref()?;
        if block.len() != 1 {
            return None;
        }
        match block[0].as_ref() {
            Content::Statement(stmt) => match stmt.as_ref() {
                Stmt::IfStmt(i) => Some(i),
                _ => None,
            },
            _ => None,
        }
    }

    fn parse_mod_eq<'a>(expr: &'a Expr) -> Option<(&'a str, i64, i64)> {
        let bin = match expr {
            Expr::Binary(b) if b.operator == "==" => b,
            _ => return None,
        };
        let (mod_expr, val_expr) = match (bin.left.as_ref(), bin.right.as_ref()) {
            (Expr::Binary(m), v) if m.operator == "%" => (m, v),
            (v, Expr::Binary(m)) if m.operator == "%" => (m, v),
            _ => return None,
        };
        let ident = match mod_expr.left.as_ref() {
            Expr::Identifier(id) => id.name.as_str(),
            _ => return None,
        };
        let divisor = match mod_expr.right.as_ref() {
            Expr::IntLit(i) if i.value != 0 => i.value,
            _ => return None,
        };
        let expected = match val_expr {
            Expr::IntLit(i) => i.value,
            _ => return None,
        };
        Some((ident, divisor, expected))
    }

    fn parse_str_eq<'a>(expr: &'a Expr) -> Option<(&'a str, u8)> {
        let bin = match expr {
            Expr::Binary(b) if b.operator == "==" => b,
            _ => return None,
        };
        let (ident, lit) = match (bin.left.as_ref(), bin.right.as_ref()) {
            (Expr::Identifier(id), Expr::StringLit(s)) => (id.name.as_str(), s),
            (Expr::StringLit(s), Expr::Identifier(id)) => (id.name.as_str(), s),
            _ => return None,
        };
        if lit.value.len() != 1 {
            return None;
        }
        Some((ident, lit.value.as_bytes()[0]))
    }

    fn lookup_int(env: &Environment, name: &str) -> Option<i64> {
        let v = env
            .variables
            .get(name)
            .or_else(|| env.constants.get(name))
            .or_else(|| env.lookup_ref(name))?;
        match v {
            Value::Int(i) => Some(*i),
            _ => None,
        }
    }

    fn lookup_char(env: &Environment, name: &str) -> Option<u8> {
        let v = env
            .variables
            .get(name)
            .or_else(|| env.constants.get(name))
            .or_else(|| env.lookup_ref(name))?;
        match v {
            Value::String(s) if s.len() == 1 => Some(s.as_bytes()[0]),
            _ => None,
        }
    }

    fn try_eval_chain_switch<'a>(if_stmt: &'a IfStmt, env: &Environment) -> Option<(ChainKey, ChainSpec<'a>)> {
        if let Some((ident, divisor, _)) = parse_mod_eq(&if_stmt.test) {
            let mut cur = if_stmt;
            loop {
                let (id2, div2, _) = parse_mod_eq(&cur.test)?;
                if id2 != ident || div2 != divisor {
                    return None;
                }
                if let Some(next) = nested_else_if(&cur.alt) {
                    cur = next;
                } else {
                    break;
                }
            }
            let key = lookup_int(env, ident)? % divisor;
            return Some((ChainKey::Int(key), ChainSpec::ModEq { ident, divisor }));
        }

        if let Some((ident, _)) = parse_str_eq(&if_stmt.test) {
            let mut cur = if_stmt;
            loop {
                let (id2, _) = parse_str_eq(&cur.test)?;
                if id2 != ident {
                    return None;
                }
                if let Some(next) = nested_else_if(&cur.alt) {
                    cur = next;
                } else {
                    break;
                }
            }
            let key = lookup_char(env, ident)?;
            return Some((ChainKey::Char(key), ChainSpec::StrEq { ident }));
        }

        None
    }

    fn expected_for_case(test: &Expr, spec: ChainSpec<'_>) -> Option<ChainKey> {
        match spec {
            ChainSpec::ModEq { ident, divisor } => {
                let (id2, div2, expected) = parse_mod_eq(test)?;
                if id2 == ident && div2 == divisor {
                    Some(ChainKey::Int(expected))
                } else {
                    None
                }
            }
            ChainSpec::StrEq { ident } => {
                let (id2, expected) = parse_str_eq(test)?;
                if id2 == ident {
                    Some(ChainKey::Char(expected))
                } else {
                    None
                }
            }
        }
    }

    if let Some((key, spec)) = try_eval_chain_switch(if_stmt, env) {
        let mut cur = if_stmt;
        loop {
            if let Some(expected) = expected_for_case(&cur.test, spec) {
                let matched = match (key, expected) {
                    (ChainKey::Int(a), ChainKey::Int(b)) => a == b,
                    (ChainKey::Char(a), ChainKey::Char(b)) => a == b,
                    _ => false,
                };
                if matched {
                    return eval_single_content_or_block(&cur.body, env);
                }
            }
            if let Some(next) = nested_else_if(&cur.alt) {
                cur = next;
                continue;
            }
            if let Some(alt) = &cur.alt {
                return eval_single_content_or_block(alt, env);
            }
            break;
        }
        return Ok(None);
    }

    if let Some(test_true) = try_eval_fast_if_test(&if_stmt.test, env, if_stmt.location.line, if_stmt.location.column)? {
        return if test_true {
            eval_single_content_or_block(&if_stmt.body, env)
        } else if let Some(alt) = &if_stmt.alt {
            eval_single_content_or_block(alt, env)
        } else {
            Ok(None)
        };
    }

    let test_result = evaluate_expression(&if_stmt.test, env)?;
    match test_result {
        Value::Boolean(true) => eval_single_content_or_block(&if_stmt.body, env),
        Value::Boolean(false) => {
            if let Some(alt) = &if_stmt.alt {
                eval_single_content_or_block(alt, env)
            } else {
                Ok(None)
            }
        }
        _ => Err(ZekkenError::type_error(
            "If statement condition must evaluate to a boolean",
            "bool",
            value_type_name(&test_result),
            if_stmt.location.line,
            if_stmt.location.column
        ))
    }
}

// Handle for statements
fn evaluate_for_statement(for_stmt: &ForStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    if let Some(ref init) = for_stmt.init {
        if let Stmt::VarDecl(var_decl) = &**init {
            let collection_value = match &var_decl.value {
                Some(content) => match content {
                    Content::Expression(expr) => evaluate_expression(expr, env)?,
                    _ => return Err(ZekkenError::runtime(
                        "Expected expression in for loop initialization",
                        for_stmt.location.line,
                        for_stmt.location.column,
                        Some("for |x| in array { ... }"),
                    )),
                },
                None => return Err(ZekkenError::runtime(
                    "For loop initialization requires a value",
                    for_stmt.location.line,
                    for_stmt.location.column,
                    None,
                )),
            };
            match collection_value {
                Value::Object(ref map) => evaluate_for_object(map, var_decl, &for_stmt.body, env),
                Value::Array(arr) => evaluate_for_array(arr, var_decl, &for_stmt.body, env),
                _ => Err(ZekkenError::type_error(
                    "For loop must iterate over an object or array",
                    "object or array",
                    value_type_name(&collection_value),
                    for_stmt.location.line,
                    for_stmt.location.column
                ))
            }
        } else {
            Err(ZekkenError::runtime(
                "For loop requires a variable declaration",
                for_stmt.location.line,
                for_stmt.location.column,
                None
            ))
        }
    } else {
        Err(ZekkenError::runtime(
            "For loop requires an initialization",
            for_stmt.location.line,
            for_stmt.location.column,
            None
        ))
    }
}

// Handle while statements
fn evaluate_while_statement(while_stmt: &WhileStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    #[derive(Clone)]
    enum NumCondOperand {
        Ident(String),
        Int(i64),
        Float(f64),
    }

    #[derive(Copy, Clone)]
    enum NumCondOp {
        Lt,
        Lte,
        Gt,
        Gte,
        Eq,
        Neq,
    }

    #[derive(Clone)]
    struct NumCond {
        left: NumCondOperand,
        right: NumCondOperand,
        op: NumCondOp,
    }

    fn as_num_operand(expr: &Expr) -> Option<NumCondOperand> {
        match expr {
            Expr::Identifier(id) => Some(NumCondOperand::Ident(id.name.clone())),
            Expr::IntLit(i) => Some(NumCondOperand::Int(i.value)),
            Expr::FloatLit(f) => Some(NumCondOperand::Float(f.value)),
            _ => None,
        }
    }

    fn as_num_value(op: &NumCondOperand, env: &Environment) -> Option<f64> {
        match op {
            NumCondOperand::Int(i) => Some(*i as f64),
            NumCondOperand::Float(f) => Some(*f),
            NumCondOperand::Ident(name) => {
                if let Some(v) = env.variables.get(name).or_else(|| env.constants.get(name)) {
                    return match v {
                        Value::Int(i) => Some(*i as f64),
                        Value::Float(f) => Some(*f),
                        _ => None,
                    };
                }
                match env.lookup_ref(name) {
                    Some(Value::Int(i)) => Some(*i as f64),
                    Some(Value::Float(f)) => Some(*f),
                    _ => None,
                }
            }
        }
    }
    fn as_int_value(op: &NumCondOperand, env: &Environment) -> Option<i64> {
        match op {
            NumCondOperand::Int(i) => Some(*i),
            NumCondOperand::Float(f) => {
                if f.fract() == 0.0 { Some(*f as i64) } else { None }
            }
            NumCondOperand::Ident(name) => {
                if let Some(v) = env.variables.get(name).or_else(|| env.constants.get(name)) {
                    return match v {
                        Value::Int(i) => Some(*i),
                        Value::Float(f) if f.fract() == 0.0 => Some(*f as i64),
                        _ => None,
                    };
                }
                match env.lookup_ref(name) {
                    Some(Value::Int(i)) => Some(*i),
                    Some(Value::Float(f)) if f.fract() == 0.0 => Some(*f as i64),
                    _ => None,
                }
            }
        }
    }

    fn build_numeric_cond(test: &Expr) -> Option<NumCond> {
        let bin = match test {
            Expr::Binary(b) => b,
            _ => return None,
        };
        let op = match bin.operator.as_str() {
            "<" => NumCondOp::Lt,
            "<=" => NumCondOp::Lte,
            ">" => NumCondOp::Gt,
            ">=" => NumCondOp::Gte,
            "==" => NumCondOp::Eq,
            "!=" => NumCondOp::Neq,
            _ => return None,
        };
        let left = as_num_operand(&bin.left)?;
        let right = as_num_operand(&bin.right)?;
        Some(NumCond { left, right, op })
    }

    let body_may_return = block_has_return(&while_stmt.body);

    if let Some(cond) = build_numeric_cond(&while_stmt.test) {
        let mut result = None;
        loop {
            let test_true = if let (Some(l), Some(r)) = (as_int_value(&cond.left, env), as_int_value(&cond.right, env)) {
                match cond.op {
                    NumCondOp::Lt => l < r,
                    NumCondOp::Lte => l <= r,
                    NumCondOp::Gt => l > r,
                    NumCondOp::Gte => l >= r,
                    NumCondOp::Eq => l == r,
                    NumCondOp::Neq => l != r,
                }
            } else {
                let l = as_num_value(&cond.left, env).ok_or_else(|| {
                    ZekkenError::type_error(
                        "While loop condition must evaluate to a boolean",
                        "bool",
                        "non-boolean",
                        while_stmt.location.line,
                        while_stmt.location.column,
                    )
                })?;
                let r = as_num_value(&cond.right, env).ok_or_else(|| {
                    ZekkenError::type_error(
                        "While loop condition must evaluate to a boolean",
                        "bool",
                        "non-boolean",
                        while_stmt.location.line,
                        while_stmt.location.column,
                    )
                })?;
                match cond.op {
                    NumCondOp::Lt => l < r,
                    NumCondOp::Lte => l <= r,
                    NumCondOp::Gt => l > r,
                    NumCondOp::Gte => l >= r,
                    NumCondOp::Eq => l == r,
                    NumCondOp::Neq => l != r,
                }
            };

            if !test_true {
                break;
            }
            if body_may_return {
                result = evaluate_block_content(&while_stmt.body, env)?;
            } else {
                evaluate_block_discard(&while_stmt.body, env)?;
            }
        }
        return Ok(result);
    }

    let mut result = None;
    loop {
        let test_result = evaluate_expression(&while_stmt.test, env)?;
        match test_result {
            Value::Boolean(true) => {
                if body_may_return {
                    result = evaluate_block_content(&while_stmt.body, env)?;
                } else {
                    evaluate_block_discard(&while_stmt.body, env)?;
                }
            }
            Value::Boolean(false) => break,
            _ => {
                return Err(ZekkenError::type_error(
                    "While loop condition must evaluate to a boolean",
                    "bool",
                    value_type_name(&test_result),
                    while_stmt.location.line,
                    while_stmt.location.column
                ))
            }
        }
    }
    Ok(result)
}

// Handle try-catch statements
fn evaluate_try_catch(try_catch: &TryCatchStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match evaluate_block_content(&try_catch.try_block, env) {
        Ok(value) => Ok(value),
        Err(error) => {
            if let Some(catch_block) = &try_catch.catch_block {
                let mut err_obj = HashMap::new();
                err_obj.insert("message".to_string(), Value::String(error.message.clone()));
                err_obj.insert("kind".to_string(), Value::String(format!("{:?}", error.kind)));
                err_obj.insert("line".to_string(), Value::Int(error.context.line as i64));
                err_obj.insert("column".to_string(), Value::Int(error.context.column as i64));
                // Add the pretty error string for display
                err_obj.insert("__zekken_error__".to_string(), Value::String(error.to_string()));

                let prev_var = env.variables.remove("e");
                let prev_const = env.constants.remove("e");
                env.declare("e".to_string(), Value::Object(err_obj), false);

                let catch_result = evaluate_block_content(catch_block, env);

                env.variables.remove("e");
                if let Some(v) = prev_var {
                    env.variables.insert("e".to_string(), v);
                }
                if let Some(c) = prev_const {
                    env.constants.insert("e".to_string(), c);
                }

                catch_result
            } else {
                Err(error)
            }
        }
    }
}

fn stmt_has_return(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return(_) => true,
        Stmt::IfStmt(i) => {
            block_has_return(&i.body) || i.alt.as_ref().map(|b| block_has_return(b)).unwrap_or(false)
        }
        Stmt::ForStmt(f) => block_has_return(&f.body),
        Stmt::WhileStmt(w) => block_has_return(&w.body),
        Stmt::TryCatchStmt(t) => {
            block_has_return(&t.try_block)
                || t.catch_block
                    .as_ref()
                    .map(|b| block_has_return(b))
                    .unwrap_or(false)
        }
        Stmt::BlockStmt(b) => block_has_return(&b.body),
        Stmt::Program(p) => {
            p.imports.iter().any(|c| content_has_return(c))
                || p.content.iter().any(|c| content_has_return(c))
        }
        // Nested function/lambda returns do not affect outer control flow.
        Stmt::FuncDecl(_)
        | Stmt::Lambda(_)
        | Stmt::VarDecl(_)
        | Stmt::ObjectDecl(_)
        | Stmt::Use(_)
        | Stmt::Include(_)
        | Stmt::Export(_) => false,
    }
}

fn content_has_return(content: &Content) -> bool {
    match content {
        Content::Statement(stmt) => stmt_has_return(stmt),
        Content::Expression(_) => false,
    }
}

fn block_has_return(content: &[Box<Content>]) -> bool {
    content.iter().any(|c| content_has_return(c))
}

fn evaluate_block_discard(content: &[Box<Content>], env: &mut Environment) -> Result<(), ZekkenError> {
    for item in content {
        match item.as_ref() {
            Content::Statement(stmt) => {
                let _ = evaluate_statement(stmt, env)?;
            }
            Content::Expression(expr) => match expr.as_ref() {
                Expr::Assign(assign) => evaluate_assignment_discard(assign, env)?,
                _ => {
                    let _ = evaluate_expression(expr, env)?;
                }
            },
        }
    }
    Ok(())
}

// Handle code blocks
fn evaluate_block(block: &BlockStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    evaluate_block_content(&block.body, env)
}

// Handle code block content
fn evaluate_block_content(content: &[Box<Content>], env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    if content.is_empty() {
        return Ok(None);
    }

    if content.len() == 1 {
        return match content[0].as_ref() {
            Content::Statement(stmt) => evaluate_statement(stmt, env),
            Content::Expression(expr) => Ok(Some(evaluate_expression(expr, env)?)),
        };
    }

    let (last, rest) = content.split_last().unwrap();

    for item in rest {
        match item.as_ref() {
            Content::Statement(stmt) => {
                let _ = evaluate_statement(stmt, env)?;
            }
            Content::Expression(expr) => match expr.as_ref() {
                Expr::Assign(assign) => {
                    evaluate_assignment_discard(assign, env)?;
                }
                _ => {
                    let _ = evaluate_expression(expr, env)?;
                }
            },
        }
    }

    match last.as_ref() {
        Content::Statement(stmt) => {
            evaluate_statement(stmt, env)
        }
        Content::Expression(expr) => {
            Ok(Some(evaluate_expression(expr, env)?))
        }
    }
}

// Handle return values in functions
fn evaluate_return(ret: &ReturnStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match &ret.value {
        Some(content) => match &**content {
            Content::Expression(expr) => {
                let value = evaluate_expression(expr, env)?;
                Ok(Some(value))
            },
            Content::Statement(stmt) => evaluate_statement(stmt, env),
        },
        None => Ok(Some(Value::Void)),
    }
}

// Handle lambda expressions
fn evaluate_lambda(lambda: &LambdaDecl, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    let usage = analyze_function_parent_usage(&lambda.params, &lambda.body);
    let captures = if usage.requires_parent_clone {
        vec![]
    } else {
        let mut v: Vec<String> = usage.captures.into_iter().collect();
        v.sort_unstable();
        v
    };
    let function_value = FunctionValue {
        params: Arc::new(lambda.params.clone()),
        body: Arc::new(lambda.body.clone()),
        return_type: lambda.return_type,
        needs_parent: usage.requires_parent_clone,
        captures: Arc::new(captures),
    };

    env.declare(lambda.ident.clone(), Value::Function(function_value), lambda.constant);
    Ok(None)
}

// Handle use statements for importing libraries
fn evaluate_use(use_stmt: &UseStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    match load_library(&use_stmt.module, env) {
        Ok(_) => {
            // If specific methods are requested, extract them from the library object
            if let Some(methods) = &use_stmt.methods {
                // Get the library object
                if let Some(Value::Object(lib_obj)) = env.lookup(&use_stmt.module) {
                    // Import each requested method directly into the target environment
                    for method in methods {
                        if let Some(value) = lib_obj.get(method) {
                            env.declare(method.clone(), value.clone(), false);
                        } else {
                            return Err(ZekkenError::runtime(
                                &format!("Method '{}' not found in library '{}'", method, use_stmt.module),
                                use_stmt.location.line,
                                use_stmt.location.column,
                                None,
                            ));
                        }
                    }
                }
            }
            Ok(None)
        },
        Err(e) => Err(ZekkenError::runtime(
            &format!("Failed to load library '{}': {}", use_stmt.module, e),
            use_stmt.location.line,
            use_stmt.location.column,
            None,
        )),
    }
}

// Handle include statements for including external files
fn evaluate_include(include: &IncludeStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    // Get the directory of the current file being processed
    let current_file = std::env::var("ZEKKEN_CURRENT_FILE").unwrap_or_else(|_| "<unknown>".to_string());
    let current_dir = if current_file == "<unknown>" {
        env.lookup("ZEKKEN_CURRENT_DIR")
            .and_then(|v| if let Value::String(s) = v { Some(s) } else { None })
            .unwrap_or_default()
    } else {
        Path::new(&current_file)
            .parent()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default()
    };

    // Always resolve paths relative to current file's directory
    let mut path = std::path::PathBuf::from(&current_dir);
    path.push(&include.file_path);

    // Try to canonicalize but don't require it to succeed
    let file_path = path.to_string_lossy().to_string();
    
    let file_contents = std::fs::read_to_string(&file_path)
        .map_err(|e| ZekkenError::runtime(
            &format!("Failed to include file '{}': {}", file_path, e),
            include.location.line,
            include.location.column,
            None,
        ))?;

    // Save previous file context
    let prev_file = std::env::var("ZEKKEN_CURRENT_FILE").unwrap_or_else(|_| "<unknown>".to_string());
    // Set current file context to included file
    std::env::set_var("ZEKKEN_CURRENT_FILE", &file_path);

    let mut parser = Parser::new();
    let included_ast = parser.produce_ast(file_contents);
    if !parser.errors.is_empty() {
        for parse_error in parser.errors {
            push_error(parse_error);
        }
        return Err(ZekkenError::syntax(
            "Failed to parse included file",
            include.location.line,
            include.location.column,
            Some("valid zekken source"),
            Some(&include.file_path),
        ));
    }

    // Create a new child environment with current env as parent
    let mut child_env = Environment::new_with_parent(env.clone());

    // Evaluate included AST in child environment
    let result = evaluate_statement(&Stmt::Program(included_ast), &mut child_env);

    // Restore previous file context
    std::env::set_var("ZEKKEN_CURRENT_FILE", prev_file);

    result?;

    // Copy exported methods or all variables from child_env to current env
    match &include.methods {
        Some(methods) => {
            for method in methods {
                if let Some(value) = child_env.lookup(method) {
                    env.declare(method.clone(), value, false);
                } else {
                    return Err(ZekkenError::runtime(
                        &format!("Method '{}' not found in included file", method),
                        include.location.line,
                        include.location.column,
                        None,
                    ));
                }
            }
        }
        None => {
            for (name, value) in &child_env.variables {
                env.declare(name.clone(), value.clone(), false);
            }
        }
    }

    Ok(None)
}

// Handle export statements
fn evaluate_export(exports: &ExportStmt, env: &mut Environment) -> Result<Option<Value>, ZekkenError> {
    for name in &exports.exports {
        if let Some(value) = env.lookup(name) {
            env.declare(name.clone(), value, false);
        } else {
            return Err(ZekkenError::runtime(
                &format!("Cannot export undefined value '{}'", name),
                exports.location.line,
                exports.location.column,
                None,
            ));
        }
    }
    Ok(None)
}

fn set_or_declare_loop_var(env: &mut Environment, name: &str, value: Value) {
    if let Some(slot) = env.variables.get_mut(name) {
        *slot = value;
    } else {
        env.declare_ref(name, value, false);
    }
}

// Handle for loop iterations over objects
fn evaluate_for_object(
    map: &HashMap<String, Value>,
    var_decl: &VarDecl,
    body: &[Box<Content>],
    env: &mut Environment
) -> Result<Option<Value>, ZekkenError> {
    // Extract key and value identifiers
    let idents: Vec<String> = var_decl.ident.split(", ").map(|s| s.to_string()).collect();
    if idents.len() != 2 {
        return Err(ZekkenError::syntax(
            "Object iteration requires two identifiers (key, value)",
            var_decl.location.line,
            var_decl.location.column,
            None,
            None,
        ));
    }
    
    // Get the keys array from the object
    let keys = if let Some(Value::Array(keys)) = map.get("__keys__") {
        keys
    } else {
        return Err(ZekkenError::type_error(
            "Object missing key order",
            "array",
            "missing",
            var_decl.location.line,
            var_decl.location.column,
        ));
    };

    // Match array iteration semantics: bind/update loop vars in the *current* env.
    //
    // This avoids cloning environments per iteration (performance) and makes the value
    // identifier usable in nested statements (e.g. `for ... in value { ... }`, `let x = value.first`).
    set_or_declare_loop_var(env, &idents[0], Value::String(String::new()));
    set_or_declare_loop_var(env, &idents[1], Value::Void);

    for key_val in keys {
        if let Value::String(ref key) = key_val {
            if let Some(value) = map.get(key) {
                // Check if the value matches the declared type
                if var_decl.type_ != DataType::Any && !check_value_type(value, &var_decl.type_) {
                    return Err(ZekkenError::type_error(
                        &format!("Type mismatch in for loop value: expected {:?}, found {}", var_decl.type_, value_type_name(value)),
                        &format!("{:?}", var_decl.type_),
                        value_type_name(value),
                        var_decl.location.line,
                        var_decl.location.column
                    ));
                }

                set_or_declare_loop_var(env, &idents[0], Value::String(key.clone()));
                set_or_declare_loop_var(env, &idents[1], value.clone());
                evaluate_block_content(body, env)?;
            }
        }
    }
    Ok(None)
}

// Handle for loop iterations over arrays
fn evaluate_for_array(
    arr: Vec<Value>,
    var_decl: &VarDecl,
    body: &[Box<Content>],
    env: &mut Environment
) -> Result<Option<Value>, ZekkenError> {
    let idents: Vec<String> = var_decl
        .ident
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if idents.is_empty() || idents.len() > 2 {
        return Err(ZekkenError::syntax(
            "Array iteration requires one or two identifiers",
            var_decl.location.line,
            var_decl.location.column,
            None,
            None,
        ));
    }

    if idents.len() == 1 {
        set_or_declare_loop_var(env, &idents[0], Value::Void);
    } else {
        set_or_declare_loop_var(env, &idents[0], Value::Int(0));
        set_or_declare_loop_var(env, &idents[1], Value::Void);
    }

    for (index, value) in arr.iter().enumerate() {
        if idents.len() == 1 {
            set_or_declare_loop_var(env, &idents[0], value.clone());
        } else {
            set_or_declare_loop_var(env, &idents[0], Value::Int(index as i64));
            set_or_declare_loop_var(env, &idents[1], value.clone());
        }
        evaluate_block_content(body, env)?;
    }
    Ok(None)
}
