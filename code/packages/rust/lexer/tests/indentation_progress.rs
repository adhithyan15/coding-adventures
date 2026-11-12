use std::process::Command;
use std::thread;
use std::time::{Duration, Instant};

use grammar_tools::token_grammar::parse_token_grammar;
use lexer::grammar_lexer::GrammarLexer;
use lexer::token::TokenType;

const CHILD: &str = "CODING_ADVENTURES_LEXER_HASH_TEST_CHILD";
const INDENTATION_GRAMMAR: &str = "mode: indentation\nNAME = /[a-z]+/\n";

#[test]
fn leading_hash_progress() {
    if std::env::var_os(CHILD).is_some() {
        assert_hash_handling();
        return;
    }

    // Re-execute this one test in a child so a stalled lexer cannot stall
    // the test runner. Only the child created here is terminated on timeout.
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "leading_hash_progress", "--nocapture"])
        .env(CHILD, "1")
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success(), "hash lexer child failed: {status}");
            return;
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            child.wait().unwrap();
            panic!("hash lexer child did not finish in five seconds");
        }
        thread::sleep(Duration::from_millis(10));
    }
}

fn assert_hash_handling() {
    for skip_rules in ["", "skip:\n  COMMENT = /--[^\\n]*/\n"] {
        let grammar = parse_token_grammar(&format!("{INDENTATION_GRAMMAR}{skip_rules}")).unwrap();
        let error = GrammarLexer::new("a\n  #", &grammar)
            .tokenize()
            .unwrap_err();
        assert!(error.message.contains("Unexpected sequence"));
        assert_eq!((error.line, error.column), (2, 3));
    }

    let grammar = parse_token_grammar(&format!("{INDENTATION_GRAMMAR}HASH = \"#\"\n")).unwrap();
    let tokens = GrammarLexer::new("a\n  #\n", &grammar).tokenize().unwrap();
    let hash = &tokens[3];
    assert_eq!(tokens[2].type_, TokenType::Indent);
    assert_eq!(hash.type_name.as_deref(), Some("HASH"));
    assert_eq!(hash.value, "#");
    assert_eq!((hash.line, hash.column), (2, 3));

    let grammar = parse_token_grammar(&format!(
        "{INDENTATION_GRAMMAR}skip:\n  COMMENT = /#[^\\r\\n]*/\n"
    ))
    .unwrap();
    let tokens = GrammarLexer::new("a\n    b\n # comment\r\n    c\n", &grammar)
        .tokenize()
        .unwrap();
    let types: Vec<_> = tokens.iter().map(|token| token.type_).collect();
    assert_eq!(
        types,
        [
            TokenType::Name,
            TokenType::Newline,
            TokenType::Indent,
            TokenType::Name,
            TokenType::Newline,
            TokenType::Name,
            TokenType::Newline,
            TokenType::Dedent,
            TokenType::Eof,
        ]
    );
    assert_eq!((tokens[5].value.as_str(), tokens[5].line), ("c", 4));

    let grammar =
        parse_token_grammar(&format!("{INDENTATION_GRAMMAR}skip:\n  EMPTY = /a*/\n")).unwrap();
    let error = GrammarLexer::new("#", &grammar).tokenize().unwrap_err();
    assert_eq!(error.message, "Zero-width skip match in indentation");
    assert_eq!((error.line, error.column), (1, 1));

    // An empty token match must not turn the fallback into repeated
    // allocations at the same byte.
    let grammar = parse_token_grammar("mode: indentation\nNAME = /[a-z]*/").unwrap();
    let error = GrammarLexer::new("#", &grammar).tokenize().unwrap_err();
    assert_eq!(error.message, "Zero-width token match in indentation");
    assert_eq!((error.line, error.column), (1, 1));
}
