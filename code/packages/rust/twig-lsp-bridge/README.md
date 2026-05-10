# `twig-lsp-bridge`

Twig language LSP implementation built on top of `grammar-lsp-bridge`.

## What it does

Provides the `LanguageSpec` for the Twig language and the `twig-lsp-server`
binary.  All LSP logic lives in `grammar-lsp-bridge` — this crate only
supplies the language-specific constants (token map, keyword list, grammar
file paths) and wires up the binary entry point.

## Stack position

```
Editor (VS Code, Neovim, …)
    │  LSP (JSON-RPC over stdio)
    ▼
twig-lsp-server  ←  binary in this crate
    │
    ▼
grammar-lsp-bridge  (all LSP logic)
    │
    ▼
grammar-tools  (lexes .tokens / .grammar)
```

## Using the language server

Build and place `twig-lsp-server` on your PATH, then configure your editor:

**VS Code** (`.vscode/settings.json`):
```json
{
  "twig.languageServerPath": "/path/to/twig-lsp-server"
}
```

**Neovim** (via `nvim-lspconfig`):
```lua
require('lspconfig').twig.setup({
  cmd = { '/path/to/twig-lsp-server' },
  filetypes = { 'twig', 'tw' },
})
```

## Features provided

- Syntax error diagnostics
- Semantic token highlighting
- Document symbols (functions, let-bindings, module exports)
- Folding ranges
- Hover (declaration info)
- Keyword + declaration completion
- Formatting (via `twig-formatter`)

## Status — LS02 PR B complete (0.2.0)

`twig_language_spec()` is fully wired — grammar files are embedded via
`include_str!`, the formatter is connected, and `twig-lsp-server` is a
real LSP server.  All eight optional `LanguageBridge` features are
supported.

## Spec reference

`code/specs/LS02-grammar-driven-language-server.md`
