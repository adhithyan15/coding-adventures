# CodingAdventures::Ls00

A generic Language Server Protocol (LSP) framework for Perl.

## What It Does

This package implements the protocol boilerplate for an LSP server. Language authors create a "bridge" object that connects their lexer and parser to this framework. The framework handles:

- JSON-RPC 2.0 transport over stdin/stdout
- Document synchronization (open, change, close, save)
- Capability advertisement based on the bridge's implemented methods
- Semantic token encoding (compact integer format)
- Parse result caching for performance
- UTF-16 offset conversion (LSP uses UTF-16; Perl uses UTF-8)

## Architecture

```
Lexer -> Parser -> [Bridge] -> [LspServer] -> VS Code / Neovim / Emacs
```

## Usage

```perl
use CodingAdventures::Ls00;

# 1. Create a bridge object (any blessed object with tokenize + parse methods)
my $bridge = MyLanguage::Bridge->new();

# 2. Create an LSP server
my $server = CodingAdventures::Ls00::Server->new($bridge, \*STDIN, \*STDOUT);

# 3. Serve (blocks until stdin closes)
$server->serve();
```

## Bridge Interface

A bridge is any Perl object. It must implement two required methods:

- `tokenize($source)` -- returns `(\@tokens, $error)`
- `parse($source)` -- returns `($ast, \@diagnostics, $error)`

Optional methods are detected via Perl's `can()`:

| Method | LSP Feature |
|--------|-------------|
| `hover($ast, $pos)` | Hover tooltips |
| `definition($ast, $pos, $uri)` | Go to Definition |
| `references($ast, $pos, $uri, $include_decl)` | Find References |
| `completion($ast, $pos)` | Autocomplete |
| `rename($ast, $pos, $new_name)` | Symbol Rename |
| `document_symbols($ast)` | Outline Panel |
| `folding_ranges($ast)` | Code Folding |
| `signature_help($ast, $pos)` | Signature Help |
| `format($source)` | Format Document |
| `semantic_tokens($source, \@tokens)` | Semantic Highlighting |

## Dependencies

- `CodingAdventures::JsonRpc` -- JSON-RPC 2.0 transport layer
- `JSON::PP` -- JSON encoding/decoding (core since Perl 5.14)
- `Encode` -- UTF-8/UTF-16 conversion (core module)

## Running Tests

```bash
prove -I../json-rpc/lib -l -v t/
```
