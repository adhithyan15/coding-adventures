# @coding-adventures/ls00

Generic Language Server Protocol (LSP) framework for TypeScript.

## What is this?

When you open a source file in VS Code and see red squiggles under syntax errors, autocomplete suggestions, or "Go to Definition" -- none of that is built into the editor. It comes from a *language server*: a separate process that communicates with the editor over the Language Server Protocol.

LSP was invented by Microsoft to solve the M x N problem:

```
M editors x N languages = M x N integrations to write
```

With LSP, each language writes one server, and every LSP-aware editor gets all features automatically. This package is the *generic* half -- it handles all the protocol boilerplate. A language author only writes the `LanguageBridge` that connects their lexer/parser to this framework.

## Architecture

```
Lexer -> Parser -> [LanguageBridge] -> [LspServer] -> VS Code / Neovim / Emacs
```

## How to use

1. Implement the `LanguageBridge` interface (and any optional provider interfaces) for your language.
2. Create an `LspServer` with your bridge and stdio streams.
3. Call `server.serve()` -- it blocks until the editor closes the connection.

```typescript
import { LspServer } from "@coding-adventures/ls00";
import type { LanguageBridge, HoverProvider } from "@coding-adventures/ls00";

// A minimal bridge with just tokenize and parse
const bridge: LanguageBridge = {
  tokenize(source) { return []; },
  parse(source) { return [source, []]; },
};

const server = new LspServer(bridge, process.stdin, process.stdout);
await server.serve();
```

## Optional Providers

The framework uses TypeScript's interface system for optional capability detection. Each feature is a separate interface with a type guard function:

| Interface | Feature | Type Guard |
|-----------|---------|------------|
| `HoverProvider` | Hover tooltips | `isHoverProvider()` |
| `DefinitionProvider` | Go to Definition | `isDefinitionProvider()` |
| `ReferencesProvider` | Find All References | `isReferencesProvider()` |
| `CompletionProvider` | Autocomplete | `isCompletionProvider()` |
| `RenameProvider` | Rename Symbol | `isRenameProvider()` |
| `SemanticTokensProvider` | Semantic Highlighting | `isSemanticTokensProvider()` |
| `DocumentSymbolsProvider` | Document Outline | `isDocumentSymbolsProvider()` |
| `FoldingRangesProvider` | Code Folding | `isFoldingRangesProvider()` |
| `SignatureHelpProvider` | Signature Help | `isSignatureHelpProvider()` |
| `FormatProvider` | Document Formatting | `isFormatProvider()` |

## Dependencies

- `@coding-adventures/json-rpc` -- JSON-RPC 2.0 transport layer

## How it fits in the stack

This package sits at the top of the compiler pipeline:

```
code/packages/typescript/json-rpc    -- transport (JSON-RPC 2.0 over stdio)
code/packages/typescript/ls00        -- THIS PACKAGE (generic LSP framework)
```

Language-specific packages implement `LanguageBridge` and plug into this framework to create a working language server.
