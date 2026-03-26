# lattice-transpiler

End-to-end Lattice source to CSS text pipeline — the public entry point
for the Lattice compiler.

## What it does

Takes a Lattice source string and returns CSS text, running the full
compilation pipeline: tokenize → parse → transform → emit.

## Pipeline

```
Lattice source text
        |
        v
┌─────────────────┐
│  Lattice Lexer  │  ← lattice.tokens grammar
└────────┬────────┘
         │ Vec<Token>
         v
┌─────────────────┐
│  Lattice Parser │  ← lattice.grammar
└────────┬────────┘
         │ GrammarASTNode (mixed CSS + Lattice)
         v
┌─────────────────────────────────┐
│  LatticeTransformer (3 passes)  │
│  Pass 1: Symbol Collection      │
│  Pass 2: Expansion              │
│  Pass 3: Cleanup                │
└────────┬────────────────────────┘
         │ GrammarASTNode (pure CSS)
         v
┌─────────────────┐
│   CSS Emitter   │
└────────┬────────┘
         │
         v
     CSS text
```

## Usage

```rust
use coding_adventures_lattice_transpiler::{transpile_lattice, transpile_lattice_minified};

// Pretty-printed CSS (2-space indent, blank lines between rules)
let css = transpile_lattice(r#"
    $primary: #4a90d9;
    $spacing: 8px;

    @mixin flex-center() {
        display: flex;
        align-items: center;
        justify-content: center;
    }

    .card {
        @include flex-center;
        padding: $spacing;
        color: $primary;
    }
"#).expect("transpile failed");

println!("{css}");
// → .card {
// →   display: flex;
// →   align-items: center;
// →   justify-content: center;
// →   padding: 8px;
// →   color: #4a90d9;
// → }

// Minified (production)
let mini = transpile_lattice_minified("h1 { color: red; font-size: 16px; }").unwrap();
// → "h1{color:red;font-size:16px;}\n"

// Custom indentation
use coding_adventures_lattice_transpiler::transpile_lattice_with_indent;
let tabbed = transpile_lattice_with_indent("h1 { color: red; }", "\t", false).unwrap();
```

## Error types

The transpiler returns `Err(LatticeError)` for semantic errors:

| Error                | Cause                                                   |
|----------------------|---------------------------------------------------------|
| `UndefinedVariable`  | `$var` used but never declared                          |
| `UndefinedMixin`     | `@include name` but `@mixin name` doesn't exist         |
| `UndefinedFunction`  | `name()` used as Lattice function but not defined       |
| `WrongArity`         | Wrong number of arguments to mixin or function          |
| `CircularReference`  | Mixin or function calls itself recursively              |
| `TypeError`          | Arithmetic on incompatible types (e.g. `px + %`)        |
| `MissingReturn`      | `@function` body has no `@return` statement             |

Note: syntax errors currently cause panics (consistent with other parsers
in this repo; will be improved in a future version).

## Dependencies

- `coding-adventures-lattice-ast-to-css` — the actual compilation engine
- `coding-adventures-lattice-parser` — tokenizer + parser
- `coding-adventures-lattice-lexer` — tokenizer

## Development

```bash
cargo test -p coding-adventures-lattice-transpiler
```
