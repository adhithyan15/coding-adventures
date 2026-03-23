# Changelog

All notable changes to this package will be documented in this file.

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
