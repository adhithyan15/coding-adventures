# Changelog

All notable changes to this package will be documented in this file.

## [0.3.1] - 2026-03-25

### Fixed

- **Less-than operator `<`** — `numCompare` in `evaluator.go` now handles `"LESS"` so `@if $val < $min` evaluates correctly.
- **Division operator `/` in expressions** — `evalMultiplicative` now dispatches on `*` vs `/`. New `divide` method mirrors `multiply` with a zero-division panic guard.
- **Mixin parameter parsing with multiple defaults** — `extractParams` in `transformer.go` now accepts `mixin_value_list` AST nodes in addition to `value_list`.

## [0.3.0] - 2026-03-23

### Added — Lattice v2 Features

- **New error types**: `MaxIterationError`, `ExtendTargetNotFoundError`,
  `RangeError`, `ZeroDivisionInExpressionError` for v2 feature error handling.

- **`ScopeChain.SetGlobal()`** — sets a variable in the root (global) scope,
  implementing the `!global` flag.

- **`LatticeMap` value type** — ordered key-value map with `MapGet`, `MapKeys`,
  `MapValues`, `MapHasKey` methods. Used with `@each $key, $value in $map`.

- **Color conversion helpers** — `colorToRGB`, `colorToHSL`, `colorFromRGB`,
  `colorFromHSL`, `hueToRGB` for built-in color manipulation functions.

- **37 built-in functions** registered in `builtinFunctions` map:
  - Map: `map-get`, `map-keys`, `map-values`, `map-has-key`, `map-merge`, `map-remove`
  - Color: `lighten`, `darken`, `saturate`, `desaturate`, `adjust-hue`,
    `complement`, `mix`, `rgba`, `red`, `green`, `blue`, `hue`, `saturation`, `lightness`
  - List: `nth`, `length`, `join`, `append`, `index`
  - Type: `type-of`, `unit`, `unitless`, `comparable`
  - Math: `math.div`, `math.floor`, `math.ceil`, `math.round`, `math.abs`, `math.min`, `math.max`

- **`@while` loops** — `expandWhile()` with max-iteration guard (default 1000).

- **`!default` and `!global` variable flags** — `parseVariableDeclaration()` and
  `setVariableWithFlags()` handle both flags in `collectVariable` and
  `expandVariableDeclaration`.

- **`@content` blocks** — `expandContent()` replaces `@content;` in mixin bodies
  with the content block from `@include`. Evaluated in the caller's scope.
  Content block and scope tracked via `contentBlockStack`/`contentScopeStack`.

- **`@at-root`** — `expandAtRoot()` hoists rules out of nesting context to
  stylesheet root. Supports both block form and inline selector form.

- **`@extend` / `%placeholder`** — `collectExtend()` records extend relationships
  in `extendMap` for later selector merging.

- **Variables in selectors** — `expandSelectorWithVars()` resolves `VARIABLE`
  tokens in `compound_selector`, `simple_selector`, `class_selector` positions.

- **`@each` over maps** — `resolveEachList()` and `expandEachOverResolved()`
  support `@each $key, $value in $map` destructuring iteration.

- **Built-in function evaluation in transformer** — `evaluateBuiltinFunctionCall()`
  resolves arguments via `ExpressionEvaluator.collectFunctionArgs()` and calls
  the registered built-in handler.

- Comprehensive test suite for all v2 features: error types, scope operations,
  LatticeMap, color conversion, all 37 built-in functions, variable flags,
  @while, @content, @at-root, @extend, selector vars, @each over maps.
  Coverage: 81.5%.

### Changed

- `expandFunctionCall()` now checks user-defined functions first (highest
  priority), then CSS built-ins that are not Lattice built-ins, then Lattice
  built-ins, then remaining CSS built-ins.

- `expandInclude()` now detects trailing block nodes and pushes them as
  `@content` blocks for the mixin expansion.

- `expandControl()` now dispatches `while_directive` to `expandWhile()`.

- `expandLatticeBlockItem()` now handles `content_directive`, `at_root_directive`,
  and `extend_directive`.

- `expandNode()` now handles `compound_selector`, `simple_selector`, and
  `class_selector` for variable resolution in selectors.

- `expandEach()` now checks if the each list resolves to a `LatticeMap` or
  `LatticeList` before falling through to token-based iteration.

## [0.2.0] - 2026-03-23

### Added

- `errors.go` — Full LatticeError hierarchy: `UndefinedVariableError`,
  `UndefinedMixinError`, `UndefinedFunctionError`, `WrongArityError`,
  `CircularReferenceError`, `TypeError`, `MissingReturnError`,
  `ModuleNotFoundError`, `ReturnOutsideFunctionError`. All implement the
  `error` interface and embed a base `LatticeError` struct with `Line`,
  `Column`, and `Message` fields.

- `scope.go` — `ScopeChain` type for lexical block scoping. Supports `Get`,
  `Set`, `Child` operations. Inner scopes shadow outer scopes; sibling scopes
  are isolated.

- `evaluator.go` — `ExpressionEvaluator` for compile-time expression
  evaluation. Supports arithmetic on numbers and dimensions (same-unit
  addition, cross-unit `calc()` emission), comparisons (`==`, `!=`, `>`,
  `>=`, `<=`), logical operators (`and`, `or`, `not`/`negate`), and variable
  lookup. `LatticeValue` discriminated union with subtypes: `LatticeNumber`,
  `LatticeDimension`, `LatticePercentage`, `LatticeString`, `LatticeIdent`,
  `LatticeColor`, `LatticeBool`, `LatticeNull`.

- `transformer.go` — `LatticeTransformer` three-pass compiler:
  - Pass 1: module resolution (`@use` stub — collects symbols, no file I/O)
  - Pass 2: symbol collection (variables, mixins, functions)
  - Pass 3: recursive AST expansion (variable substitution, `@include`
    expansion with arity/defaults/cycle detection, `@if`/`@else if`/`@else`,
    `@for through`/`@for to`, `@each`, user-defined function calls)
  - Grammar fallback: multi-parameter `@mixin`/`@function` definitions that
    the parser emits as `at_rule` are detected and collected via
    `collectMixinFromAtRule` / `collectFunctionFromAtRule`
  - User-defined functions take priority over same-named CSS built-ins
    (e.g., `scale` as a Lattice function is not confused with CSS transform)

- `emitter.go` — `CSSEmitter` that walks a clean CSS AST and reconstructs
  CSS text. Supports pretty-print (configurable indent string) and minified
  modes. Dispatches on `RuleName`: `stylesheet`, `qualified_rule`,
  `selector_list`, `block`, `block_contents`, `block_item`,
  `declaration_or_nested`, `declaration`, `value_list`, `at_rule`,
  `function_call`. Falls back to recursive child expansion for unknown nodes.

- `lattice_ast_to_css.go` — Public API:
  - `TranspileLatticeFull(source, minify, indent)` — full pipeline
  - `TranspileLattice(source)` — pretty-print convenience wrapper
  - `TranspileLatticeMinified(source)` — minified convenience wrapper
  - `TransformLatticeAST(ast)` — transform step only
  - `EmitCSS(ast)` — emit step only (pretty-print)
  - `EmitCSSMinified(ast)` — emit step only (minified)

- Comprehensive test suite in `lattice_ast_to_css_test.go` covering: variable
  substitution, mixin expansion (with/without args, with defaults, nested),
  `@if`/`@else if`/`@else`, `@for through`/`@for to`, `@each`, `@function`,
  scope shadowing, emitter modes, arithmetic, error cases, `ScopeChain` unit
  tests, `LatticeValue` type/string tests, error type unit tests, `CSSEmitter`
  unit tests, and integration tests. Coverage: 80.4%.

### Changed

- Spec divergence: `@for` and `@each` loops must appear inside a mixin or
  function body (they are `lattice_block_item` constructs). The grammar does
  not parse them as top-level rules. Spec updated to reflect this constraint.

- Spec divergence: `#{}` string interpolation is not implemented. The Lattice
  lexer panics on bare `#` characters (they are only valid as hex color `HASH`
  tokens). Spec updated: interpolation is a non-goal by design (it would
  require modal lexing and a context-sensitive grammar).

- Spec divergence: the `<` comparison operator is not in the Lattice grammar.
  Only `>`, `>=`, `<=`, `==`, `!=` are defined. Use `>=` / `<=` for
  range checks.

## [0.1.0] - 2026-03-23

### Added

- Initial package scaffolding generated by scaffold-generator
