//! Tokenizer DFA -- formal model of the hand-written lexer's dispatch logic.
//!
//! The hand-written tokenizer in [`super::tokenizer`] has an *implicit* DFA
//! in its main loop: it looks at the current character, classifies it, and
//! dispatches to the appropriate sub-routine. This module makes that implicit
//! DFA *explicit* by defining it as a formal DFA using the state-machine crate.
//!
//! # States
//!
//! ```text
//! State           Description
//! -----------     -----------
//! start           Idle, examining the next character
//! in_number       Reading a sequence of digits
//! in_name         Reading an identifier
//! in_string       Reading a string literal
//! in_operator     Emitting a single-character operator/delimiter
//! in_equals       Handling = with lookahead for ==
//! at_newline      Emitting a NEWLINE token
//! at_whitespace   Skipping whitespace
//! done            End of input
//! error           Unexpected character
//! ```
//!
//! # How the DFA is used
//!
//! The DFA does NOT replace the tokenizer's logic. The sub-routines like
//! `read_number()` and `read_string()` still do the actual work. What the
//! DFA provides is a formal, verifiable model of the dispatch decision.

use std::collections::{HashMap, HashSet};

use state_machine::DFA;

/// Classify a character into one of the DFA's alphabet symbols.
///
/// Maps every possible character to a named class. The DFA's transition
/// table uses these class names to decide what to do next.
///
/// # Character class table
///
/// | Class           | Characters      | Triggers            |
/// |-----------------|-----------------|---------------------|
/// | `eof`           | None (end)      | EOF token           |
/// | `whitespace`    | space/tab/CR    | skip whitespace     |
/// | `newline`       | `\n`            | NEWLINE token       |
/// | `digit`         | `0-9`           | read number         |
/// | `alpha`         | `a-zA-Z`        | read name/keyword   |
/// | `underscore`    | `_`             | read name/keyword   |
/// | `quote`         | `"`             | read string literal |
/// | `equals`        | `=`             | lookahead for ==    |
/// | `operator`      | `+-*/`          | simple operator     |
/// | `open_paren`    | `(`             | LPAREN              |
/// | `close_paren`   | `)`             | RPAREN              |
/// | `comma`         | `,`             | COMMA               |
/// | `colon`         | `:`             | COLON               |
/// | `semicolon`     | `;`             | SEMICOLON           |
/// | `open_brace`    | `{`             | LBRACE              |
/// | `close_brace`   | `}`             | RBRACE              |
/// | `open_bracket`  | `[`             | LBRACKET            |
/// | `close_bracket` | `]`             | RBRACKET            |
/// | `dot`           | `.`             | DOT                 |
/// | `bang`          | `!`             | BANG                 |
/// | `other`         | everything else | error               |
pub fn classify_char(ch: Option<char>) -> &'static str {
    match ch {
        None => "eof",
        Some(c) => match c {
            ' ' | '\t' | '\r' => "whitespace",
            '\n' => "newline",
            '0'..='9' => "digit",
            'a'..='z' | 'A'..='Z' => "alpha",
            '_' => "underscore",
            '"' => "quote",
            '=' => "equals",
            '+' | '-' | '*' | '/' => "operator",
            '(' => "open_paren",
            ')' => "close_paren",
            ',' => "comma",
            ':' => "colon",
            ';' => "semicolon",
            '{' => "open_brace",
            '}' => "close_brace",
            '[' => "open_bracket",
            ']' => "close_bracket",
            '.' => "dot",
            '!' => "bang",
            _ => "other",
        },
    }
}

/// The alphabet of character classes used by the tokenizer DFA.
fn dfa_alphabet() -> HashSet<String> {
    [
        "digit", "alpha", "underscore", "quote", "newline", "whitespace",
        "operator", "equals", "open_paren", "close_paren", "comma", "colon",
        "semicolon", "open_brace", "close_brace", "open_bracket",
        "close_bracket", "dot", "bang", "eof", "other",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// The set of states for the tokenizer DFA.
fn dfa_states() -> HashSet<String> {
    [
        "start", "in_number", "in_name", "in_string",
        "in_operator", "in_equals", "at_newline", "at_whitespace",
        "done", "error",
    ]
    .iter()
    .map(|s| s.to_string())
    .collect()
}

/// Build the full transition map for the tokenizer DFA.
fn build_transitions() -> HashMap<(String, String), String> {
    let alphabet: Vec<String> = dfa_alphabet().into_iter().collect();

    // From "start", each character class goes to a specific handler state.
    let start_dispatch: Vec<(&str, &str)> = vec![
        ("digit", "in_number"),
        ("alpha", "in_name"),
        ("underscore", "in_name"),
        ("quote", "in_string"),
        ("newline", "at_newline"),
        ("whitespace", "at_whitespace"),
        ("operator", "in_operator"),
        ("equals", "in_equals"),
        ("open_paren", "in_operator"),
        ("close_paren", "in_operator"),
        ("comma", "in_operator"),
        ("colon", "in_operator"),
        ("semicolon", "in_operator"),
        ("open_brace", "in_operator"),
        ("close_brace", "in_operator"),
        ("open_bracket", "in_operator"),
        ("close_bracket", "in_operator"),
        ("dot", "in_operator"),
        ("bang", "in_operator"),
        ("eof", "done"),
        ("other", "error"),
    ];

    let mut transitions: HashMap<(String, String), String> = HashMap::new();

    // From "start", dispatch based on character class.
    for (char_class, target) in &start_dispatch {
        transitions.insert(
            ("start".to_string(), char_class.to_string()),
            target.to_string(),
        );
    }

    // All handler states return to "start" on every symbol.
    let handlers = [
        "in_number", "in_name", "in_string", "in_operator",
        "in_equals", "at_newline", "at_whitespace",
    ];
    for handler in &handlers {
        for symbol in &alphabet {
            transitions.insert(
                (handler.to_string(), symbol.clone()),
                "start".to_string(),
            );
        }
    }

    // "done" loops on itself for every symbol.
    for symbol in &alphabet {
        transitions.insert(
            ("done".to_string(), symbol.clone()),
            "done".to_string(),
        );
    }

    // "error" loops on itself for every symbol.
    for symbol in &alphabet {
        transitions.insert(
            ("error".to_string(), symbol.clone()),
            "error".to_string(),
        );
    }

    transitions
}

/// Create a new tokenizer dispatch DFA.
///
/// Each call returns a fresh DFA so callers can process independently.
/// The DFA models the top-level character classification dispatch of the
/// hand-written tokenizer.
///
/// # Example
///
/// ```ignore
/// use lexer::tokenizer_dfa::{new_tokenizer_dfa, classify_char};
///
/// let mut dfa = new_tokenizer_dfa();
/// let char_class = classify_char(Some('5'));
/// let next_state = dfa.process(char_class).unwrap();
/// assert_eq!(next_state, "in_number");
/// ```
pub fn new_tokenizer_dfa() -> DFA {
    DFA::new(
        dfa_states(),
        dfa_alphabet(),
        build_transitions(),
        "start".to_string(),
        HashSet::from(["done".to_string()]),
    )
    .expect("TOKENIZER_DFA construction should never fail")
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // classify_char tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_classify_char_eof() {
        assert_eq!(classify_char(None), "eof");
    }

    #[test]
    fn test_classify_char_digit() {
        assert_eq!(classify_char(Some('0')), "digit");
        assert_eq!(classify_char(Some('9')), "digit");
    }

    #[test]
    fn test_classify_char_alpha() {
        assert_eq!(classify_char(Some('a')), "alpha");
        assert_eq!(classify_char(Some('Z')), "alpha");
    }

    #[test]
    fn test_classify_char_underscore() {
        assert_eq!(classify_char(Some('_')), "underscore");
    }

    #[test]
    fn test_classify_char_whitespace() {
        assert_eq!(classify_char(Some(' ')), "whitespace");
        assert_eq!(classify_char(Some('\t')), "whitespace");
        assert_eq!(classify_char(Some('\r')), "whitespace");
    }

    #[test]
    fn test_classify_char_newline() {
        assert_eq!(classify_char(Some('\n')), "newline");
    }

    #[test]
    fn test_classify_char_quote() {
        assert_eq!(classify_char(Some('"')), "quote");
    }

    #[test]
    fn test_classify_char_equals() {
        assert_eq!(classify_char(Some('=')), "equals");
    }

    #[test]
    fn test_classify_char_operators() {
        assert_eq!(classify_char(Some('+')), "operator");
        assert_eq!(classify_char(Some('-')), "operator");
        assert_eq!(classify_char(Some('*')), "operator");
        assert_eq!(classify_char(Some('/')), "operator");
    }

    #[test]
    fn test_classify_char_delimiters() {
        assert_eq!(classify_char(Some('(')), "open_paren");
        assert_eq!(classify_char(Some(')')), "close_paren");
        assert_eq!(classify_char(Some(',')), "comma");
        assert_eq!(classify_char(Some(':')), "colon");
        assert_eq!(classify_char(Some(';')), "semicolon");
        assert_eq!(classify_char(Some('{')), "open_brace");
        assert_eq!(classify_char(Some('}')), "close_brace");
        assert_eq!(classify_char(Some('[')), "open_bracket");
        assert_eq!(classify_char(Some(']')), "close_bracket");
        assert_eq!(classify_char(Some('.')), "dot");
        assert_eq!(classify_char(Some('!')), "bang");
    }

    #[test]
    fn test_classify_char_other() {
        assert_eq!(classify_char(Some('@')), "other");
        assert_eq!(classify_char(Some('#')), "other");
    }

    // -----------------------------------------------------------------------
    // DFA construction and transition tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_dfa_creation() {
        let dfa = new_tokenizer_dfa();
        assert_eq!(dfa.current_state(), "start");
    }

    #[test]
    fn test_dfa_is_complete() {
        let dfa = new_tokenizer_dfa();
        assert!(dfa.is_complete());
    }

    #[test]
    fn test_dfa_start_to_in_number_on_digit() {
        let mut dfa = new_tokenizer_dfa();
        let next = dfa.process("digit").unwrap();
        assert_eq!(next, "in_number");
    }

    #[test]
    fn test_dfa_start_to_in_name_on_alpha() {
        let mut dfa = new_tokenizer_dfa();
        let next = dfa.process("alpha").unwrap();
        assert_eq!(next, "in_name");
    }

    #[test]
    fn test_dfa_start_to_in_name_on_underscore() {
        let mut dfa = new_tokenizer_dfa();
        let next = dfa.process("underscore").unwrap();
        assert_eq!(next, "in_name");
    }

    #[test]
    fn test_dfa_start_to_done_on_eof() {
        let mut dfa = new_tokenizer_dfa();
        let next = dfa.process("eof").unwrap();
        assert_eq!(next, "done");
    }

    #[test]
    fn test_dfa_start_to_error_on_other() {
        let mut dfa = new_tokenizer_dfa();
        let next = dfa.process("other").unwrap();
        assert_eq!(next, "error");
    }

    #[test]
    fn test_dfa_handler_returns_to_start() {
        let mut dfa = new_tokenizer_dfa();
        dfa.process("digit").unwrap(); // -> in_number
        let next = dfa.process("eof").unwrap(); // -> start (handler returns)
        assert_eq!(next, "start");
    }
}
