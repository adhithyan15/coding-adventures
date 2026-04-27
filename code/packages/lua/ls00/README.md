# coding-adventures-ls00

Generic LSP (Language Server Protocol) server framework for Lua. Language-specific "bridges" plug into this framework using plain Lua tables with function fields.

## What is LSP?

The Language Server Protocol is a standardized protocol between code editors (VS Code, Neovim, Emacs) and language servers. It solves the M x N problem: instead of writing M*N integrations for M editors and N languages, each language writes one server and every LSP-aware editor gets all features automatically.

## Architecture

```
Lexer -> Parser -> [Bridge Table] -> [LspServer] -> VS Code / Neovim / Emacs
```

This package is the generic half. It handles:
- JSON-RPC 2.0 transport (via `coding-adventures-json-rpc`)
- Document synchronization (open/change/close)
- Parse caching (avoid re-parsing unchanged documents)
- UTF-16 offset conversion (LSP uses UTF-16 code units)
- Capability advertisement (only advertise what the bridge supports)
- Semantic token encoding (compact delta format)
- All LSP request/notification routing

## Usage

```lua
local ls00 = require("coding_adventures.ls00")

-- 1. Create a bridge table with required + optional functions.
local bridge = {
    tokenize = function(source)
        -- Return array of Token tables.
        return tokens, nil
    end,
    parse = function(source)
        -- Return AST, diagnostics array, error.
        return ast, diagnostics, nil
    end,
    -- Optional: enable hover tooltips.
    hover = function(ast, pos)
        return ls00.HoverResult("**symbol** description")
    end,
}

-- 2. Create and run the server.
local server = ls00.LspServer:new(bridge, io.stdin, io.stdout)
server:serve()  -- blocks until EOF
```

## Bridge API

### Required

| Function | Signature | Description |
|----------|-----------|-------------|
| `tokenize` | `(source) -> tokens, err` | Lex source into token array |
| `parse` | `(source) -> ast, diagnostics, err` | Parse source into AST |

### Optional (each enables one LSP feature)

| Function | Enables |
|----------|---------|
| `hover` | Hover tooltips |
| `definition` | Go to Definition |
| `references` | Find All References |
| `completion` | Autocomplete |
| `rename` | Symbol Rename |
| `semantic_tokens` | Semantic Highlighting |
| `document_symbols` | Document Outline |
| `folding_ranges` | Code Folding |
| `signature_help` | Signature Hints |
| `format` | Document Formatting |

## Dependencies

- `coding-adventures-json-rpc` (JSON-RPC 2.0 transport layer)

## Installation

```bash
luarocks make --local coding-adventures-ls00-0.1.0-1.rockspec
```

## Testing

```bash
cd tests && busted . --verbose --pattern=test_
```
