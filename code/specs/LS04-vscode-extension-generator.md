# LS04 — VS Code Extension Generator

> **Depends on**: [`LS00`](LS00-language-server-framework.md) (LSP framework),
> [`LS01`](LS01-lsp-language-bridge.md) (per-language LSP bridge),
> [`LS03`](LS03-dap-adapter-core.md) (per-language DAP adapter),
> [`LANG06`](LANG06-debug-integration.md) (debug integration plan),
> [`LANG07`](LANG07-lsp-integration.md) (LSP integration plan).

## Motivation

Once a language built on the LANG-VM pipeline has a working LSP server
(`<lang>-lsp-server` per LS02) **and/or** a DAP server (`<lang>-dap` per
LS03), the only thing standing between the language and a real
authoring experience inside VS Code is a thin **extension** — a few
hundred bytes of `package.json` declaring contributions, plus an
`extension.ts` activation function that launches the appropriate
subprocesses.

Hand-writing this scaffold per language has the same problem the rest
of the repo's scaffold-generators solve: it is mechanical, error-prone,
and the failures are subtle (missing `activationEvents`, malformed
`contributes.debuggers`, wrong main entry, missing `vscode-languageclient`
dependency).

This spec defines a **generator program** that takes language metadata
on the command line and emits a fully wired, CI-ready VS Code extension
package. Any language with a `*-lsp-server` and/or `*-dap` binary gets
editor + debugger integration in one command.

## Goals

1. **Single-command generation.** A language author runs the generator
   once per language. The output is a complete extension package that
   `tsc` compiles, packages with `vsce`, and installs in VS Code.
2. **LSP and DAP wrapped together.** The standard VS Code extension
   pattern (rust-analyzer, Pylance, etc.) ships language intelligence
   and the debugger in one extension. The generator follows that
   convention so users install one thing.
3. **Either-or-both.** If a language only has an LSP server (or only a
   DAP server), the generator emits a working extension with just that
   capability wired. Both is the common case.
4. **Sensible defaults, overridable.** Generated extensions look up
   binaries via PATH by default; users can override via VS Code
   settings (`<lang-id>.serverPath`, `<lang-id>.adapterPath`) — same
   convention as rust-analyzer.
5. **Idempotent.** Re-running with the same inputs produces byte-identical
   output (deterministic file ordering, no timestamps).

## Non-goals

- The generator does **not** package the binaries themselves. The
  extension assumes `<lang>-lsp-server` / `<lang>-dap` are installed
  separately (cargo, brew, prebuilt download script). Bundling binaries
  is a future concern (`LS05-vscode-extension-bundler`).
- The generator does **not** ship rich syntax highlighting. It can emit
  a minimal TextMate grammar from a `--keywords` list, but full syntax
  highlighting belongs to a future `*-textmate-grammar` package per
  language.
- The generator does **not** know about LSP feature flags. It enables
  whatever the LSP server advertises — capability negotiation happens
  at runtime per the LSP spec.

## Inputs

### Spec-driven flow (recommended)

The canonical flow is **spec-driven**: each language ships a small
`<lang>-spec-dump` binary in its parser crate (e.g. `twig-spec-dump`
in `code/packages/rust/twig-parser/bin/twig_spec_dump.rs`).  That
binary pulls the build-time-compiled lexer and parser grammars
straight out of the lexer/parser crates, runs
`grammar_tools::dump_spec::dump_language_spec`, and prints a JSON
document to stdout.

```bash
twig-spec-dump > twig.spec.json
vscode-lang-extension-generator \
  --language-spec twig.spec.json \
  --lsp-binary twig-lsp-server \
  --dap-binary twig-dap \
  --output-dir code/packages/typescript/twig-vscode
```

That's the whole pipeline.  No manual keyword listing, no manual
file-extension declarations, no chance of drift between the lexer's
view of the language and the editor's view.

The runtime data flow is:

```
code/grammars/<lang>.tokens     (build-time-only artifact)
code/grammars/<lang>.grammar    (build-time-only artifact)
        │   include_str! at build time
        ▼
<lang>-lexer/build.rs           (compile_token_grammar → Rust source)
<lang>-parser/build.rs          (compile_parser_grammar → Rust source)
        │   embedded as struct literals
        ▼
<lang>-lexer (rlib)             (TokenGrammar bakes in)
<lang>-parser (rlib)            (ParserGrammar bakes in)
        │   exposed via twig_token_grammar_spec() / twig_grammar()
        ▼
<lang>-spec-dump (binary)       (formats as LanguageSpec JSON)
        │   stdout / file
        ▼
<lang>.spec.json                (build artifact)
        │   --language-spec flag
        ▼
vscode-lang-extension-generator (this generator)
        │   write tree
        ▼
<lang>-vscode/                  (extension package)
```

`.tokens` and `.grammar` source files are **build-time-only**.
Nothing downstream reads them at runtime; the lexer and parser
crates are the runtime source of truth.

### CLI flags

| Flag | Required | Description |
|---|---|---|
| `--language-spec <path>` | spec-driven flow | Path to a JSON spec produced by `<lang>-spec-dump`.  When set, languageId/Name/extensions/keywords/lineComment/blockComment are read from this file. |
| `--language-id <id>` | when no spec | Internal slug (e.g. `twig`). Lowercase, `[a-z0-9-]+`. |
| `--language-name <name>` | when no spec | Display name (e.g. `Twig`). |
| `--file-extensions <list>` | when no spec | Comma-separated, leading dots: `.twig,.tw`. |
| `--output-dir <path>` | yes | Directory to create. Errors if non-empty. |
| `--lsp-binary <name>` | optional* | Name of LSP server binary (e.g. `twig-lsp-server`). |
| `--dap-binary <name>` | optional* | Name of DAP adapter binary (e.g. `twig-dap`). |
| `--line-comment <str>` | optional | Override the spec's lineComment. |
| `--block-comment-start <str>` | optional | Override the spec's blockComment open. |
| `--block-comment-end <str>` | optional | Override the spec's blockComment close. |
| `--keywords <list>` | optional | Override the spec's keywords (comma-separated). |
| `--description <str>` | optional | Extension description; defaults to autogenerated. |
| `--ext-version <semver>` | optional | Extension semver. Defaults to `0.1.0`. |

\* At least one of `--lsp-binary` or `--dap-binary` must be provided.
Otherwise the extension would do nothing and the generator errors.

When `--language-spec` is set, every other CLI flag is an
**optional override**.  Explicit flag values win; otherwise the
spec value is used.

### LanguageSpec JSON schema (v1)

```jsonc
{
  "$schemaVersion": 1,
  "languageId":     "twig",
  "languageName":   "Twig",
  "fileExtensions": ["twig", "tw"],   // no leading dots
  "keywords":         ["define", "if", "..."],
  "reservedKeywords": [],
  "contextKeywords":  [],
  "lineComment":      ";",            // or null
  "blockComment":     null,           // or ["/*", "*/"]
  "brackets":         [["(", ")"]],
  "rules":            ["program", "expression", "..."],
  "declarationRules": ["define", "module_form"],
  "caseSensitive":    true
}
```

Fields are described in `code/packages/rust/grammar-tools/src/dump_spec.rs`.

## Output Layout

```
<output-dir>/
├── package.json
├── tsconfig.json
├── .vscodeignore
├── README.md
├── CHANGELOG.md
├── BUILD
├── BUILD_windows
├── language-configuration.json
├── src/
│   ├── extension.ts                # always
│   ├── lsp.ts                      # iff --lsp-binary
│   └── dap.ts                      # iff --dap-binary
├── syntaxes/
│   └── <language-id>.tmLanguage.json   # iff --keywords
└── examples/
    └── sample<first-extension>     # always
```

### `package.json` skeleton

```jsonc
{
  "name": "<language-id>-vscode",
  "displayName": "<language-name>",
  "description": "<description>",
  "version": "<version>",
  "main": "./out/extension.js",
  "engines": { "vscode": "^1.85.0" },
  "activationEvents": [
    "onLanguage:<language-id>",      // always
    "onDebug"                        // iff DAP wired
  ],
  "contributes": {
    "languages": [{
      "id": "<language-id>",
      "aliases": ["<language-name>"],
      "extensions": [".twig", ".tw"],
      "configuration": "./language-configuration.json"
    }],
    "grammars": [/* iff keywords */],
    "debuggers": [/* iff DAP */],
    "configuration": {
      "title": "<language-name>",
      "properties": {
        "<language-id>.serverPath": { /* iff LSP */ },
        "<language-id>.adapterPath": { /* iff DAP */ }
      }
    }
  },
  "scripts": {
    "build": "tsc -p .",
    "test": "vitest run"
  },
  "dependencies": {
    "vscode-languageclient": "^9.0.1"   // iff LSP
  },
  "devDependencies": {
    "@types/node": "^22.0.0",
    "@types/vscode": "^1.85.0",
    "typescript": "^5.0.0",
    "vitest": "^3.0.0"
  }
}
```

### `extension.ts` skeleton

```ts
import * as vscode from "vscode";
// import only the helpers that match the wired capabilities.
// Both helpers are emitted into the package iff their --*-binary flag was set.

let lspDispose: () => Promise<void> | undefined;

export async function activate(ctx: vscode.ExtensionContext): Promise<void> {
  // LSP block — emitted iff --lsp-binary was provided.
  const { startLanguageClient } = await import("./lsp");
  lspDispose = await startLanguageClient(ctx);

  // DAP block — emitted iff --dap-binary was provided.
  const { registerDebugAdapter } = await import("./dap");
  registerDebugAdapter(ctx);
}

export async function deactivate(): Promise<void> {
  if (lspDispose) await lspDispose();
}
```

(Conditionally-imported pieces are concrete inline imports in the actual
output — the skeleton above is illustrative.)

### `lsp.ts` (emitted iff `--lsp-binary`)

Wraps `vscode-languageclient/node`'s `LanguageClient`. Resolves the
binary via `<lang-id>.serverPath` setting, falling back to PATH.

### `dap.ts` (emitted iff `--dap-binary`)

Registers a `DebugAdapterDescriptorFactory` that returns a
`DebugAdapterExecutable` pointing at the DAP binary, resolved via
`<lang-id>.adapterPath` then PATH.

## Behaviour Contract

The generator MUST:

1. Refuse to write to a non-empty `--output-dir` (overwriting risk).
2. Emit identical files for identical inputs (sort dependency lists,
   sort keys with deterministic stringify).
3. Validate that `--language-id` matches `^[a-z][a-z0-9-]*$`.
4. Validate that each `--file-extensions` entry starts with `.` and
   contains no whitespace.
5. Emit only the files matching the wired capabilities (no orphan
   `lsp.ts` if LSP is not wired).
6. Produce a `package.json` whose `activationEvents`,
   `contributes.debuggers`, and `contributes.configuration.properties`
   match the wired capabilities exactly.

The generator MUST NOT:

1. Run `npm install`. The `BUILD` file does that.
2. Bundle the binaries. The user installs them separately.
3. Emit absolute paths in the generated package.

## Test Strategy

- **Unit tests**: every template builder function covered for both
  presence and absence of optional inputs.
- **Snapshot tests**: golden-file fixtures for representative inputs:
  Twig (LSP+DAP, line-comment `;`, keywords), Nib (DAP-only), and a
  hypothetical Smol (LSP-only, no syntax highlighting).
- **Smoke test**: invoke the generator end-to-end into a temp dir,
  verify the file tree, parse the emitted JSON files, and ensure the
  emitted TypeScript files contain the expected feature blocks.
- **Coverage target**: ≥95% (library quality, see CLAUDE.md).

The generated extension is independently exercised in the demo:
`code/packages/typescript/twig-vscode/` is created by running the
generator with Twig's parameters; its `BUILD` runs `tsc -p .` against
the emitted source.

## Future Work (Not in Scope)

- **`LS05-vscode-extension-bundler`**: ship binaries with the extension
  via post-install script or `vsce`-bundled platform binaries.
- **Marketplace publishing automation**: signed `vsce publish`.
- **Notebook kernel integration** (per `LANG09`).
- **Full TextMate grammar** beyond the generator's keyword-based
  fallback — language-specific package.

## Stack Position

```
                    Language author runs the generator once.
                                    │
                                    ▼
   ┌──────────────────────────────────────────────────────┐
   │   <lang>-vscode/    (this spec's output)             │
   │   ├── activate() wires LSP + DAP                     │
   │   ▼                                                  │
   │  vscode-languageclient   ◀── stdio LSP ──▶ <lang>-lsp-server │
   │  vscode.debug API        ◀── stdio DAP ──▶ <lang>-dap        │
   └──────────────────────────────────────────────────────┘
                                    │
                                    ▼
                         User opens a .<ext> file in VS Code
                         and gets language intelligence + debugger.
```
