# `oct-lsp-bridge` — Oct LSP server bridge

Wires the Oct grammar into
[`grammar_lsp_bridge::GrammarLanguageBridge`] so VS Code (and any
other LSP editor) can drive an Oct language server.

The Oct counterpart to `twig-lsp-bridge` / `basic-lsp-bridge` /
`nib-lsp-bridge`.

## Configuration in VS Code

```json
{
  "oct.languageServerPath": "/path/to/oct-lsp-server"
}
```

## Versions

- `0.1.0` — initial release; `format_fn: None`.

See [CHANGELOG.md](./CHANGELOG.md) for details.
