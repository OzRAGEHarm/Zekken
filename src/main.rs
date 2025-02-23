mod ast; // Assuming your AST code is in a module named `ast`

use ast::*;

fn main() {
    // Create a constant float variable declaration
    let const_float_decl = VarDecl {
        constant: true,
        ident: "pi".to_string(),
        value: Some(Content::Expression(Box::new(ast::Expr::FloatLit(FloatLit { value: 3.14 })))), // Assigning a float literal
    };

    // Create a non-constant integer variable declaration
    let non_const_int_decl = VarDecl {
        constant: false,
        ident: "count".to_string(),
        value: Some(Content::Expression(Box::new(ast::Expr::IntLit(IntLit { value: 42 })))), // Assigning an integer literal
    };

    // Create the program body with the variable declarations
    let program = Program {
        body: vec![
            Content::Statement(Box::new(ast::Stmt::VarDecl(const_float_decl))), // Add constant float declaration
            Content::Statement(Box::new(ast::Stmt::VarDecl(non_const_int_decl))), // Add non-constant integer declaration
        ],
    };

    // Print the program for debugging
    println!("{:#?}", program);
}