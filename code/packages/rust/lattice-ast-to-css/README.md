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
│  Pass 1: Symbol Collection      │  reads ALL definitions before expansion
│    - collect $variable bindings │
│    - collect @mixin definitions │
│    - collect @function defs     │
│    - handle !default/!global    │
│                                 │
│  Pass 2: Expansion              │  rewrites the AST
│    - substitute $variables      │
│    - expand @include calls      │
│    - evaluate @if/@for/@each    │
│    - evaluate @while loops      │  (v2)
│    - evaluate @function calls   │
│    - resolve built-in functions │  (v2)
│    - expand @content blocks     │  (v2)
│    - hoist @at-root rules       │  (v2)
│    - collect @extend targets    │  (v2)
│    - resolve $vars in selectors │  (v2)
│                                 │
│  Pass 3: Cleanup + Post-process │
│    - remove empty nodes         │
│    - apply @extend merging      │  (v2)
│    - splice @at-root hoisted    │  (v2)
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
| `values`        | `LatticeValue` — runtime values (Number, Dimension, Map...)  |
| `evaluator`     | `ExpressionEvaluator` + built-in function dispatch           |
| `transformer`   | `LatticeTransformer` — the three-pass transformation engine  |
| `emitter`       | `CSSEmitter` — pretty-print or minify the CSS AST            |

## Usage

```rust
use coding_adventures_lattice_ast_to_css::{transform_lattice, transform_lattice_minified};

// Pretty-printed CSS (2-space indent)
let css = transform_lattice("$color: red; h1 { color: $color; }").unwrap();

// Minified CSS
let mini = transform_lattice_minified("h1 { color: red; }").unwrap();
// "h1{color:red;}\n"
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

### v1 Features

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

### v2 Features

| Feature                | Example                                              |
|------------------------|------------------------------------------------------|
| `@while` loops         | `@while $i <= 12 { ... $i: $i + 1; }`               |
| Variables in selectors | `.$var { ... }`, `.col-$i { ... }`                   |
| `@content` blocks      | `@mixin wrap { @content; }` / `@include wrap { p {} }` |
| `!default` flag        | `$color: blue !default;`                              |
| `!global` flag         | `$theme: dark !global;`                               |
| Property nesting       | `font: { size: 14px; weight: bold; }`                |
| `@at-root`             | `@at-root .root-level { ... }`                       |
| `@extend`              | `@extend %placeholder;` / `@extend .class;`          |
| `%placeholder`         | `%shared { border: 1px solid; }`                     |
| Maps                   | `$map: (primary: #4a90d9, secondary: #7b68ee);`     |
| Color functions        | `lighten($color, 20%)`, `darken()`, `mix()`, etc.   |
| List functions         | `nth($list, 2)`, `length()`, `join()`, `append()`   |
| Map functions          | `map-get($map, key)`, `map-keys()`, `map-merge()`   |
| Math functions         | `math.div(100px, 2)`, `math.floor()`, `math.round()` |
| Type functions         | `type-of($val)`, `unit($dim)`, `unitless($n)`       |
| `if()` function        | `if($cond, $yes, $no)`                               |

## Dependencies

- `coding-adventures-lattice-parser` — parses source to AST
- `coding-adventures-lattice-lexer` — tokenizes source (used via parser)
- `grammar-tools`, `parser`, `lexer` — core grammar/AST infrastructure

## Development

```bash
cargo test -p coding-adventures-lattice-ast-to-css
```
