# Changelog — cas-pretty-printer (Rust)

## [0.1.0] — 2026-04-27

### Added

- Initial Rust port of the Python `cas-pretty-printer` package.
- `Dialect` trait in `dialect.rs`: contract every dialect must satisfy.
  - Methods: `name`, numeric formatters (`format_integer`, `format_rational`,
    `format_float`, `format_string`, `format_symbol`), operator spellings
    (`binary_op`, `unary_op`), `function_name`, `list_brackets`,
    `call_brackets`, `precedence`, `is_right_associative`, `try_sugar`.
- Precedence constants: `PREC_OR` (10) through `PREC_ATOM` (100).
- Default free functions in `dialect.rs`:
  - `default_binary_op` — shared infix table (`+`, `-`, `*`, `/`, `^`, …).
  - `default_unary_op` — shared prefix table (`-`, `not `).
  - `default_function_name` — MACSYMA lowercase aliases (`Sin` → `sin`, …).
  - `default_precedence` — shared precedence table.
- Walker in `walker.rs`: `pretty(node, dialect) -> String`.
  - 6-step dispatch: sugar → custom formatter → List literal → unary op →
    binary/n-ary op → function call.
  - Correct associativity-aware parenthesisation for left- and right-
    associative operators.
  - Negative integer / rational / float literals wrapped in parens when
    `min_prec > 0`.
- `register_head_formatter` / `unregister_head_formatter` — global
  thread-safe registry for extending the walker with new IR heads.
  Uses `Arc<HeadFormatterFn>` (cloned before calling) to avoid deadlock
  when the formatter calls back into `pretty`.
- `MacsymaDialect` — MACSYMA/Maxima flavor.
  - `try_sugar`: `Mul(-1,x)→Neg(x)`, `Add(a,Neg(b))→Sub(a,b)`,
    `Mul(a,Inv(b))→Div(a,b)`.
  - Lowercase function names; `[…]` lists; `(…)` calls.
- `MathematicaDialect` — overrides `Equal`→`==`, `NotEqual`→`!=`,
  `And`→`&&`, `Or`→`||`, `Not`→`!`; `{…}` lists; `[…]` calls;
  CamelCase function names.  Shares MACSYMA sugar.
- `MapleDialect` — overrides `NotEqual`→`<>`.  All else identical to
  MACSYMA.
- `LispDialect` — no binary/unary ops; every head formatted as function
  call with space-separated args.
- `format_lisp(node) -> String` — standalone S-expression renderer that
  bypasses the walker and all registered head formatters.
- 60 integration tests + 6 doc-tests; all passing.
