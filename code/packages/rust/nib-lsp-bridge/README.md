# `nib-lsp-bridge` — Nib LSP server bridge

Wires the Nib grammar into
[`grammar_lsp_bridge::GrammarLanguageBridge`] so VS Code (and any
other LSP editor) can drive a Nib language server.

The Nib counterpart to `twig-lsp-bridge` and `basic-lsp-bridge`.

## Configuration in VS Code

```json
{
  "nib.languageServerPath": "/path/to/nib-lsp-server"
}
```

## Versions

- `0.1.0` — initial release; `format_fn: None` (formatter wires in
  a follow-up PR).

See [CHANGELOG.md](./CHANGELOG.md) for details.
