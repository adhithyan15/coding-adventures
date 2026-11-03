//! Regenerate both native grammar embeddings from the canonical inputs.
//!
//! `cargo run -p coding_adventures_warrant_grammar --example regenerate`
//! writes them; `-- --check` compares bytes without changing tracked files.
use std::{env, fs, path::PathBuf};

use grammar_tools::compiler::{compile_parser_grammar, compile_token_grammar};
use grammar_tools::cross_validator::cross_validate;
use grammar_tools::parser_grammar::parse_parser_grammar;
use grammar_tools::token_grammar::parse_token_grammar;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = env::args().skip(1).collect();
    let check = match args.as_slice() {
        [] => false,
        [option] if option == "--check" => true,
        _ => return Err("usage: regenerate [--check]".into()),
    };
    let package = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let grammar_dir = package
        .ancestors()
        .nth(3)
        .ok_or("missing code directory")?
        .join("grammars/warrant");
    let tokens = parse_token_grammar(&fs::read_to_string(grammar_dir.join("warrant.tokens"))?)?;
    let grammar = parse_parser_grammar(&fs::read_to_string(grammar_dir.join("warrant.grammar"))?)?;
    if !cross_validate(&tokens, &grammar).is_empty() {
        return Err("canonical grammar cross-validation failed".into());
    }
    for (name, contents) in [
        (
            "_tokens.rs",
            compile_token_grammar(&tokens, "warrant.tokens"),
        ),
        (
            "_parser.rs",
            compile_parser_grammar(&grammar, "warrant.grammar"),
        ),
    ] {
        let target = package.join("src").join(name);
        if check {
            if fs::read(&target)? != contents.as_bytes() {
                return Err(format!("generated grammar drift: {name}").into());
            }
        } else {
            fs::write(target, contents.as_bytes())?;
        }
    }
    println!(
        "Warrant grammar embeddings {}",
        if check { "current" } else { "written" }
    );
    Ok(())
}
