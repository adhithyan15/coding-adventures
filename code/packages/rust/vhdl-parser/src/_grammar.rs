// AUTO-GENERATED FILE - DO NOT EDIT
// Source family: vhdl
// Regenerate with: grammar-tools generate-rust-compiled-grammars vhdl
//
// This file embeds versioned ParserGrammar values as native Rust data structures.

use grammar_tools::parser_grammar::ParserGrammar;

pub const SUPPORTED_VERSIONS: &[&str] = &["1987", "1993", "2002", "2008", "2019"];

pub fn parser_grammar(version: &str) -> Option<ParserGrammar> {
    match version {
        "1987" => Some(_grammar_1987::parser_grammar()),
        "1993" => Some(_grammar_1993::parser_grammar()),
        "2002" => Some(_grammar_2002::parser_grammar()),
        "2008" => Some(_grammar_2008::parser_grammar()),
        "2019" => Some(_grammar_2019::parser_grammar()),
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
