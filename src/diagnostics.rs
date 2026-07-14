use crate::ast::{Content, Expr, Location, Stmt};
use crate::bytecode;
use crate::environment::{Environment, FunctionValue, Value};
use crate::errors::{
    clear_collected_errors, extract_exit_code, sort_and_dedup_errors, take_collected_errors,
    ZekkenError,
};
use crate::eval::expression::evaluate_expression;
use crate::eval::lint::{collect_lint_expression, collect_lint_statement};
use crate::eval::statement::evaluate_statement;
use crate::lexer::DataType;
use hashbrown::HashMap;
use std::collections::HashSet;
use std::sync::Arc;

#[allow(dead_code)]
#[derive(Clone, Copy)]
pub enum ExecutionMode {
    TreeWalk,
    Bytecode,
}

pub struct RunReport {
    pub value: Option<Value>,
    pub errors: Vec<ZekkenError>,
    #[allow(dead_code)]
    pub exit_code: Option<i32>,
}

fn dummy_value(ty: DataType) -> Value {
    match ty {
        DataType::Int => Value::Int(0),
        DataType::Float => Value::Float(0.0),
        DataType::String => Value::String(String::new()),
        DataType::Bool => Value::Boolean(false),
        DataType::Array => Value::Array(Vec::new()),
        DataType::Object => Value::Object(HashMap::new()),
        DataType::Fn => Value::Function(FunctionValue {
            params: Arc::new(Vec::new()),
            body: Arc::new(Vec::new()),
            return_type: None,
            needs_parent: false,
            captures: Arc::new(Vec::new()),
            capture_values: Arc::new(HashMap::new()),
            compiled_insts: None,
            compiled_reg_count: 0,
        }),
        DataType::Any => Value::Void,
    }
}

fn content_location(content: &Content) -> Location {
    match content {
        Content::Statement(stmt) => match stmt.as_ref() {
            Stmt::Program(node) => node.location.clone(),
            Stmt::VarDecl(node) => node.location.clone(),
            Stmt::FuncDecl(node) => node.location.clone(),
            Stmt::ObjectDecl(node) => node.location.clone(),
            Stmt::IfStmt(node) => node.location.clone(),
            Stmt::ForStmt(node) => node.location.clone(),
            Stmt::WhileStmt(node) => node.location.clone(),
            Stmt::TryCatchStmt(node) => node.location.clone(),
            Stmt::BlockStmt(node) => node.location.clone(),
            Stmt::Use(node) => node.location.clone(),
            Stmt::Include(node) => node.location.clone(),
            Stmt::Export(node) => node.location.clone(),
            Stmt::Return(node) => node.location.clone(),
            Stmt::Lambda(node) => node.location.clone(),
        },
        Content::Expression(expr) => match expr.as_ref() {
            Expr::Assign(node) => node.location.clone(),
            Expr::Member(node) => node.location.clone(),
            Expr::Call(node) => node.location.clone(),
            Expr::Unary(node) => node.location.clone(),
            Expr::Binary(node) => node.location.clone(),
            Expr::Identifier(node) => node.location.clone(),
            Expr::Property(node) => node.location.clone(),
            Expr::IntLit(node) => node.location.clone(),
            Expr::FloatLit(node) => node.location.clone(),
            Expr::StringLit(node) => node.location.clone(),
            Expr::BoolLit(node) => node.location.clone(),
            Expr::ArrayLit(node) => node.location.clone(),
            Expr::ObjectLit(node) => node.location.clone(),
        },
    }
}

fn declare_shape(content: &Content, env: &mut Environment) {
    let stmt = match content {
        Content::Statement(stmt) => stmt.as_ref(),
        Content::Expression(_) => return,
    };

    match stmt {
        Stmt::VarDecl(decl) => env.declare_ref_typed(
            &decl.ident,
            dummy_value(decl.type_),
            decl.type_,
            decl.constant,
        ),
        Stmt::FuncDecl(decl) => env.declare_ref_typed(
            &decl.ident,
            Value::Function(FunctionValue {
                params: Arc::new(decl.params.clone()),
                body: Arc::new(decl.body.clone()),
                return_type: decl.return_type,
                needs_parent: true,
                captures: Arc::new(Vec::new()),
                capture_values: Arc::new(HashMap::new()),
                compiled_insts: None,
                compiled_reg_count: 0,
            }),
            DataType::Fn,
            false,
        ),
        Stmt::Lambda(decl) => env.declare_ref_typed(
            &decl.ident,
            Value::Function(FunctionValue {
                params: Arc::new(decl.params.clone()),
                body: Arc::new(decl.body.clone()),
                return_type: decl.return_type,
                needs_parent: true,
                captures: Arc::new(Vec::new()),
                capture_values: Arc::new(HashMap::new()),
                compiled_insts: None,
                compiled_reg_count: 0,
            }),
            DataType::Fn,
            decl.constant,
        ),
        Stmt::ObjectDecl(decl) => env.declare_ref_typed(
            &decl.ident,
            Value::Object(HashMap::new()),
            DataType::Object,
            false,
        ),
        _ => {}
    }
}

fn execute_content(
    content: &Content,
    env: &mut Environment,
    mode: ExecutionMode,
) -> Result<Option<Value>, ZekkenError> {
    match mode {
        ExecutionMode::Bytecode => {
            let boxed = Box::new(content.clone());
            bytecode::execute_contents(std::slice::from_ref(&boxed), env)
        }
        ExecutionMode::TreeWalk => match content {
            Content::Statement(stmt) => evaluate_statement(stmt, env),
            Content::Expression(expr) => evaluate_expression(expr, env).map(Some),
        },
    }
}

fn append_runtime_result(
    result: Result<Option<Value>, ZekkenError>,
    errors: &mut Vec<ZekkenError>,
    value: &mut Option<Value>,
    exit_code: &mut Option<i32>,
) {
    match result {
        Ok(result) => *value = result,
        Err(error) => {
            if let Some(code) = extract_exit_code(&error.message) {
                *exit_code = Some(code);
            } else {
                errors.push(error);
            }
        }
    }
    errors.extend(take_collected_errors());
}

pub fn run_program_collecting(
    program: &crate::ast::Program,
    syntax_errors: &[ZekkenError],
    env: &mut Environment,
    mode: ExecutionMode,
) -> RunReport {
    clear_collected_errors();
    let mut errors = syntax_errors.to_vec();
    let invalid_lines: HashSet<usize> = syntax_errors
        .iter()
        .map(|error| error.context.line)
        .collect();
    let mut value = None;
    let mut exit_code = None;

    for import in &program.imports {
        append_runtime_result(
            execute_content(import, env, mode),
            &mut errors,
            &mut value,
            &mut exit_code,
        );
        if exit_code.is_some() {
            break;
        }
    }

    let mut analysis_env = env.clone();
    for content in &program.content {
        declare_shape(content, &mut analysis_env);
    }
    for content in &program.content {
        match content.as_ref() {
            Content::Statement(stmt) => collect_lint_statement(stmt, &analysis_env, &mut errors),
            Content::Expression(expr) => collect_lint_expression(expr, &analysis_env, &mut errors),
        }
    }

    if !errors.is_empty() {
        env.declare_ref_typed(
            "println",
            Value::NativeFunction(Arc::new(|_| Ok(Value::Void))),
            DataType::Fn,
            true,
        );
    }

    if exit_code.is_none() {
        for content in &program.content {
            if invalid_lines.contains(&content_location(content).line) {
                declare_shape(content, env);
                continue;
            }

            let result = execute_content(content, env, mode);
            let failed = result.is_err();
            append_runtime_result(result, &mut errors, &mut value, &mut exit_code);
            if failed {
                declare_shape(content, env);
            }
            if exit_code.is_some() {
                break;
            }
        }
    }

    sort_and_dedup_errors(&mut errors);
    RunReport {
        value,
        errors,
        exit_code,
    }
}
