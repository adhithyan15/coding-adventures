//! `regen_grammars` — regenerate `src/_grammar.rs` from the source
//! `ruby.grammar` in `code/grammars/ruby/`.
//!
//! The generated `src/_grammar.rs` is AUTO-GENERATED — do not edit it by
//! hand. After editing `ruby.grammar`, run:
//!
//! ```sh
//! cargo run -p coding-adventures-ruby-parser --bin regen_grammars
//! ```
//!
//! Mirrors `adj-lang`'s `regen_grammars` binary (the same pattern, applied to
//! the Ruby grammar). The Ruby *lexer* is hand-written (not grammar-driven),
//! so only the parser grammar is regenerated here.

use std::fs;
use std::path::PathBuf;

use grammar_tools::compiler::compile_parser_grammar;
use grammar_tools::parser_grammar::parse_parser_grammar;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let grammars = manifest.join("../../../grammars");
    let src = manifest.join("src");

    let gram = fs::read_to_string(grammars.join("ruby").join("ruby.grammar")).expect("read ruby.grammar");
    let pg = parse_parser_grammar(&gram).expect("parse ruby.grammar");
    fs::write(src.join("_grammar.rs"), compile_parser_grammar(&pg, "ruby.grammar")).expect("write _grammar.rs");

    println!("regenerated src/_grammar.rs");
}
