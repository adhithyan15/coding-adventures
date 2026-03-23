# Changelog

All notable changes to this package will be documented in this file.

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
