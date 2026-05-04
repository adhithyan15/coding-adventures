//! `twig-lsp-server` — Twig Language Server entry point.
//!
//! Runs a JSON-RPC LSP server over stdin/stdout.
//!
//! ## Usage
//!
//! ```text
//! twig-lsp-server
//! ```
//!
//! Configure your editor to launch this binary as the Twig language server.
//! VS Code: set `twig.languageServerPath` in settings.json.
//!
//! ## Implementation (LS02 PR B)
//!
//! 1. Call `twig_lsp_bridge::twig_language_spec()`.
//! 2. Construct `grammar_lsp_bridge::GrammarLanguageBridge::new(spec)`.
//! 3. Call `ls00::serve_stdio(bridge)` — this blocks, running the event loop.
//!
//! TODO (LS02 PR B): Uncomment and implement once LS02 PR A is merged.
//! Verify the exact ls00 serve function name (serve_stdio? run? serve?).
//! File: code/packages/rust/ls00/src/lib.rs

fn main() {
    eprintln!("twig-lsp-server: LS02 PR B not yet implemented.");
    eprintln!("Implement grammar-lsp-bridge (LS02 PR A) first, then wire here.");
    std::process::exit(1);

    // TODO (LS02 PR B): replace stub above with:
    //
    // use grammar_lsp_bridge::GrammarLanguageBridge;
    // use twig_lsp_bridge::twig_language_spec;
    //
    // let bridge = GrammarLanguageBridge::new(twig_language_spec());
    // coding_adventures_ls00::serve_stdio(bridge)
    //     .expect("LSP server error");
}
