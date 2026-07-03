# `basic-lsp-bridge` — Dartmouth BASIC LSP server bridge

Wires the Dartmouth BASIC grammar into
[`grammar_lsp_bridge::GrammarLanguageBridge`] so VS Code (and any
other LSP editor) can drive a BASIC language server.

The BASIC counterpart to `twig-lsp-bridge`.  Part of the LSP phase
(task #41) of the multi-language authoring-experience track.

## Architecture

```text
Editor (VS Code / Neovim / …)
    │  LSP / JSON-RPC over stdio
    ▼
basic-lsp-server  (bin/basic_lsp_server.rs)
    │  GrammarLanguageBridge::new(basic_language_spec())
    ▼
grammar-lsp-bridge   ← all 8 LSP features live here
    │
    ▼
lexer + parser       ← runtime tokenisation / parsing
```

## What's wired

- **Semantic tokens** — every KEYWORD / LINE_NUM / NUMBER / STRING /
  NAME / BUILTIN_FN / USER_FN / operator gets an LSP semantic
  token type.
- **Diagnostics** — parser errors surface as red squigglies in
  the editor.
- **Completion** — keyword + built-in-function completion at
  cursor.
- **Hover** — keyword + built-in-function hover documentation.
- **Document symbols** — every BASIC line whose statement is a
  user-defined function (`DEF FNa = …`) is surfaced as a top-level
  symbol.
- **Folding ranges** — multi-statement `FOR … NEXT` and `IF … THEN`
  blocks fold.
- **Goto definition** — jumps from `GOTO 100` / `GOSUB 100` to the
  matching line number.
- **References** — all `GOTO`/`GOSUB` targets of a given line.

## Configuration in VS Code

```json
{
  "basic.languageServerPath": "/path/to/basic-lsp-server"
}
```

## Versions

- `0.1.0` — initial release; `format_fn: None` (formatter wires in
  a follow-up PR).

See [CHANGELOG.md](./CHANGELOG.md) for details.
