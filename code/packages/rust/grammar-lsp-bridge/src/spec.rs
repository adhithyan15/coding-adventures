//! [`LanguageSpec`] — the per-language configuration struct.
//!
//! A language author creates one `LanguageSpec` (usually as a `static`) and
//! passes it to [`GrammarLanguageBridge::new`].  Everything else is generic.

// ---------------------------------------------------------------------------
// LspSemanticTokenType
// ---------------------------------------------------------------------------

/// The subset of LSP semantic token types surfaced by the generic bridge.
///
/// Maps to the `semanticTokenTypes` capability advertised in `initialize`.
/// The bridge uses these to classify tokens from the `.tokens` grammar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LspSemanticTokenType {
    /// Language keywords (`define`, `if`, `let`, …).
    Keyword,
    /// Operator tokens (`+`, `=`, `->`, …).
    Operator,
    /// Variable / identifier references.
    Variable,
    /// Function names (at declaration sites and call sites).
    Function,
    /// Function parameter names.
    Parameter,
    /// String literals.
    String,
    /// Numeric literals.
    Number,
    /// Comment text.
    Comment,
    /// Type names.
    Type,
    /// Struct/record field names.
    Property,
}

// ---------------------------------------------------------------------------
// LanguageSpec
// ---------------------------------------------------------------------------

/// Everything a language author provides to get a generic LSP server.
///
/// All fields are `'static` — build the spec once (as a `static` or via
/// `OnceLock`) and it lives for the process lifetime.
///
/// ## Minimum viable spec
///
/// ```rust,ignore
/// use grammar_lsp_bridge::{LanguageSpec, LspSemanticTokenType};
///
/// static SPEC: LanguageSpec = LanguageSpec {
///     name: "twig",
///     file_extensions: &["twig"],
///     tokens_source: include_str!("twig.tokens"),
///     grammar_source: include_str!("twig.grammar"),
///     token_kind_map: &[
///         ("KEYWORD", LspSemanticTokenType::Keyword),
///         ("NAME",    LspSemanticTokenType::Variable),
///         ("INTEGER", LspSemanticTokenType::Number),
///     ],
///     declaration_rules: &["define"],
///     keyword_names: &["define", "lambda", "let", "if", "begin"],
///     format_fn: None,
/// };
/// ```
pub struct LanguageSpec {
    // ── Identity ────────────────────────────────────────────────────────────

    /// Human-readable language name (used in logs and error messages).
    pub name: &'static str,

    /// File extensions this server handles.
    ///
    /// Example: `&["twig", "tw"]`.
    pub file_extensions: &'static [&'static str],

    // ── Grammar ─────────────────────────────────────────────────────────────

    /// Full content of the `.tokens` lexical grammar file.
    ///
    /// Use `include_str!("path/to/lang.tokens")` to embed at compile time.
    pub tokens_source: &'static str,

    /// Full content of the `.grammar` syntactic grammar file.
    ///
    /// Use `include_str!("path/to/lang.grammar")` to embed at compile time.
    pub grammar_source: &'static str,

    // ── Token classification ─────────────────────────────────────────────────

    /// Maps `.tokens` token kind names to LSP semantic token types.
    ///
    /// Each entry is `(token_kind_name, lsp_type)`.  Token kinds not in this
    /// slice are silently omitted from the semantic token response (e.g.
    /// punctuation like `LPAREN`, `RPAREN`).
    ///
    /// The kind names must match the uppercase names defined in `.tokens`
    /// (e.g. `"KEYWORD"`, `"NAME"`, `"INTEGER"`).
    pub token_kind_map: &'static [(&'static str, LspSemanticTokenType)],

    // ── Symbol discovery ─────────────────────────────────────────────────────

    /// Names of grammar rules that represent top-level declarations.
    ///
    /// The bridge walks the top-level AST children; any child whose
    /// `rule_name` is in this slice is treated as a declaration.  The first
    /// `NAME` token child of that node becomes the declaration's name.
    ///
    /// Example for Twig: `&["define"]`.
    /// Example for a C-like language: `&["function_decl", "global_var"]`.
    pub declaration_rules: &'static [&'static str],

    /// Reserved word names — used for keyword completions and hover labels.
    ///
    /// Typically the same words listed under `keywords:` in the `.tokens`
    /// file.  Duplicated here so the bridge does not re-parse the tokens
    /// source at runtime.
    pub keyword_names: &'static [&'static str],

    // ── Formatting ───────────────────────────────────────────────────────────

    /// Optional pretty-printer.
    ///
    /// If `Some`, called by the LSP `textDocument/formatting` handler.
    /// Receives the full document source; returns the reformatted source.
    ///
    /// If `None`, the bridge advertises no formatting capability.
    pub format_fn: Option<fn(source: &str) -> Result<String, String>>,

    // ── Phase 2 extension points (reserved; always None for Phase 1) ─────────

    /// Optional symbol-table builder for type-aware hover and completion.
    ///
    /// Phase 2 only.  Leave as `None` for Phase 1; the generic AST-walk
    /// hover + completion will be used instead.
    //
    // NOTE: Uses raw pointer to avoid generic parameter on LanguageSpec.
    // Phase 2 will introduce a proper trait object here.
    pub symbol_table_fn: Option<()>, // placeholder; will be a fn pointer in Phase 2
}

// Manual Debug impl because fn pointers don't implement Debug in all contexts.
impl std::fmt::Debug for LanguageSpec {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LanguageSpec")
            .field("name", &self.name)
            .field("file_extensions", &self.file_extensions)
            .field("declaration_rules", &self.declaration_rules)
            .field("keyword_names", &self.keyword_names)
            .field("format_fn", &self.format_fn.map(|_| "<fn>"))
            .finish()
    }
}
