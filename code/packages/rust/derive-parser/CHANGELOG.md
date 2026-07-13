# Changelog

## [0.1.0] - 2026-07-13

### Added

- Initial grammar-driven Rust Derive parser (MA07 §2, task D-3).
- `derive.grammar` (compiled ahead of time into the committed
  `src/_grammar.rs`), implementing the D-1-scoped precedence cascade from
  MA07 §3: assignment (`:=`, right-associative, shared by both variable
  assignment and function definition — `x := 5` and `F(x) := x^2 + 1`
  parse through the identical rule, disambiguated only at the later D-4
  lowering stage) → `OR` → `AND` → `NOT` → comparison (`=`/`<=`/`<`/`>`/`>=`)
  → additive → multiplicative → unary minus → power (`^`, right-associative)
  → postfix function/named-builtin application (`DIF(u, x)`, ordinary
  parentheses — Derive's defining syntactic difference from Wolfram's
  `f[x]`) → atoms, plus vector/matrix literals (`[a, b, c]` /
  `[a, b, c; d, e, f]`, `;` as the row separator).
- A bespoke `MAX_RULE_DEPTH = 200` recursion-depth cap, empirically measured
  the same way `r-parser`/`s-parser`/`macsyma-parser`/`nib-parser`/
  `oct-parser` measured their own values (the shared crate's generic
  `DEFAULT_MAX_RULE_DEPTH` of 128 was not assumed safe, and this grammar's
  own native-stack floor was measured independently rather than reused from
  a sibling): parses safely to 21 real nesting levels on an uncapped,
  default-stack worker thread, crashes at 22 (297/298 in rule-frame terms —
  coincidentally the exact same floor `r-parser` measured). 200 sits about
  33% below that floor and still supports 14 real nesting levels before
  tripping.
- 16 tests covering every construct in the D-1 surface grammar (function
  application, the shared `:=` token across both assignment forms, `=`-vs-
  `:=` disambiguation, vector/matrix literals, the boolean keywords,
  arithmetic precedence and associativity, grouping, nested/multi-arg
  builtin calls, syntax-error rejection) plus 3 depth-guard regression
  tests mirroring the sibling crates' own (deep adversarial input on an
  enlarged-stack thread returns a clean error, input at the measured
  real-nesting boundary still parses one level past it doesn't, and the
  cap trips before the native stack would overflow even on a default-stack
  thread).
