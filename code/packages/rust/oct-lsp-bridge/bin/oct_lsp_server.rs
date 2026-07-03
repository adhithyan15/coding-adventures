//! `oct-lsp-server` — Oct LSP entry point.

use std::io::{self, BufReader};

use coding_adventures_ls00::language_bridge::LanguageBridge;
use coding_adventures_ls00::server::LspServer;
use grammar_lsp_bridge::GrammarLanguageBridge;
use oct_lsp_bridge::oct_language_spec;

fn main() {
    let bridge = GrammarLanguageBridge::new(oct_language_spec());
    let boxed: Box<dyn LanguageBridge> = Box::new(bridge);
    let stdin  = io::stdin();
    let stdout = io::stdout();
    let reader = BufReader::new(stdin.lock());
    let writer = stdout.lock();
    let mut server = LspServer::new(boxed, reader, writer);
    server.serve();
}
