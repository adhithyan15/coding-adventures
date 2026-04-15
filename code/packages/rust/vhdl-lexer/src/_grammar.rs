// AUTO-GENERATED FILE - DO NOT EDIT
// Source family: vhdl
// Regenerate with: grammar-tools generate-rust-compiled-grammars vhdl
//
// This file embeds versioned TokenGrammar values as native Rust data structures.

use grammar_tools::token_grammar::TokenGrammar;

pub const SUPPORTED_VERSIONS: &[&str] = &["1987", "1993", "2002", "2008", "2019"];

pub fn token_grammar(version: &str) -> Option<TokenGrammar> {
    match version {
        "1987" => Some(_grammar_1987::token_grammar()),
        "1993" => Some(_grammar_1993::token_grammar()),
        "2002" => Some(_grammar_2002::token_grammar()),
        "2008" => Some(_grammar_2008::token_grammar()),
        "2019" => Some(_grammar_2019::token_grammar()),
        _ => None,
    }
}

#[path = "_grammar_1987.rs"]
mod _grammar_1987;
#[path = "_grammar_1993.rs"]
mod _grammar_1993;
#[path = "_grammar_2002.rs"]
mod _grammar_2002;
#[path = "_grammar_2008.rs"]
mod _grammar_2008;
#[path = "_grammar_2019.rs"]
mod _grammar_2019;
