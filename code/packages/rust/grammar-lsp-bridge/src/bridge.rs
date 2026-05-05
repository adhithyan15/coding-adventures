//! [`GrammarLanguageBridge`] — the generic `ls00::LanguageBridge` implementation.
//!
//! ## Implementation plan (LS02 PR A)
//!
//! 1. Read ls00's LanguageBridge trait — confirm exact method signatures.
//!    File: code/packages/rust/ls00/src/language_bridge.rs
//!
//! 2. Implement each method by delegating to the helper modules:
//!    - tokenize()        → crate::tokenize::run()
//!    - parse()           → crate::parse::run()
//!    - semantic_tokens() → crate::semantic_tokens::run()
//!    - document_symbols()→ crate::symbols::run()
//!    - folding_ranges()  → crate::folding::run()
//!    - hover()           → crate::hover::run()
//!    - completion()      → crate::completion::run()
//!    - format()          → crate::format::run()
//!
//! 3. Cache the DeclarationTable (built from parse()) between calls using a
//!    Mutex<HashMap<document_uri, (version, DeclarationTable)>>.
//!
//! ## Key types from ls00 (verify before implementing):
//!
//!   ls00::Token          { kind: Option<String>, text: String, line: u32, col: u32 }
//!   ls00::Diagnostic     { message: String, range: Range, severity: DiagnosticSeverity }
//!   ls00::Position       { line: u32, character: u32 }
//!   ls00::Range          { start: Position, end: Position }
//!   ls00::DocumentSymbol { name, kind, range, selection_range, children }
//!   ls00::FoldingRange   { start_line, end_line, kind }
//!   ls00::HoverResult    { contents: String, range: Option<Range> }
//!   ls00::CompletionItem { label, kind, detail, documentation }
//!   ls00::SemanticToken  { delta_line, delta_start, length, token_type, token_modifiers }

use crate::LanguageSpec;

/// Generic LSP bridge driven by a [`LanguageSpec`].
///
/// Construct with [`GrammarLanguageBridge::new`] and pass to `ls00::serve()`.
///
/// ## TODO — implement LanguageBridge trait
///
/// The struct compiles today (stub).  LS02 PR A fills in all method bodies.
/// After reading ls00's actual LanguageBridge trait definition, uncomment
/// and implement the `impl ls00::LanguageBridge for GrammarLanguageBridge` block.
pub struct GrammarLanguageBridge {
    pub(crate) spec: &'static LanguageSpec,
}

impl GrammarLanguageBridge {
    /// Construct the bridge for the given language spec.
    pub fn new(spec: &'static LanguageSpec) -> Self {
        GrammarLanguageBridge { spec }
    }

    /// The language spec this bridge was constructed with.
    pub fn spec(&self) -> &'static LanguageSpec {
        self.spec
    }
}

// TODO (LS02 PR A): impl ls00::LanguageBridge for GrammarLanguageBridge {
//     Delegate to crate::tokenize, parse, semantic_tokens, symbols,
//     folding, hover, completion, format modules.
// }
