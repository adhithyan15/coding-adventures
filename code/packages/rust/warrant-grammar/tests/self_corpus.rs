//! Authored syntax verdicts and exact subtree witnesses run over L0.
//!
//! The package exports grammar data, not an untrusted-source parse API. This
//! adapter uses the existing shared engine on the small, embedded public corpus.
//! P006b still owns source quotas, original-byte spans and native-stack audits.

use std::collections::HashSet;

use coding_adventures_test_bootstrap::run::{self, Ctx, Suite};
use coding_adventures_test_bootstrap::tfv::{self, Record, Value};
use coding_adventures_test_bootstrap::Verdict;
use coding_adventures_warrant_grammar::{parser_grammar, token_grammar};
use grammar_tools::parser_grammar::{parse_parser_grammar, ParserGrammar};
use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::TokenType;
use parser::grammar_parser::{ASTNodeOrToken, GrammarASTNode, GrammarParser};

const CORPUS: &str = include_str!("../../../../specs/fixtures/warrant-syntax-v1/cases.tfv");
const TOKENS: &str = include_str!("../../../../grammars/warrant/warrant.tokens");
const GRAMMAR: &str = include_str!("../../../../grammars/warrant/warrant.grammar");

#[derive(Clone, Copy)]
enum Fault {
    None,
    AlwaysAccept,
    ForgetWitnesses,
    WrongPrecedence,
}

fn wrong_precedence() -> ParserGrammar {
    // Both operators still parse; multiplication is now looser than addition.
    // The independent subtree witness must detect grouping, not just syntax.
    let changed = GRAMMAR
        .replace("sum = product { ( PLUS | MINUS ) product } ;", "sum = product { STAR product } ;")
        .replace("product = unary { STAR unary } ;", "product = unary { ( PLUS | MINUS ) unary } ;");
    assert_ne!(changed, GRAMMAR, "control must actually mutate the grammar");
    parse_parser_grammar(&changed).expect("controlled grammar must remain well formed")
}

fn parse(source: &str, loaded: bool, fault: Fault) -> Option<GrammarASTNode> {
    let tokens = if loaded {
        parse_token_grammar(TOKENS).expect("canonical token grammar")
    } else {
        token_grammar()
    };
    let grammar = if matches!(fault, Fault::WrongPrecedence) {
        wrong_precedence()
    } else if loaded {
        parse_parser_grammar(GRAMMAR).expect("canonical parser grammar")
    } else {
        parser_grammar()
    };
    let tokens = GrammarLexer::new(source, &tokens).tokenize().ok()?;
    GrammarParser::new(tokens, grammar)
        .with_max_depth(128)
        .parse()
        .ok()
}

fn witnesses(root: &GrammarASTNode) -> HashSet<String> {
    // Walk both trees iteratively. The AST is data; creating a witness must
    // not introduce a second recursive parser or clone a deep tree on stack.
    let mut result = HashSet::new();
    let mut nodes = vec![root];
    while let Some(node) = nodes.pop() {
        let mut text = Vec::new();
        let mut pending: Vec<_> = node.children.iter().rev().collect();
        while let Some(child) = pending.pop() {
            match child {
                ASTNodeOrToken::Node(nested) => pending.extend(nested.children.iter().rev()),
                ASTNodeOrToken::Token(token) => {
                    if !matches!(token.type_, TokenType::Newline | TokenType::Indent | TokenType::Dedent | TokenType::Eof) {
                        text.push(token.value.as_str());
                    }
                }
            }
        }
        result.insert(format!("{}={}", node.rule_name, text.join(" ")));
        nodes.extend(node.children.iter().filter_map(|child| match child {
            ASTNodeOrToken::Node(nested) => Some(nested),
            ASTNodeOrToken::Token(_) => None,
        }));
    }
    result
}

fn check(ctx: &Ctx, record: &Record, fault: Fault) -> Verdict {
    let source = record.field("source").and_then(Value::as_text);
    let expected = match record.field("accept") {
        Some(Value::Bool(value)) => Some(*value),
        _ => None,
    };
    let required = record.field("witnesses").and_then(Value::as_list);
    let (Some(source), Some(expected), Some(required)) = (source, expected, required) else {
        return Verdict::fail("corpus-malformed", "invalid syntax case fields");
    };
    let Some(required): Option<Vec<_>> = required.iter().map(Value::as_text).collect() else {
        return Verdict::fail("corpus-malformed", "invalid subtree witness kind");
    };
    let ast = parse(source, false, fault);
    let accepted = matches!(fault, Fault::AlwaysAccept) || ast.is_some();
    let found = ast.as_ref().map(witnesses).unwrap_or_default();
    let observed = !matches!(fault, Fault::ForgetWitnesses)
        && required.iter().all(|entry| found.contains(*entry));
    let same_artifact = ast == parse(source, true, fault);
    // Report only opaque outcomes. No source, token values or parser exception
    // is copied into TAP if a public case or an internal adapter fails.
    ctx.deep_equal(
        &Value::List(vec![Value::Bool(accepted == expected), Value::Bool(observed), Value::Bool(same_artifact)]),
        &Value::List(vec![Value::Bool(true), Value::Bool(true), Value::Bool(true)]),
    )
}

fn negative(ctx: &Ctx, record: &Record, fault: Fault) -> Verdict {
    if !matches!(check(ctx, record, Fault::None), Verdict::Pass) {
        return Verdict::fail("corpus-mismatch", "negative-control baseline failed");
    }
    match check(ctx, record, fault) {
        Verdict::Fail { .. } => Verdict::Pass,
        Verdict::Pass => Verdict::fail("corpus-mismatch", "broken syntax behavior was accepted"),
    }
}

fn main() {
    let records = match tfv::parse_file(CORPUS) {
        Ok(records) => records,
        Err(_) => {
            eprintln!("invalid embedded syntax transport");
            std::process::exit(1);
        }
    };
    let mut suite = Suite::new();
    for record in &records {
        let record = record.clone();
        suite.add(record.id.clone(), move |ctx| check(ctx, &record, Fault::None));
    }
    for (id, case, fault) in [
        ("adapter/always-accept", "declarations/unknown-block", Fault::AlwaysAccept),
        ("adapter/forget-witnesses", "operators/multiply", Fault::ForgetWitnesses),
        ("adapter/wrong-precedence", "operators/multiply", Fault::WrongPrecedence),
    ] {
        let record = records.iter().find(|r| r.id == case).expect("control case exists").clone();
        suite.add(id, move |ctx| negative(ctx, &record, fault));
    }
    std::process::exit(run::main(&suite));
}
