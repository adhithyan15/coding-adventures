# ls00 — Generic LSP Framework (Python)

A generic Language Server Protocol (LSP) framework that language-specific "bridges" plug into. This is the Python port of the Go implementation at `code/packages/go/ls00/`.

## What is LSP?

When you open a source file in VS Code and see red squiggles under syntax errors, autocomplete suggestions, or "Go to Definition" -- none of that is built into the editor. It comes from a *language server*: a separate process that communicates with the editor over the Language Server Protocol.

LSP solves the M x N problem: M editors x N languages = M x N integrations. With LSP, each language writes one server, and every LSP-aware editor gets all features automatically.

## Architecture

```
Lexer -> Parser -> [LanguageBridge] -> [LspServer] -> VS Code / Neovim / Emacs
```

This package is the *generic* half -- it handles all the protocol boilerplate. A language author only writes the `LanguageBridge` that connects their lexer/parser to this framework.

## How It Works

The framework uses Python's `typing.Protocol` with `@runtime_checkable` to implement capability detection. A bridge class simply defines the methods it supports -- no explicit registration needed.

### Required Interface

Every bridge must implement `tokenize()` and `parse()`:

```python
class MyBridge:
    def tokenize(self, source: str) -> list[Token]:
        # Your lexer here
        return []

    def parse(self, source: str) -> tuple[Any, list[Diagnostic]]:
        # Your parser here
        return (ast, diagnostics)
```

### Optional Features

Add more methods to enable more LSP features:

```python
class MyBridge:
    # ... required methods ...

    def hover(self, ast, pos):
        """Enables hover tooltips."""
        return HoverResult(contents="**int** variable")

    def definition(self, ast, pos, uri):
        """Enables Go to Definition (F12)."""
        return Location(uri=uri, range=decl_range)

    def completion(self, ast, pos):
        """Enables autocomplete."""
        return [CompletionItem(label="myVar")]
```

The server automatically detects which protocols the bridge implements and advertises only those capabilities to the editor.

## Quick Start

```python
import sys
from ls00 import LspServer, Token, Diagnostic

class MyBridge:
    def tokenize(self, source):
        return []

    def parse(self, source):
        return (None, [])

server = LspServer(MyBridge(), sys.stdin.buffer, sys.stdout.buffer)
server.serve()  # blocks until editor disconnects
```

## Dependencies

- Python >= 3.11
- `json-rpc` (internal package at `../json-rpc`)
- No external dependencies (stdlib only)

## Package Structure

```
src/ls00/
  __init__.py          # Public API exports
  types.py             # All LSP types as dataclasses
  language_bridge.py   # Protocol classes for bridge + optional providers
  document_manager.py  # Document tracking + UTF-16 conversion
  parse_cache.py       # (uri, version)-keyed parse result cache
  capabilities.py      # Dynamic capability building + semantic token encoding
  lsp_errors.py        # LSP-specific error code constants
  server.py            # LspServer coordinator class
  handlers.py          # All LSP request/notification handlers
```

## Testing

```bash
cd code/packages/python/ls00
uv venv .venv
uv pip install -e ".[dev]"
python -m pytest tests/ -v --cov=ls00
```
