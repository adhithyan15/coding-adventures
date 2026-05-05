//! `twig-lsp-server` — Twig Language Server entry point.
//!
//! Runs a JSON-RPC LSP server over stdin/stdout.  All editor traffic is
//! handled by `coding_adventures_ls00::server::LspServer`, which delegates
//! every language-specific decision to the `GrammarLanguageBridge` we
//! construct from the static `twig_language_spec()`.
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
//! ## How it's wired
//!
//! ```text
//! main()
//!   │
//!   ▼  twig_language_spec()     ← static LanguageSpec for Twig
//!   │
//!   ▼  GrammarLanguageBridge::new(spec)
//!   │
//!   ▼  Box<dyn LanguageBridge>
//!   │
//!   ▼  LspServer::new(boxed_bridge, stdin, stdout)
//!   │
//!   ▼  server.serve()           ← blocks until EOF on stdin
//! ```

use std::io::{self, BufReader};

use coding_adventures_ls00::language_bridge::LanguageBridge;
use coding_adventures_ls00::server::LspServer;
use grammar_lsp_bridge::GrammarLanguageBridge;
use twig_lsp_bridge::twig_language_spec;

fn main() {
    // Build the language-specific bridge from the static Twig spec.
    let bridge = GrammarLanguageBridge::new(twig_language_spec());

    // Erase the concrete type — `LspServer::new` takes
    // `Box<dyn LanguageBridge>`.
    let boxed: Box<dyn LanguageBridge> = Box::new(bridge);

    // Stdio is the LSP standard transport.  We wrap stdin in a `BufReader`
    // because `LspServer` requires `BufRead` for line-oriented framing.
    let stdin  = io::stdin();
    let stdout = io::stdout();

    // Lock both — `LspServer` needs exclusive ownership of the streams
    // for the duration of the session, and locking eliminates the per-call
    // re-locking overhead inside the read loop.
    let reader = BufReader::new(stdin.lock());
    let writer = stdout.lock();

    let mut server = LspServer::new(boxed, reader, writer);

    // Block until the editor closes the connection (EOF).  Errors during
    // serving are logged inside the server, not bubbled up here, so this
    // call is infallible from `main`'s perspective.
    server.serve();
}
