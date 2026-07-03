//! `basic-lsp-server` — Dartmouth BASIC LSP entry point.
//!
//! Runs a JSON-RPC LSP server over stdin/stdout.  All editor
//! traffic is handled by `coding_adventures_ls00::server::LspServer`,
//! which delegates every language-specific decision to the
//! `GrammarLanguageBridge` we construct from
//! [`basic_lsp_bridge::basic_language_spec`].

use std::io::{self, BufReader};

use basic_lsp_bridge::basic_language_spec;
use coding_adventures_ls00::language_bridge::LanguageBridge;
use coding_adventures_ls00::server::LspServer;
use grammar_lsp_bridge::GrammarLanguageBridge;

fn main() {
    let bridge = GrammarLanguageBridge::new(basic_language_spec());
    let boxed: Box<dyn LanguageBridge> = Box::new(bridge);
    let stdin  = io::stdin();
    let stdout = io::stdout();
    let reader = BufReader::new(stdin.lock());
    let writer = stdout.lock();
    let mut server = LspServer::new(boxed, reader, writer);
    server.serve();
}
