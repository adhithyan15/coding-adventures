//! Regenerate moslayout's embedded parser grammar after editing the canonical
//! `code/grammars/moslayout/moslayout.grammar` source.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

use grammar_tools::compiler::compile_parser_grammar;
use grammar_tools::parser_grammar::parse_parser_grammar;

fn main() {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let grammar_path = manifest.join("../../../grammars/moslayout/moslayout.grammar");
    let source = fs::read_to_string(&grammar_path).expect("read moslayout.grammar");
    let grammar = parse_parser_grammar(&source).expect("parse moslayout.grammar");
    let generated = compile_parser_grammar(&grammar, "moslayout.grammar");

    // Preserve the separately generated token grammar at the top of the
    // combined file and replace only its parser-grammar section.
    let output_path = manifest.join("src/_grammar.rs");
    let existing = fs::read_to_string(&output_path).expect("read src/_grammar.rs");
    let marker = "// ===========================================================================\n// Parser grammar (from moslayout.grammar)\n// ===========================================================================\n";
    let token_section = existing
        .split_once(marker)
        .map(|(prefix, _)| prefix)
        .expect("combined grammar must contain parser marker");
    let parser_body = generated
        .split_once("\npub fn parser_grammar")
        .map(|(_, suffix)| suffix)
        .expect("generated parser grammar must contain its entry point");
    let combined = format!("{token_section}{marker}\npub fn parser_grammar{parser_body}");
    fs::write(&output_path, combined).expect("write src/_grammar.rs");
    let status = Command::new("rustfmt")
        .arg("--edition")
        .arg("2021")
        .arg(&output_path)
        .status()
        .expect("run rustfmt on src/_grammar.rs");
    assert!(status.success(), "rustfmt src/_grammar.rs");
    println!("regenerated src/_grammar.rs parser grammar");
}
