# Changelog

## [0.2.1] - 2026-03-25

### Fixed

- **Less-than operator `<`** — `_compare` now handles `"LESS"` so `@if $val < $min` evaluates correctly.
- **Division operator `/` in expressions** — `_eval_lattice_multiplicative` now dispatches on `*` vs `/`. New `_divide` method with zero-division guard mirrors `_multiply`.
- **Mixin parameter parsing with multiple defaults** — `_extract_params` in `transformer.py` now accepts `mixin_value_list` nodes in addition to `value_list`.

## [0.2.0] - 2026-03-23

### Added — Lattice v2 Features

#### Tier 1: Grammar + Transformer

- **@while loops**: condition-based iteration with configurable max-iteration
  guard (default 1000). Raises `MaxIterationError` on runaway loops.
- **$var in selectors**: VARIABLE tokens in compound_selector, simple_selector,
  and class_selector positions are resolved to their string values during
  transformation. Adjacent tokens concatenate (`.col-` + `3` → `.col-3`).
- **@content blocks**: inside a mixin body, `@content;` is replaced with the
  content block passed to the `@include` call. Content is evaluated in the
  caller's scope, not the mixin's scope.
- **!default flag**: `$var: value !default;` only sets the variable if it is
  not already defined anywhere in the scope chain.
- **!global flag**: `$var: value !global;` sets the variable in the root
  (global) scope regardless of nesting depth. Combined `!default !global`
  checks the global scope before setting.
- **Property nesting**: `font: { size: 14px; weight: bold; }` flattens to
  `font-size: 14px; font-weight: bold;`. Supports arbitrary nesting depth.
- **@at-root**: hoists rules to the stylesheet root level, escaping nesting
  context. Supports both block form and inline selector form.
- **@extend and %placeholder selectors**: `@extend .target;` appends the
  current rule's selector to the target rule's selector list. Placeholder
  selectors (`%name`) are removed from output after extension.

#### Tier 2: New Value Types + Built-in Functions

- **LatticeMap**: frozen dataclass for ordered key-value maps. Supports
  get, keys, values, has_key operations. Truthy even when empty.
- **Map functions**: `map-get`, `map-keys`, `map-values`, `map-has-key`,
  `map-merge`, `map-remove` — 6 functions for map manipulation.
- **Color functions**: `lighten`, `darken`, `saturate`, `desaturate`,
  `adjust-hue`, `complement`, `mix`, `rgba`, `red`, `green`, `blue`,
  `hue`, `saturation`, `lightness` — 14 functions for color manipulation.
  Includes hex↔RGB and RGB↔HSL conversion helpers on LatticeColor.
- **List functions**: `nth`, `length`, `join`, `append`, `index` — 5
  functions for list manipulation.
- **Type functions**: `type-of`, `unit`, `unitless`, `comparable` — 4
  functions for type introspection.
- **Math functions**: `math.div`, `math.floor`, `math.ceil`, `math.round`,
  `math.abs`, `math.min`, `math.max` — 7 functions for numeric operations.
- **@each over maps**: `@each $key, $value in $map` destructures map entries.
- **Built-in function registry**: 37 built-in functions registered in
  `BUILTIN_FUNCTIONS` dictionary. User-defined functions shadow built-ins.

#### New Error Types

- `MaxIterationError` — @while loop exceeded iteration limit
- `ExtendTargetNotFoundError` — @extend target selector not in stylesheet
- `RangeError` — value out of valid range (nth index, color amounts)
- `ZeroDivisionInExpressionError` — math.div() with zero divisor

#### Scope Enhancement

- `ScopeChain.set_global()` — set variable in root scope from any depth

### Changed

- `LatticeTransformer.__init__` now accepts `max_while_iterations` parameter
- `LatticeTransformer.transform` runs @extend selector merging and @at-root
  hoisting as post-processing steps in Pass 3
- Expression evaluator recognizes `LatticeMap` in variable lookups
- `LatticeValue` type alias includes `LatticeMap`
- `CSS_FUNCTIONS` frozenset interaction with built-in function registry:
  user-defined functions take priority, then Lattice built-ins, then CSS passthrough

## [0.1.0] - 2026-03-22

### Added

- Three-pass AST transformer: symbol collection, expansion, cleanup
- Lexical scope chain with parent-chain variable lookup and shadowing
- Compile-time expression evaluator supporting 9 value types
  (Number, Dimension, Percentage, String, Ident, Color, Bool, Null, List)
- Arithmetic on numbers, dimensions (same-unit), and percentages
- Comparison operators: `==`, `!=`, `>`, `>=`, `<=`
- Logical operators: `and`, `or`
- Variable expansion with `$name` references
- Mixin definitions (`@mixin`) and expansion (`@include`) with parameters
- Function definitions (`@function`) and evaluation with `@return`
- Control flow: `@if`/`@else`, `@for` loops, `@each` iteration
- Cycle detection for circular mixin/function references
- CSS emitter with pretty-print and minified output modes
- 10 structured error types with line/column position info
- All errors inherit from `LatticeError` for unified error handling
