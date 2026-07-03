//! `regen_grammars` — regenerate the adj-lang lexer/parser grammar Rust files
//! from the source `.tokens` / `.grammar` in `code/grammars/`.
//!
//! The generated `src/_lexer_grammar.rs` / `src/_parser_grammar.rs` are
//! AUTO-GENERATED — do not edit them by hand. After editing `adj_lang.tokens`
//! or `adj_lang.grammar`, run:
//!
//! ```sh
//! cargo run -p adj-lang --bin regen_grammars
//! ```
//!
//! This is the codegen entry point the crate previously lacked (the files had
//! been produced by an ad-hoc invocation). It is idempotent on an unchanged
//! grammar.

use std::fs;
use std::path::PathBuf;

use grammar_tools::compiler::{compile_parser_grammar, compile_token_grammar};
use grammar_tools::parser_grammar::parse_parser_grammar;
use grammar_tools::token_grammar::parse_token_grammar;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let grammars = manifest.join("../../../grammars");
    let src = manifest.join("src");

    let toks = fs::read_to_string(grammars.join("adj_lang").join("adj_lang.tokens")).expect("read adj_lang.tokens");
    let tg = parse_token_grammar(&toks).expect("parse adj_lang.tokens");
    fs::write(src.join("_lexer_grammar.rs"), compile_token_grammar(&tg, "adj_lang.tokens"))
        .expect("write _lexer_grammar.rs");

    let gram = fs::read_to_string(grammars.join("adj_lang").join("adj_lang.grammar")).expect("read adj_lang.grammar");
    let pg = parse_parser_grammar(&gram).expect("parse adj_lang.grammar");
    fs::write(src.join("_parser_grammar.rs"), compile_parser_grammar(&pg, "adj_lang.grammar"))
        .expect("write _parser_grammar.rs");

    println!("regenerated src/_lexer_grammar.rs and src/_parser_grammar.rs");
}
