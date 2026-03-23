# lattice-ast-to-css

Three-pass compiler: transforms a Lattice AST (mixed CSS + Lattice nodes) into
a pure CSS AST. The third stage of the Lattice compiler pipeline.

## Architecture

The transformation runs in three passes, mimicking the Python reference
implementation:

```
GrammarASTNode (mixed CSS + Lattice)
        |
        v
┌─────────────────────────────────┐
│  LatticeTransformer (3 passes)  │
│                                 │
│  Pass 1: Symbol Collection      │  ← reads ALL definitions before expansion
│    - collect $variable bindings │
│    - collect @mixin definitions │
│    - collect @function defs     │
│                                 │
│  Pass 2: Expansion              │  ← rewrites the AST
│    - substitute $variables      │
│    - expand @include calls      │
│    - evaluate @if/@for/@each    │
│    - evaluate @function calls   │
│                                 │
│  Pass 3: Cleanup                │  ← remove empty nodes
└────────────────┬────────────────┘
                 |
                 v
        GrammarASTNode (pure CSS)
                 |
                 v
        ┌─────────────┐
        │ CSSEmitter  │
        └──────┬──────┘
               |
               v
           CSS text
```

Pass 1 runs before Pass 2, which is what enables **forward references** —
a mixin can be called before it is defined.

## Modules

| Module          | Purpose                                                      |
|-----------------|--------------------------------------------------------------|
| `errors`        | `LatticeError` enum — semantic errors the transformer emits  |
| `scope`         | `ScopeChain` — lexically-scoped variable bindings            |
| `values`        | `LatticeValue` — runtime values (Number, Dimension, Bool...) |
| `evaluator`     | `ExpressionEvaluator` — evaluates `@if`/`@for` expressions   |
| `transformer`   | `LatticeTransformer` — the three-pass transformation engine  |
| `emitter`       | `CSSEmitter` — pretty-print or minify the CSS AST            |

## Usage

```rust
use coding_adventures_lattice_ast_to_css::{transform_lattice, transform_lattice_minified};

// Pretty-printed CSS (2-space indent)
let css = transform_lattice("$color: red; h1 { color: $color; }").unwrap();

// Minified CSS
let mini = transform_lattice_minified("h1 { color: red; }").unwrap();
// → "h1{color:red;}\n"
```

## Error handling

```rust
use coding_adventures_lattice_ast_to_css::errors::LatticeError;

match transform_lattice("p { color: $missing; }") {
    Err(LatticeError::UndefinedVariable { name, line, column }) => {
        eprintln!("Undefined variable {} at {line}:{column}", name);
    }
    Ok(css) => println!("{css}"),
    Err(e) => eprintln!("Error: {e}"),
}
```

## Supported Lattice features

| Feature              | Example                                            |
|----------------------|----------------------------------------------------|
| Variables            | `$spacing: 8px; padding: $spacing;`                |
| Mixins               | `@mixin flex { display: flex; }` / `@include flex` |
| Mixin parameters     | `@mixin pad($n) { padding: $n; }` / `@include pad(8px)` |
| Forward references   | Use mixin before defining it                       |
| `@if` / `@else`      | `@if $theme == dark { ... } @else { ... }`         |
| `@for` through/to    | `@for $i from 1 through 3 { ... }`                 |
| `@each`              | `@each $c in red, blue { ... }`                    |
| `@function`          | `@function double($n) { @return $n * 2; }`         |
| Arithmetic           | `$n * 2`, `$a + $b`, `16px + 4px`                  |
| Comparisons          | `$x == 1`, `$n >= 10`, `$a != $b`                  |
| Boolean logic        | `$a and $b`, `$a or $b`                             |
| CSS passthrough      | `@media`, `@import`, `linear-gradient(...)`, etc.  |

## Dependencies

- `coding-adventures-lattice-parser` — parses source to AST
- `coding-adventures-lattice-lexer` — tokenizes source (used via parser)
- `grammar-tools`, `parser`, `lexer` — core grammar/AST infrastructure

## Development

```bash
cargo test -p coding-adventures-lattice-ast-to-css
```
