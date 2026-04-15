// AUTO-GENERATED FILE - DO NOT EDIT
// Source family: verilog
// Regenerate with: grammar-tools generate-rust-compiled-grammars verilog
//
// This file embeds versioned ParserGrammar values as native Rust data structures.

use grammar_tools::parser_grammar::ParserGrammar;

pub const SUPPORTED_VERSIONS: &[&str] = &["1995", "2001", "2005"];

pub fn parser_grammar(version: &str) -> Option<ParserGrammar> {
    match version {
        "1995" => Some(_grammar_1995::parser_grammar()),
        "2001" => Some(_grammar_2001::parser_grammar()),
        "2005" => Some(_grammar_2005::parser_grammar()),
        _ => None,
    }
}

#[path = "_grammar_1995.rs"]
mod _grammar_1995;
#[path = "_grammar_2001.rs"]
mod _grammar_2001;
#[path = "_grammar_2005.rs"]
mod _grammar_2005;
