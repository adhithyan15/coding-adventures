# Changelog

All notable changes to this package will be documented in this file.

## [0.2.1] - 2026-03-25

### Fixed

- **Less-than operator `<`** — `compare_nums` now handles `"LESS"` so `@if $val < $min` evaluates correctly.
- **Division operator `/` in expressions** — `eval_multiplicative` now dispatches on `*` vs `/`. New `divide` method with `LatticeError::ZeroDivision` guard mirrors `multiply`.
- **Mixin parameter parsing with multiple defaults** — `extract_params` in `transformer.rs` now accepts `mixin_value_list` AST nodes in addition to `value_list`.

## [0.2.0] - 2026-03-23

### Added — Lattice v2 Features

- **@while loops**: `@while $i <= 12 { ... }` with max-iteration guard (1000 iterations, configurable). Raises `MaxIteration` error on infinite loops.
- **Variables in selectors**: `.$variable { ... }`, `$tag { ... }`, `.prefix-$var { ... }` — variables in selector positions are resolved and concatenated with adjacent tokens.
- **@content blocks**: Mixins can accept content blocks via `@include mixin { ... }`. Inside the mixin, `@content;` is replaced with the caller's block, evaluated in the caller's scope.
- **!default flag**: `$var: value !default;` — only sets the variable if it is not already defined anywhere in the scope chain.
- **!global flag**: `$var: value !global;` — sets the variable in the root (global) scope, regardless of nesting depth. Can be combined with `!default`.
- **Property nesting**: `font: { size: 14px; weight: bold; }` expands to `font-size: 14px; font-weight: bold;` (infrastructure ready, dispatched when parser produces `property_nesting` nodes).
- **@at-root**: Emits rules at the stylesheet root level, escaping nesting context. Both block form `@at-root { ... }` and inline form `@at-root .selector { ... }`.
- **@extend and %placeholder selectors**: `@extend .target;` appends the current rule's selector to the target's selector list. `%placeholder` selectors are supported and removed from output.
- **Maps**: `LatticeValue::Map` — ordered key-value stores written as `(key: value, ...)`. Access via built-in functions: `map-get`, `map-keys`, `map-values`, `map-has-key`, `map-merge`, `map-remove`.
- **Built-in color functions**: `lighten`, `darken`, `saturate`, `desaturate`, `adjust-hue`, `complement`, `mix`, `red`, `green`, `blue`, `hue`, `saturation`, `lightness` — all operate on hex color values with HSL-based adjustments.
- **Built-in list functions**: `nth`, `length`, `join`, `append`, `index`.
- **Built-in math functions**: `math.div`, `math.floor`, `math.ceil`, `math.round`, `math.abs`, `math.min`, `math.max`.
- **Built-in type functions**: `type-of`, `unit`, `unitless`, `comparable`.
- **Built-in `if()` function**: `if($condition, $if-true, $if-false)`.
- **New error types**: `MaxIteration`, `ExtendTargetNotFound`, `Range`, `ZeroDivision` — all with line/column position info.
- **ScopeChain.set_global()**: Sets a variable in the root scope for `!global` support.
- **Color conversion helpers**: `hex_to_rgba`, `rgb_to_hsl`, `hsl_to_rgb`, `rgba_to_hex` in values.rs.
- 40+ new tests covering all v2 features (120 total tests, up from 80).

## [0.1.0] - 2026-03-23

### Added

- Initial implementation of the three-pass Lattice-to-CSS compiler
- `transform_lattice(source: &str) -> Result<String, LatticeError>` — main entry point
- `transform_lattice_minified(source: &str) -> Result<String, LatticeError>` — minified output
- `transform_lattice_with_options(source, indent, minified)` — full control over output format
- `transform_ast_to_css(ast, indent, minified)` — transform a pre-parsed AST
- **errors.rs**: `LatticeError` enum with 10 variants: `Return`, `ReturnOutsideFunction`, `UndefinedVariable`, `UndefinedMixin`, `UndefinedFunction`, `WrongArity`, `CircularReference`, `TypeError`, `UnitMismatch`, `MissingReturn`
- **scope.rs**: `ScopeChain` with lexical scoping via `Option<Box<ScopeChain>>` parent links; `ScopeValue` enum with `Evaluated(LatticeValue)` and `Raw(String)` variants
- **values.rs**: `LatticeValue` enum with 9 variants: `Number`, `Dimension`, `Percentage`, `String`, `Ident`, `Color`, `Bool`, `Null`, `List`; `token_to_value()` converter
- **evaluator.rs**: `ExpressionEvaluator` with full operator precedence (`or`, `and`, comparison, additive, multiplicative, unary); variable lookup; arithmetic on compatible types
- **transformer.rs**: Three-pass `LatticeTransformer`: Pass 1 collects symbols (variables, mixins, functions) enabling forward references; Pass 2 expands the AST (substitutes variables, inlines mixins, evaluates control flow, calls functions); Pass 3 cleans up empty nodes
- **emitter.rs**: `CSSEmitter` with pretty-print and minified modes; handles all CSS rules, selectors, declarations, at-rules, and media queries
- Top-level `@if`/`@for`/`@each` control flow at stylesheet level (not just inside blocks)
- 66 unit tests across all modules plus integration tests
