//! # The two grammar pairs, parsed once
//!
//! `code/grammars/vos_contract/*` reads `contract.vos`; `code/grammars/vos_document/*`
//! reads the marker skeleton of a generated document (and, with the same tokens, tells
//! a `.part` whether it contains anything marker-like).  The files are embedded with
//! `include_str!` (the pattern of `vos-shape-check`'s Rust lexer), parsed by
//! `grammar-tools` on first use and cached for the life of the process.

use std::sync::OnceLock;

use grammar_tools::parser_grammar::{parse_parser_grammar, ParserGrammar};
use grammar_tools::token_grammar::{parse_token_grammar, TokenGrammar};

/// `vos_contract.tokens`.
pub const CONTRACT_TOKENS_SRC: &str =
    include_str!("../../../../grammars/vos_contract/vos_contract.tokens");
/// `vos_contract.grammar`.
pub const CONTRACT_GRAMMAR_SRC: &str =
    include_str!("../../../../grammars/vos_contract/vos_contract.grammar");
/// `vos_document.tokens`.
pub const DOCUMENT_TOKENS_SRC: &str =
    include_str!("../../../../grammars/vos_document/vos_document.tokens");
/// `vos_document.grammar`.
pub const DOCUMENT_GRAMMAR_SRC: &str =
    include_str!("../../../../grammars/vos_document/vos_document.grammar");

static CONTRACT_TOKENS: OnceLock<TokenGrammar> = OnceLock::new();
static CONTRACT_GRAMMAR: OnceLock<ParserGrammar> = OnceLock::new();
static DOCUMENT_TOKENS: OnceLock<TokenGrammar> = OnceLock::new();
static DOCUMENT_GRAMMAR: OnceLock<ParserGrammar> = OnceLock::new();

/// The token grammar of `contract.vos`.
pub fn contract_tokens() -> &'static TokenGrammar {
    CONTRACT_TOKENS.get_or_init(|| {
        parse_token_grammar(CONTRACT_TOKENS_SRC).expect(
            "code/grammars/vos_contract/vos_contract.tokens parses (checked by tests/grammars.rs)",
        )
    })
}

/// The parser grammar of `contract.vos`.
pub fn contract_grammar() -> &'static ParserGrammar {
    CONTRACT_GRAMMAR.get_or_init(|| {
        parse_parser_grammar(CONTRACT_GRAMMAR_SRC).expect(
            "code/grammars/vos_contract/vos_contract.grammar parses (checked by tests/grammars.rs)",
        )
    })
}

/// The token grammar of a generated document.
pub fn document_tokens() -> &'static TokenGrammar {
    DOCUMENT_TOKENS.get_or_init(|| {
        parse_token_grammar(DOCUMENT_TOKENS_SRC).expect(
            "code/grammars/vos_document/vos_document.tokens parses (checked by tests/grammars.rs)",
        )
    })
}

/// The parser grammar of a generated document.
pub fn document_grammar() -> &'static ParserGrammar {
    DOCUMENT_GRAMMAR.get_or_init(|| {
        parse_parser_grammar(DOCUMENT_GRAMMAR_SRC).expect(
            "code/grammars/vos_document/vos_document.grammar parses (checked by tests/grammars.rs)",
        )
    })
}

/// The name of a token as the grammars spell it: `NAME`, `DEC`, `LINE`,
/// `CONTRACT_BEGIN`, ...  The engine gives a grammar-defined token its own name when
/// the name is not a built-in kind, and the built-in `NEWLINE` and `EOF` kinds a
/// fixed one.
pub fn token_kind(t: &lexer::token::Token) -> String {
    use lexer::token::TokenType;
    if let Some(name) = &t.type_name {
        return name.to_ascii_uppercase();
    }
    match t.type_ {
        TokenType::Newline => "NEWLINE".to_string(),
        TokenType::Eof => "EOF".to_string(),
        TokenType::Name => "NAME".to_string(),
        other => format!("{other:?}").to_ascii_uppercase(),
    }
}
