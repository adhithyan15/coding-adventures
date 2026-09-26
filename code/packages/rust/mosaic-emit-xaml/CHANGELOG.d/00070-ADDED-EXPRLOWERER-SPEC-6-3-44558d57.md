### Added — `ExprLowerer` (spec §6.3)

- A small recursive-descent parser-and-lowerer over the UI29 §3.3
  expression grammar. Returns one of:
  - `Bindable(path)` — direct `{x:Bind X}` path. Covers bare slot ref
    (`slot: foo`), bare for-bound name (`row`), boolean literal
    (`true`/`false`), and dotted member access (`row.value.bg`).
  - `Helper(call)` — a registered helper-method call (e.g.
    `Expr_a3f24b6c(R, C)`). Covers indexers (`row[c]`), comparisons
    (`==`, `!=`, `<`, `<=`, `>`, `>=`), logical (`&&`, `||`), and
    unary `!`.
  - `Unsupported(reason)` — anything else gets a human-readable
    diagnostic via `PipelineEmitError::UnsupportedExpression`.
- Helper methods land inline in the code-behind partial class as
  `private <Type> Expr_<hash>(<params>) => <body>;`. The body is a
  direct transliteration of the moslayout expression to C# (operators
  carry through identically; `slot:` becomes `this.`; for-bound names
  become PascalCased parameters).
- Helpers dedupe by name (deterministic FNV-1a hash of the original
  expression source).

