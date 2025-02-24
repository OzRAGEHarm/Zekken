mod ast;
mod lexer;
mod parser;

fn main() {
    let source = String::from("let const\n func if else then for use include export obj in\n ( ) { } [ ] + - * % / < > . ; : ,\n || | -> => = ! && & == != <= >= ' \"");
    let tokens = lexer::tokenize(source);
    println!("{:#?}", tokens);
}