//! `src/generated_html1.rs` is compiled from `html1.lexer.states.toml` by
//! `state-machine-source-compiler`. (The skeleton module is kept by hand in
//! rustfmt's shape, so it is not checked here.)
//! This test keeps the two in step: edit the TOML, then regenerate with
//!
//! ```text
//! HTML_LEXER_REGENERATE=1 cargo test -p coding-adventures-html-lexer --test generated_source_test
//! ```
//!
//! and commit both.

use state_machine_markup_deserializer::from_states_toml;
use state_machine_source_compiler::to_rust_source;

fn check(toml: &str, generated_path: &str, generated: &str) {
    let definition = from_states_toml(toml).expect("definition parses");
    let source = to_rust_source(&definition).expect("definition compiles");
    if std::env::var_os("HTML_LEXER_REGENERATE").is_some() {
        std::fs::write(generated_path, &source).expect("write the generated module");
        return;
    }
    assert!(
        source == generated,
        "{generated_path} is stale; regenerate it (see this test's header)"
    );
}

#[test]
fn generated_html1_matches_its_definition() {
    check(
        include_str!("../html1.lexer.states.toml"),
        "src/generated_html1.rs",
        include_str!("../src/generated_html1.rs"),
    );
}
