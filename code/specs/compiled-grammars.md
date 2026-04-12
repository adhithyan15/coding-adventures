# Compiled Grammar Files

## Why compile grammars?

All lexer and parser packages currently load their `.tokens` and `.grammar` files at runtime.
A path-walking helper walks up the directory tree until it finds `code/grammars/`, then opens and
parses the file. This works in development but carries three costs:

1. **File I/O at startup** — every process must find and open files on disk.
2. **Parse overhead** — the grammar is re-parsed on every run.
3. **Deployment coupling** — `.tokens` and `.grammar` files must ship alongside programs, and the
   packages are tightly coupled to the repository directory layout.

The `grammar-tools compile-tokens` and `grammar-tools compile-grammar` commands (added in PR 2)
solve this: given a parsed grammar object, they emit language-native source code that embeds the
grammar as data structures — no I/O, no parsing, no directory-walking at runtime.

## The `_grammar.{ext}` files

Each compiled file lives inside the package that uses it, next to the library source:

```
code/packages/{lang}/{name}-lexer/
  src/{pkg}/
    _grammar.{ext}      ← compiled from code/grammars/{name}.tokens

code/packages/{lang}/{name}-parser/
  src/{pkg}/
    _grammar.{ext}      ← compiled from code/grammars/{name}.grammar
```

The underscore prefix (`_grammar`) marks the file as a generated artifact — it must not be edited
by hand. Regenerate it with the Rust `grammar-tools` program whenever the source grammar changes.

### Per-language filenames and exported symbols

| Language   | File           | Exported symbol                                         |
|------------|----------------|---------------------------------------------------------|
| Python     | `_grammar.py`  | `TOKEN_GRAMMAR: TokenGrammar` / `PARSER_GRAMMAR: ...`   |
| Go         | `_grammar.go`  | `func TokenGrammar() *gt.TokenGrammar` / `func ParserGrammar() ...` |
| Ruby       | `_grammar.rb`  | `TOKEN_GRAMMAR` / `PARSER_GRAMMAR` (module constants)   |
| TypeScript | `_grammar.ts`  | `export const TOKEN_GRAMMAR: TokenGrammar` / `PARSER_GRAMMAR` |
| Rust       | `_grammar.rs`  | `pub fn token_grammar() -> TokenGrammar` / `pub fn parser_grammar() ...` |
| Elixir     | `_grammar.ex`  | `def token_grammar/0` / `def parser_grammar/0`          |
| Lua        | `_grammar.lua` | `return { token_grammar = token_grammar }` / `{ parser_grammar = ... }` |

Go is the only language where the package declaration inside the file matters; the generation
script passes `--package {pkgname}` to set it correctly per directory.

## How to regenerate

For Rust packages, run the native generator from the repository root:

```sh
cargo run --release --manifest-path code/programs/rust/grammar-tools/Cargo.toml -- generate-rust-compiled-grammars
```

You can optionally filter to one grammar or package family:

```sh
cargo run --release --manifest-path code/programs/rust/grammar-tools/Cargo.toml -- generate-rust-compiled-grammars sql dartmouth_basic mosaic
```

The Rust command iterates over `code/packages/rust`, finds `*-lexer` and `*-parser` crates with
matching files under `code/grammars`, and calls the appropriate compile step for each. Its mapping
logic now lives in `code/programs/rust/grammar-tools/src/lib.rs`.

The older `scripts/generate-compiled-grammars.sh` script remains as legacy orchestration for the
non-Rust language outputs until those generators are also moved into native programs.

## Grammar coverage

30 grammar files in `code/grammars/` cover 15 grammars/formats:

| Grammar    | .tokens | .grammar | Languages with packages          |
|------------|---------|----------|----------------------------------|
| css        | ✓       | ✓        | Python, Rust                     |
| excel      | ✓       | ✓        | Python, Go, Ruby, TypeScript, Rust |
| javascript | ✓       | ✓        | Python, Go, Ruby, TypeScript, Rust |
| json       | ✓       | ✓        | Python, Go, Ruby, TypeScript, Rust |
| lattice    | ✓       | ✓        | Python, Go, Ruby, TypeScript, Rust |
| lisp       | ✓       | ✓        | Python, Rust                     |
| python     | ✓       | ✓        | Python, Go, Ruby, TypeScript     |
| ruby       | ✓       | ✓        | Python, Go, Ruby, TypeScript, Rust |
| sql        | ✓       | ✓        | Python, Go, Ruby, TypeScript, Rust |
| starlark   | ✓       | ✓        | Python, Go, Ruby, TypeScript, Rust |
| toml       | ✓       | ✓        | Python, Go, Ruby, TypeScript, Rust |
| typescript | ✓       | ✓        | Python, Go, Ruby, TypeScript, Rust |
| verilog    | ✓       | ✓        | Python, Go, Ruby, TypeScript, Rust |
| vhdl       | ✓       | ✓        | Python, Go, Ruby, TypeScript, Rust |
| xml        | ✓       | —        | Python, Go, Ruby, TypeScript, Rust (lexer only) |
| xml_rust   | ✓       | —        | Rust (lexer only)                |

Elixir and Lua do not yet have grammar-dependent packages; those will gain `_grammar.{ext}` files
when the packages are created.

## What comes next (PR 4)

PR 3 commits the generated `_grammar.{ext}` files as checked-in artifacts. PR 4 updates each
package to actually **use** them:

- Remove the path-walking `find_grammars_dir()` helper calls.
- Import / require `_grammar.{ext}` directly instead of opening the `.tokens` / `.grammar` file.
- Remove the grammar-files data dependency from each package's BUILD file.

After PR 4, packages have zero startup I/O for grammar loading and no coupling to the repo layout.
