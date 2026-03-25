# Changelog

All notable changes to this package will be documented in this file.

## [0.2.1] - 2026-03-25

### Fixed

- **Less-than operator `<`** — `compare` now handles `"LESS"` so `@if $val < $min` evaluates correctly.
- **Division operator `/` in expressions** — `eval_lattice_multiplicative` now dispatches on `*` vs `/`. New `divide` method with `LatticeZeroDivisionError` guard mirrors `multiply`.
- **Mixin parameter parsing with multiple defaults** — `extract_params` in `transformer.rb` now accepts `mixin_value_list` nodes in addition to `value_list`.

## [0.2.0] - 2026-03-23

### Added

- **@while loops**: `while_directive` support with max-iteration guard (default
  1000) to prevent infinite loops. `LatticeMaxIterationError` raised when limit
  exceeded.
- **$var in selectors**: VARIABLE tokens in `compound_selector`,
  `simple_selector`, and `class_selector` positions are resolved to their string
  values, enabling dynamic selector generation (e.g., `.col-$i`).
- **@content blocks**: mixins can now accept content blocks via `@include
  mixin-name { ... }`. Inside the mixin body, `@content;` is replaced with the
  caller-provided block, evaluated in the caller's scope.
- **!default flag**: variable declarations with `!default` only set the variable
  if it is not already defined in the scope chain.
- **!global flag**: variable declarations with `!global` set the variable in the
  root (global) scope regardless of nesting depth.
- **Property nesting**: `font: { size: 14px; weight: bold; }` expands to
  `font-size: 14px; font-weight: bold;`. Supports arbitrary nesting depth.
- **@at-root directive**: rules inside `@at-root` are hoisted to the stylesheet
  root level, escaping any nesting context. Both block and inline forms supported.
- **@extend and %placeholder selectors**: `@extend` records extend relationships
  and placeholder-only rules (`%name { ... }`) are removed from CSS output.
- **LatticeMap value type**: ordered key-value map with access via built-in
  functions. `@each $key, $value in $map` destructures map entries.
- **LatticeColor RGB/HSL conversions**: `to_rgb`, `to_hsl`, `color_from_rgb`,
  `color_from_hsl` methods for color manipulation.
- **Built-in functions** (37 total):
  - Map: `map-get`, `map-keys`, `map-values`, `map-has-key`, `map-merge`,
    `map-remove`
  - Color: `lighten`, `darken`, `saturate`, `desaturate`, `adjust-hue`,
    `complement`, `mix`, `rgba`, `red`, `green`, `blue`, `hue`, `saturation`,
    `lightness`
  - List: `nth`, `length`, `join`, `append`, `index`
  - Type: `type-of`, `unit`, `unitless`, `comparable`
  - Math: `math.div`, `math.floor`, `math.ceil`, `math.round`, `math.abs`,
    `math.min`, `math.max`
- **ScopeChain#set_global**: sets a variable in the root scope for `!global`.
- Four new error classes: `LatticeMaxIterationError`,
  `LatticeExtendTargetNotFoundError`, `LatticeRangeError`,
  `LatticeZeroDivisionError`.
- Comprehensive test suite for all v2 features (errors, scope, value types,
  built-in functions, transformer integration).

## [0.1.0] - 2026-03-23

### Added

- `LatticeTransformer#transform(ast)` — three-pass Lattice-to-CSS AST
  transformation: symbol collection, expansion, and cleanup.
- `CSSEmitter#emit(node)` — dispatch-based CSS text generator supporting
  pretty-print (default) and minified (`minified: true`) modes with
  configurable indentation.
- Nine value types (`LatticeNumber`, `LatticeDimension`, `LatticePercentage`,
  `LatticeString`, `LatticeIdent`, `LatticeColor`, `LatticeBool`,
  `LatticeNull`, `LatticeList`) as Ruby `Struct` for structural equality.
- `ScopeChain` for lexically scoped variable lookup with parent traversal.
- `ExpressionEvaluator` for compile-time expression evaluation: arithmetic
  (`+`, `-`, `*`), comparison (`==`, `!=`, `>`, `>=`, `<=`), boolean
  (`and`, `or`), and unary minus.
- `ExpressionEvaluator` accepts an optional `function_resolver:` callback so
  that user-defined Lattice functions inside `@return` expressions are
  evaluated at compile time and circular calls are detected.
- Eight error classes: `LatticeUndefinedVariableError`,
  `LatticeUndefinedMixinError`, `LatticeUndefinedFunctionError`,
  `LatticeWrongArityError`, `LatticeCircularReferenceError`,
  `LatticeMissingReturnError`, `LatticeTypeErrorInExpression`,
  `LatticeUnitMismatchError`.
- `rebuild_node` helper handles both immutable `Data.define` `ASTNode`
  (uses `Data#with`) and mutable `SimpleNode` in the same code path.
- `CSS_FUNCTIONS` constant lists CSS built-in function names; user-defined
  Lattice functions with the same name shadow CSS built-ins.
- Minified mode emits no whitespace or newlines between rules/declarations.
