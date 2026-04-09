# Changelog

All notable changes to `coding-adventures-semantic-ir` are documented here.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-04-07

### Added

- **`SIRSourceLocation`** — sidecar type carrying `file`, `start_line`, `start_col`,
  `end_line`, `end_col` (lines 1-based, columns 0-based, matching V3 source map spec).
  Every SIR node carries `loc: SIRSourceLocation | None = None`.

- **Type nodes** — `SIRAnyType`, `SIRNeverType`, `SIRVoidType`, `SIRPrimitiveType`
  (string / number / boolean / null / undefined / symbol / bigint), `SIRUnionType`,
  `SIRIntersectionType`, `SIRArrayType`, `SIRObjectField`, `SIRObjectType`,
  `SIRFunctionType`, `SIRGenericType`, `SIRReferenceType`, `SIRTupleType`.
  Union alias: `SIRType`.

- **`SIRParam`** — function parameter with `name`, `type_annotation`, `default_value`,
  `rest` (for `*args` / `...rest`), `loc`, `extra`.

- **Declaration nodes** — `SIRVariableDecl` (kind: let/const/var/assign),
  `SIRFunctionDecl` (async + generator flags), `SIRMethodDef` (constructor / method /
  getter / setter / static_method), `SIRPropertyDef` (static flag), `SIRClassDecl`
  (superclass + type_params), `SIRPropertySignature`, `SIRMethodSignature`,
  `SIRInterfaceDecl`, `SIRTypeAliasDecl`, `SIRImportSpecifier`, `SIRImport`,
  `SIRExportSpecifier`, `SIRExport`.
  Union alias: `SIRDeclaration`.

- **Statement nodes** — `SIRBlock`, `SIRExpressionStmt`, `SIRIfStmt`, `SIRWhileStmt`,
  `SIRForOfStmt` (async for-await), `SIRForInStmt` (JS key iteration), `SIRForStmt`
  (C-style), `SIRReturnStmt`, `SIRThrowStmt`, `SIRCatchClause`, `SIRTryStmt`,
  `SIRBreakStmt`, `SIRContinueStmt`, `SIRSwitchCase`, `SIRSwitchStmt`,
  `SIRLangSpecific` (escape hatch).
  Union alias: `SIRStatement`.

- **Expression nodes** — `SIRLiteral`, `SIRIdentifier`, `SIRBinaryOp`, `SIRUnaryOp`,
  `SIRSpread`, `SIRAssignment`, `SIRCall` (new + optional call), `SIRMemberAccess`
  (optional chaining), `SIRIndex`, `SIRConditional`, `SIRProperty`, `SIRObjectLiteral`,
  `SIRArrayLiteral` (elision holes via `None`), `SIRArrowFunction` (block and concise
  body), `SIRFunctionExpression`, `SIRTemplateLiteral`, `SIRAwait`, `SIRYield`
  (yield*), `SIRTypeAssertion`, `SIRSequence`.
  Union alias: `SIRExpression`.

- **`SIRModule`** — root node with `source_language`, `body`, `loc`, `extra`.

- **`SIRNode`** — top-level union alias covering all node categories.

- **`__all__`** — explicit public API list for clean star-imports.

- **`py.typed`** — PEP 561 marker enabling downstream type checkers to use the
  package's type annotations.

- **Tests** (`tests/test_semantic_ir.py`) — 180+ assertions covering:
  - Construction of every node type.
  - Default field values (`resolved_type` → `SIRAnyType()`, `loc` → `None`,
    `extra` → `{}`).
  - `type` discriminant correctness for all 60+ node types.
  - `SIRTemplateLiteral` quasis/expressions length invariant.
  - `extra` dict isolation (no accidental sharing between instances).
  - `resolved_type` instance isolation (each node gets its own `SIRAnyType`).
  - Complex tree construction (the canonical `greet()` function from the spec).
  - Source location threading at module, declaration, statement, and expression level.
