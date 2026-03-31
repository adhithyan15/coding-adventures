# coding-adventures-lattice-ast-to-css (Lua)

Lattice AST → CSS compiler. Walks a Lattice AST (produced by
`coding-adventures-lattice-parser`) and emits plain CSS text.

## What is Lattice?

Lattice is a CSS superset language. Every valid CSS file is valid Lattice.
Lattice adds variables, mixins, control flow, functions, and nesting.

## How it fits in the stack

```
Lattice Source
     │
     ▼
lattice_lexer   — tokenizes Lattice text
     │
     ▼
lattice_parser  — produces a Lattice AST
     │
     ▼
lattice_ast_to_css   ← you are here
     │
     ▼
CSS text
```

## Usage

```lua
local lattice_parser     = require("coding_adventures.lattice_parser")
local lattice_ast_to_css = require("coding_adventures.lattice_ast_to_css")

local ast = lattice_parser.parse([[
  $primary: #4a90d9;

  @mixin button($bg, $fg: white) {
    background: $bg;
    color: $fg;
    padding: 8px 16px;
  }

  .btn {
    @include button($primary);
  }
]])

local css = lattice_ast_to_css.compile(ast)
-- css:
-- .btn {
--   background: #4a90d9;
--   color: white;
--   padding: 8px 16px;
-- }
```

## API

### `M.compile(ast) → string`

Compile a Lattice AST to CSS text.

- `ast` — ASTNode root returned by `lattice_parser.parse()`
- Returns a CSS string (always ends with `\n`, or `""` for empty input)

## Features

| Feature            | Example                                  |
|--------------------|------------------------------------------|
| Variables          | `$color: red;  h1 { color: $color; }`   |
| Nested rules       | `.parent { .child { color: blue; } }`   |
| Mixins             | `@mixin flex { display: flex; }`         |
| @include           | `.box { @include flex; }`               |
| @if / @else        | `@if $debug { .d { display: block; } }` |
| @for               | `@for $i from 1 through 12 { ... }`     |
| @each              | `@each $c in red, blue { ... }`         |
| @function          | `@function spacing($n) { @return ... }` |
| Selector nesting & | `.nav { &:hover { color: red; } }`      |
