# LS02 — Grammar-Driven Language Server

> **Depends on**: [`LS00`](LS00-language-server-framework.md) (LSP server
> framework), [`LS01`](LS01-lsp-language-bridge.md) (bridge trait definition),
> [`grammar-format`](grammar-format.md) (`.tokens` + `.grammar` file format),
> [`LANG07`](LANG07-lsp-integration.md) (per-language LSP integration plan)

Any language with a `.tokens` file and a `.grammar` file should get a
fully-functional LSP server **without writing any Rust code**. This spec
describes `grammar-lsp-bridge` — the generic crate that automatically
implements the `LanguageBridge` trait from `ls00` using only those two files
plus a small declarative `LanguageSpec` config — and `twig-lsp-bridge`, the
first instantiation that repositions Twig's existing LSP component crates on
top of it.

---

## Motivation

The existing Twig LSP components (`twig-semantic-tokens`, `twig-hover`,
`twig-completion`, `twig-document-symbols`, `twig-folding-ranges`,
`twig-formatter`) are all implemented against the typed Twig AST. They are
correct and complete, but they are Twig-specific: a new language author cannot
reuse them. The generic `ls00::LanguageBridge` trait defines the interface, but
nothing provides a default implementation.

The insight from the Twig components is that **every one of them uses only the
shape of the AST, never type information or a symbol table**. The `grammar-tools`
crate already produces a generic `GrammarASTNode` tree from any `.grammar` file.
This means the same algorithm that walks the Twig AST can walk any
grammar-tools-produced AST, with two small language-specific inputs:

1. A mapping from token kinds → LSP semantic token types (10–20 lines).
2. The names of grammar rules that represent top-level declarations (1–3
   strings, e.g. `"define"` for Twig).

That is the entire per-language contract for the syntax tier.

---

## Architecture

```
.tokens + .grammar files   (authored by the language designer)
        │
        ▼ grammar-tools build-time codegen (already exists)
GrammarLexer + GrammarParser   (runtime; produce GrammarASTNode trees)
        │
        ▼
LanguageSpec   (small static config; the language author's only deliverable)
  .token_kind_map        — token kind → LspSemanticTokenType
  .declaration_rules     — which grammar rules = top-level bindings
  .keyword_names         — reserved names for completion
  .format_fn             — optional fn(&str) -> String
        │
        ▼
grammar-lsp-bridge::GrammarLanguageBridge   (NEW — this spec)
  implements ls00::LanguageBridge:
    tokenize()           → GrammarLexer run → Token vec
    parse()              → GrammarParser run → GrammarASTNode + Diagnostics
    semantic_tokens()    → token walk + token_kind_map
    document_symbols()   → AST walk on declaration_rules
    folding_ranges()     → multi-line GrammarASTNode detection
    hover()              → identifier at cursor → declaration table lookup
    completion()         → keyword_names + declaration table
    format()             → calls format_fn if provided
        │
        ▼
ls00::LspServer   (already exists)
  JSON-RPC over stdio, document manager, capability advertisement
        │
        ▼
Editor (VS Code, Neovim, Helix, …)
```

A new language gets a complete LSP server by creating one Rust file that
constructs a `LanguageSpec` and passes it to `GrammarLanguageBridge::new()`.

---

## The `LanguageSpec` struct

```rust
/// Everything a language author needs to provide to get an LSP server.
///
/// All fields are `'static` — the spec is constructed once at startup
/// (or as a `OnceLock<LanguageSpec>`) and lives for the process lifetime.
pub struct LanguageSpec {
    /// Human-readable language name (used in error messages and logs).
    pub name: &'static str,

    /// File extensions this server handles (e.g. `&["twig", "tw"]`).
    pub file_extensions: &'static [&'static str],

    /// Contents of the `.tokens` file for this language.
    ///
    /// Typically injected via `include_str!("../path/to/lang.tokens")` at
    /// compile time so the bridge has no file I/O at runtime.
    pub tokens_source: &'static str,

    /// Contents of the `.grammar` file for this language.
    ///
    /// Same pattern: `include_str!("../path/to/lang.grammar")`.
    pub grammar_source: &'static str,

    /// Maps grammar-tools token kinds to LSP semantic token types.
    ///
    /// The `grammar-tools` lexer assigns each token a `TokenKind` string
    /// (the uppercase name from the `.tokens` file, e.g. `"INTEGER"`,
    /// `"NAME"`, `"KEYWORD"`).  This slice maps those strings to the LSP
    /// `SemanticTokenType` enum.
    ///
    /// Unmapped kinds are silently omitted from the semantic token response
    /// (which is correct — punctuation typically has no semantic colour).
    pub token_kind_map: &'static [(&'static str, LspSemanticTokenType)],

    /// Names of grammar rules whose top-level occurrences represent
    /// named bindings (functions or values).
    ///
    /// For Twig: `&["define"]`.
    /// For a C-like language: `&["function_decl", "global_var_decl"]`.
    ///
    /// The bridge walks the top-level AST and, for each node whose
    /// `rule_name` is in this slice, extracts the first `NAME` token child
    /// as the declaration's name.
    pub declaration_rules: &'static [&'static str],

    /// Reserved word names — promoted from NAME tokens to keyword completions.
    ///
    /// Sourced from the `keywords:` section of the `.tokens` file, but
    /// duplicated here so the bridge does not need to re-parse the tokens
    /// source at runtime.
    ///
    /// Example for Twig: `&["define", "lambda", "let", "if", "begin", ...]`.
    pub keyword_names: &'static [&'static str],

    /// Optional pretty-printer.  Called by the `format` LSP handler.
    ///
    /// If `None`, the bridge returns `None` from `format()`, which makes
    /// `ls00` respond with an empty text-edit list (no formatting support).
    ///
    /// The function receives the full document source and returns the
    /// reformatted source.  Errors are surfaced as LSP error responses.
    pub format_fn: Option<fn(source: &str) -> Result<String, String>>,
}
```

### `LspSemanticTokenType` enum

```rust
/// The subset of LSP semantic token types used by the generic bridge.
///
/// Extended in future PRs if languages need additional types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LspSemanticTokenType {
    Keyword,
    Operator,
    Variable,
    Function,
    Parameter,
    String,
    Number,
    Comment,
    Type,
    Property,
}
```

---

## `GrammarLanguageBridge` implementation

The bridge holds a `LanguageSpec` reference and a lazily-initialised
`DeclarationTable` built from the AST on each `parse()` call.

### tokenize

```
tokenize(source: &str) → Result<Vec<ls00::Token>, String>
```

1. Construct a `grammar_tools::GrammarLexer` from `spec.tokens_source`.
2. Run the lexer over `source`.
3. Map each `grammar_tools::LexToken { kind, text, line, column }` to
   `ls00::Token { kind: spec.token_kind_map.get(kind), text, line, column }`.
4. Return the vec.

Lexer errors (unrecognised characters) become individual `ls00::Diagnostic`s
embedded as `Unknown` tokens so the document still renders partially.

### parse

```
parse(source: &str) → Result<(Box<dyn Any + Send + Sync>, Vec<Diagnostic>), String>
```

1. Tokenize (as above).
2. Construct a `grammar_tools::GrammarParser` from `spec.grammar_source`.
3. Run the parser.
4. Collect parser errors → `Vec<ls00::Diagnostic>`.
5. Box the `GrammarASTNode` root as `Box<dyn Any + Send + Sync>` and return
   it alongside the diagnostics.

The boxed type is `Arc<GrammarASTNode>` so it is cheap to clone across
handler invocations.

### semantic_tokens

```
semantic_tokens(source: &str, tokens: &[ls00::Token])
    → Option<Result<Vec<LspSemanticToken>, String>>
```

Walk the token vec, look up each `token.kind` in `spec.token_kind_map`.  If
found, emit an `LspSemanticToken` with the mapped type, the token's position,
and its length.  Skip tokens without a mapping (punctuation, whitespace).

No AST needed — this is a pure token-stream pass.

### document_symbols

```
document_symbols(ast: &dyn Any) → Option<Result<Vec<DocumentSymbol>, String>>
```

Downcast `ast` to `Arc<GrammarASTNode>`.  Walk the top-level children.  For
each child whose `rule_name` is in `spec.declaration_rules`:

1. Extract the first `NAME` token child as the symbol's name.
2. Infer `SymbolKind`:
   - If a subsequent child node is a parameter-list-like rule (`params`,
     `param_list`, `args`, or contains `LPAREN`) → `Function`.
   - Otherwise → `Variable`.
3. Collect the node's start/end lines for the range.

Return the `DocumentSymbol` vec.

### folding_ranges

```
folding_ranges(ast: &dyn Any) → Option<Result<Vec<FoldingRange>, String>>
```

Walk every `GrammarASTNode` recursively.  Emit a `FoldingRange` for any node
that spans more than one line (i.e. `start_line != end_line`).  This handles
all compound forms automatically without per-language configuration.

### hover

```
hover(ast: &dyn Any, pos: Position) → Option<Result<HoverResult, String>>
```

1. Build a `DeclarationTable` from the AST using `declaration_rules` (name →
   (kind, start_line, start_column)).  Cache this on the bridge (keyed by
   document version) to avoid rebuilding on every hover.
2. Walk the AST to find the `NAME` token at `pos`.
3. If the name is in `spec.keyword_names` → return a keyword hover with the
   keyword name.
4. If the name is in the declaration table → return the declaration's name +
   kind + location.
5. Otherwise → return an "unresolved reference" hover (dim marker).

### completion

```
completion(ast: &dyn Any, pos: Position)
    → Option<Result<Vec<CompletionItem>, String>>
```

1. Collect `spec.keyword_names` → `CompletionItem { kind: Keyword, ... }`.
2. Walk AST using `declaration_rules` to collect declared names →
   `CompletionItem { kind: Function | Variable, ... }`.
3. Return sorted: keywords first, then user-defined symbols alphabetically.

No prefix filtering at this layer — `ls00` filters by the prefix typed so far.

### format

```
format(source: &str) → Option<Result<String, String>>
```

If `spec.format_fn` is `None` → `None` (feature not supported).
Otherwise → call `spec.format_fn(source)` and wrap result.

---

## Crate layout

```
code/packages/rust/grammar-lsp-bridge/
├── Cargo.toml              version = "0.1.0"
├── README.md
├── CHANGELOG.md
└── src/
    ├── lib.rs              pub use bridge::*, spec::*, types::*
    ├── spec.rs             LanguageSpec, LspSemanticTokenType
    ├── bridge.rs           GrammarLanguageBridge (LanguageBridge impl)
    ├── tokenize.rs         tokenize() helper
    ├── parse.rs            parse() helper
    ├── semantic_tokens.rs  semantic_tokens() helper
    ├── symbols.rs          document_symbols() + DeclarationTable
    ├── folding.rs          folding_ranges() helper
    ├── hover.rs            hover() helper
    ├── completion.rs       completion() helper
    └── format.rs           format() helper
```

```
code/packages/rust/twig-lsp-bridge/
├── Cargo.toml              version = "0.1.0"
│                           deps: grammar-lsp-bridge, ls00,
│                                 twig-formatter (for format_fn)
├── README.md
├── CHANGELOG.md
├── src/
│   ├── lib.rs              pub fn twig_language_spec() -> &'static LanguageSpec
│   └── spec.rs             builds LanguageSpec from include_str! + token_kind_map
└── bin/
    └── twig-lsp-server.rs  main() → ls00::serve(GrammarLanguageBridge::new(twig_language_spec()))
```

---

## Token kind map for Twig

The `twig-lsp-bridge` spec provides this mapping (derived from `twig.tokens`):

| `.tokens` kind | LSP token type |
|----------------|----------------|
| `KEYWORD`      | `Keyword`      |
| `NAME`         | `Variable`     |
| `INTEGER`      | `Number`       |
| `BOOL_TRUE`    | `Keyword`      |
| `BOOL_FALSE`   | `Keyword`      |
| `LPAREN`       | (unmapped)     |
| `RPAREN`       | (unmapped)     |
| `QUOTE`        | `Operator`     |
| `COLON`        | `Operator`     |
| `ARROW`        | `Operator`     |

Function-position NAME tokens are reclassified to `Function` by the
`document_symbols` + `hover` passes (which know from the declaration table
that the name binds a lambda).  The raw `semantic_tokens` pass uses `Variable`
for all NAMEs; editors apply the `Function` classification from the semantic
token modifier emitted alongside the `document_symbols` result.

---

## PR sequence

### PR LS02-A — `grammar-lsp-bridge` crate

**Scope**: The generic crate only — no Twig wiring.

**Deliverables**:
- `grammar-lsp-bridge` crate with all 8 provider implementations.
- `LanguageSpec` + `LspSemanticTokenType` types.
- `GrammarLanguageBridge` struct implementing `ls00::LanguageBridge`.
- Unit tests: a minimal test grammar (a 3-token toy language) used to verify
  all 8 providers produce correct output.
- Integration test: a slightly richer grammar with declarations, verifying
  `document_symbols`, `hover`, and `completion` are consistent.

**Acceptance criteria**:
- `cargo test -p grammar-lsp-bridge` passes.
- Given a `LanguageSpec` with a toy grammar, the bridge correctly:
  - Returns tokens with mapped kinds.
  - Returns parse errors as `Diagnostic`s.
  - Returns document symbols for declaration rules.
  - Returns folding ranges for multi-line nodes.
  - Returns hover text for a known name.
  - Returns completions containing keywords and declared names.

### PR LS02-B — `twig-lsp-bridge` + server binary

**Scope**: Twig instantiation of the bridge + a runnable LSP server binary.

**Deliverables**:
- `twig-lsp-bridge` crate with `twig_language_spec()` function.
- `twig-lsp-server` binary (stdin/stdout JSON-RPC server).
- Wire `twig-formatter` as the `format_fn`.
- Smoke test: launch the server, send `initialize` + `textDocument/didOpen`
  with a Twig file, verify `publishDiagnostics` response is well-formed.

**Acceptance criteria**:
- `cargo build -p twig-lsp-server` produces a binary.
- The binary responds correctly to `initialize`, `textDocument/didOpen`,
  `textDocument/semanticTokens/full`, `textDocument/documentSymbol`,
  `textDocument/foldingRange`, `textDocument/hover`, `textDocument/completion`,
  `textDocument/formatting`.
- A fully-annotated Twig file produces zero diagnostics and correct symbols.

### PR LS02-C — Reposition existing Twig LSP components (optional)

**Scope**: Audit whether `twig-semantic-tokens`, `twig-hover`, etc. can be
retired in favour of the generic bridge, or whether they should become
fallbacks for the Phase 2 semantic tier (type-aware hover, scope-aware
completion).

**Decision gate**: If the generic bridge's output matches the existing
components on the Twig acceptance suite → deprecate the old components. If
the generic bridge is slightly weaker in hover accuracy → keep the existing
components as optional overrides via `spec.hover_fn`.

---

## Extending to Phase 2 (semantic tier)

The `LanguageSpec` struct reserves extension points for the semantic tier
without breaking the Phase 1 generic API:

```rust
pub struct LanguageSpec {
    // ... Phase 1 fields above ...

    /// Optional symbol-table builder for hover/definition/references.
    ///
    /// If provided, overrides the generic AST-walk hover with a
    /// type-aware implementation.
    pub symbol_table_fn: Option<fn(ast: &GrammarASTNode) -> SymbolTable>,

    /// Optional go-to-definition handler.
    pub definition_fn: Option<fn(ast: &GrammarASTNode, pos: Position)
                                  -> Option<Location>>,
}
```

A language author who wants richer hover or go-to-definition fills in
`symbol_table_fn`.  Languages that leave it `None` get the generic
AST-walk behaviour.

---

## Risk register

| Risk | Likelihood | Impact | Mitigation |
|------|-----------|--------|-----------|
| `grammar-tools` GrammarASTNode does not carry position info | Medium | High | Add `start_line`, `end_line`, `start_col`, `end_col` to `GrammarASTNode` if missing; check before PR LS02-A |
| `ls00::Token` type diverges from `grammar-tools::LexToken` | Low | Medium | Both are under our control; align at PR time |
| Hover accuracy for function-vs-variable distinction is weak | Medium | Low | Acceptable for Phase 1; Phase 2 symbol table overrides it |
| `twig-formatter` returns multi-second latency on large files | Low | Medium | `format_fn` runs on a background thread in `ls00`; already async |
| New grammar adds node types that break `declaration_rules` walk | Low | Low | `declaration_rules` is additive; old rules still match |

---

## Out of scope

- Go-to-definition, find-references, rename (Phase 2 — symbol table required).
- Signature help (Phase 2).
- Inlay hints (future).
- Multi-file / workspace-scope symbol resolution (future).
- VS Code extension packaging (tracked in `LANG07`).
- Type-aware completion (Phase 2).

---

## Acceptance criteria

1. A new language with a `.tokens` + `.grammar` file can get a working LSP
   server by writing ≤ 30 lines of Rust (the `LanguageSpec` instantiation).
2. Twig's LSP server (`twig-lsp-server`) correctly handles all 8 LSP features
   listed above via the generic bridge.
3. The generic bridge passes its own test suite with a toy grammar.
4. The `twig-lsp-bridge` smoke test sends a round-trip request to the server
   binary and receives a well-formed response.
5. No regression in `twig-formatter` output (formatting round-trip is
   idempotent: `format(format(src)) == format(src)`).
