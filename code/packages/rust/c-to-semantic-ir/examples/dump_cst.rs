use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode};
fn dump(n: &GrammarASTNode, d: usize) {
    println!("{}{}", "  ".repeat(d), n.rule_name);
    for c in &n.children {
        match c {
            ASTNodeOrToken::Node(x) => dump(x, d + 1),
            ASTNodeOrToken::Token(t) => println!(
                "{}· {} = {:?}",
                "  ".repeat(d + 1),
                t.effective_type_name(),
                t.value
            ),
        }
    }
}
fn main() {
    let src = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "int main(void) { uint8_t c = 200 + 100; return c; }".to_string());
    dump(&coding_adventures_c_parser::parse_c(&src), 0);
}
