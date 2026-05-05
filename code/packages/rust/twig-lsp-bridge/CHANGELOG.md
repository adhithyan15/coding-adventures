# Changelog — `twig-lsp-bridge`

## 0.2.0 — 2026-05-04

**LS02 PR B — Twig wiring on top of `grammar-lsp-bridge`.**

The skeleton is now a fully-functional Twig LSP server.  All eight LSP
features (tokenize, parse + diagnostics, semantic tokens, document
symbols, folding ranges, hover, completion, format) are inherited from
`grammar-lsp-bridge` 0.2.0 — this crate just supplies Twig-specific
configuration and the binary entry point.

### What changed

- **`src/lib.rs`** — the static `LanguageSpec` is now real:
  - `tokens_source` / `grammar_source` use `include_str!` against
    `code/grammars/twig.tokens` and `code/grammars/twig.grammar`.
  - `token_kind_map` covers `KEYWORD`, `NAME`, `INTEGER`, `BOOL_TRUE`,
    `BOOL_FALSE`, `QUOTE`, `COLON`, `ARROW`.
  - `declaration_rules = ["define", "module_form"]`.
  - `keyword_names` mirrors the `keywords:` section of `twig.tokens`
    (`define`, `lambda`, `let`, `if`, `begin`, `quote`, `nil`, `module`,
    `export`, `import`).
  - `format_fn = Some(twig_format_wrapper)` — adapts
    `twig_formatter::format` to the `fn(&str) -> Result<String, String>`
    shape required by `LanguageSpec`.

- **`bin/twig_lsp_server.rs`** — real `main()`:
  builds a `GrammarLanguageBridge`, boxes it as `dyn LanguageBridge`,
  hands it to `LspServer::new(boxed, BufReader::new(stdin.lock()),
  stdout.lock())`, and calls `server.serve()`.

- **`Cargo.toml`** — adds `twig-formatter` workspace dependency; version
  bumped `0.1.0 → 0.2.0`.

### Tests

20 unit tests covering:
- spec sanity (name, file extensions, grammar source loaded, declaration
  rules, keyword names, format_fn set)
- bridge construction
- tokenize (smoke, comment/whitespace skipping)
- parse (valid + invalid)
- semantic_tokens classification
- document_symbols (finds `(define foo …)` and `(define bar …)`)
- hover (doesn't crash)
- completion (keywords + user-defined names)
- format (supported + round-trip reparse)
- all `supports_*` capability flags

### Smoke test

The compiled `twig-lsp-server` binary launches cleanly, responds to
malformed input with a proper LSP `-32700 Parse error`, and exits 0 on
EOF.

## 0.1.0 — 2026-05-04

Initial skeleton. Spec, types, and module structure committed.
Implementation stubs in place with detailed inline TODO guides.
See spec `LS02-grammar-driven-language-server.md` / `LS03-dap-adapter-core.md`.
