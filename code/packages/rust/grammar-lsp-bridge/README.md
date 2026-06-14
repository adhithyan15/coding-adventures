# `grammar-lsp-bridge`

Generic Language Server Protocol bridge driven by `.tokens` and `.grammar` files.

## What it does

`grammar-lsp-bridge` turns a language's token + grammar specification into a
full-featured LSP server with zero per-language boilerplate.  Supply a
`LanguageSpec` — two `include_str!` paths plus a few static slices — and get:

| LSP Feature | How it works |
|---|---|
| Diagnostics | Parse errors from `grammar-tools` |
| Semantic tokens | Token-kind → `LspSemanticTokenType` map |
| Document symbols | Subtrees matching `declaration_rules` |
| Folding ranges | Block-level rules from the grammar |
| Hover | Declaration name + enclosing rule name |
| Completion | Keyword list + in-scope declarations |
| Formatting | Delegates to optional `format_fn` |

## Stack position

```
Editor (VS Code, Neovim, …)
    │  LSP (JSON-RPC over stdio)
    ▼
grammar-lsp-bridge  ←  this crate
    ├── grammar-tools     (lexes + parses via .tokens / .grammar)
    └── ls00              (protocol framing + event loop)
    │  implements ls00::LanguageBridge
    ▼
LanguageSpec (provided per-language, e.g. twig-lsp-bridge)
```

## Usage

```rust
use grammar_lsp_bridge::{GrammarLanguageBridge, LanguageSpec, LspSemanticTokenType};

static SPEC: LanguageSpec = LanguageSpec {
    language_name:    "mylang",
    file_extensions:  &["ml"],
    tokens_source:    include_str!("../../../grammars/mylang.tokens"),
    grammar_source:   include_str!("../../../grammars/mylang.grammar"),
    token_kind_map:   &[
        ("KEYWORD", LspSemanticTokenType::Keyword),
        ("IDENT",   LspSemanticTokenType::Variable),
        ("NUMBER",  LspSemanticTokenType::Number),
        ("STRING",  LspSemanticTokenType::String),
    ],
    declaration_rules: &["function_def", "let_binding"],
    keyword_names:     &["if", "else", "let", "fn"],
    format_fn:         None,
};

fn main() {
    let bridge = GrammarLanguageBridge::new(&SPEC);
    coding_adventures_ls00::serve_stdio(bridge).expect("LSP error");
}
```

## Status — SKELETON (LS02 PR A)

The `LanguageSpec` type and all module stubs are defined and compile cleanly.
Implementation work starts in LS02 PR A.

**Prerequisites before implementing LS02 PR A:**
1. Verify `GrammarASTNode` in `grammar-tools` carries position info
   (`start_line`, `end_line`, `start_col`, `end_col`).  Add if missing.
2. Read `ls00/src/language_bridge.rs` for exact `LanguageBridge` trait
   signatures.

## Crate layout

| File | Purpose |
|---|---|
| `src/spec.rs` | `LanguageSpec`, `LspSemanticTokenType` |
| `src/bridge.rs` | `GrammarLanguageBridge` — implements `ls00::LanguageBridge` |
| `src/tokenize.rs` | Token-stream conversion |
| `src/parse.rs` | Grammar-tools parse → `GrammarASTNode` |
| `src/semantic_tokens.rs` | Token-kind → semantic token mapping |
| `src/symbols.rs` | Declaration table + document symbols |
| `src/folding.rs` | Block-level folding ranges |
| `src/hover.rs` | Hover content from AST context |
| `src/completion.rs` | Keywords + in-scope declaration completions |
| `src/format.rs` | Thin delegation to `spec.format_fn` |

## Spec reference

`code/specs/LS02-grammar-driven-language-server.md`
