# Changelog

## [0.1.0] - 2026-07-17

### Added

- Initial grammar-driven Rust Reduce parser (MA08 §2, task R-3).
- `reduce.grammar` (compiled ahead of time into the committed
  `src/_grammar.rs`), implementing the R-1-scoped precedence cascade from
  MA08 §3: `if`/`<< ... >>` (usable as expressions, sitting above
  assignment via an ordered-choice `expr = if_expr | group_expr |
  assignment` shape mirroring `macsyma-parser`'s own `expression`) →
  assignment (`:=`, right-associative, shared by both variable assignment
  and procedure definition) → `or` → `and` → `not` → comparison (`=`/
  `neq`/`<`/`>`/`<=`/`>=`, flat non-chaining) → cons (`.`, right-
  associative — a tier MA08 itself leaves unplaced in the manual's own
  precedence table; this crate binds it looser than `additive` but
  tighter than `comparison`, documented in `reduce.grammar`'s header and
  in a new MA08 §3 addendum) → additive → multiplicative → unary minus →
  power (`^`/`**`, same operator, right-associative) → postfix
  function/procedure/array-subscript call (one production for all three
  per MA08 §3) → atoms, plus curly-brace list literals (`{a, b, c}`).
- A bespoke `MAX_RULE_DEPTH = 128` recursion-depth cap. Five distinct
  self-referential productions were measured independently (parenthesised
  nesting, a `:=` chain, an `if`/`else` chain, a cons chain, a power
  chain) — every "flat chain of one operator" production written with
  EBNF `{ x }` repetition instead was separately confirmed, via a
  throwaway probe grammar, to cost *zero* native stack regardless of
  width (parsed one million repeated items on a default-stack thread with
  no crash), so those needed no measurement at all.
- A genuine surprise once each shape's *nesting-count* crash floor was
  converted into *rule-frame* terms (the units `MAX_RULE_DEPTH` actually
  bounds): the cons chain tolerates the *most* nesting levels of the five
  (163 safe / 164 crash) but has the *lowest* rule-frame floor (179 safe /
  180 crash) — lower than parenthesised nesting's 289/290, despite parens
  crashing at far fewer levels (19/20). Parenthesised nesting binds for
  nearly every sibling `*-parser` crate in this repo; here it does not
  once measured in the frame terms the depth guard actually enforces, and
  assuming it did (without the second, rule-frame-terms measurement)
  would have shipped a cap unsafe specifically for cons chains. `128` sits
  about 28.5% below the binding 179 floor. Full measurement table and
  reasoning in `MAX_RULE_DEPTH`'s own doc comment (`src/lib.rs`).
- Caught and fixed a bug in this crate's *own measurement methodology*
  along the way: an early cons-chain probe built from repeated `1.1.1...`
  NUMBER literals silently measured a diluted, halved chain, because
  `NUMBER`'s own regex (`[0-9]+\.?[0-9]*`) greedily absorbs one trailing
  `.digit` run per token — switching the probe to `NAME` atoms (`a.a.a...`,
  immune to the ambiguity) produced a floor consistent with the other
  four shapes.
- 36 tests covering every construct in the R-1 surface grammar (function/
  procedure calls, array-subscript reads, the shared `:=` token across
  both assignment forms, `=`-vs-`:=` disambiguation, every comparison
  operator, curly-brace list literals and the `list(...)` spelling, the
  cons operator and its precedence relative to `additive`, list
  accessors/constructors, arithmetic precedence and associativity,
  `^`/`**` as the same operator, grouping, the boolean keywords and their
  lowercase-only case sensitivity, `if`/`else` including dangling-else
  resolution and usability as an expression, group statements and their
  usability as an expression, multi-statement programs, syntax-error
  rejection) plus a regression test for a juxtaposition-acceptance bug
  found while writing this crate (see below) and 3 depth-guard tests
  exercising all five measured shapes at once (deep adversarial input on
  an enlarged-stack thread returns a clean error for every shape, the cap
  trips before the native stack would overflow even on a default-stack
  thread for every shape, and reasonable hand-written nesting for every
  shape stays well under the cap).
- Fixed a grammar-design bug found while writing this crate's own tests:
  an early draft of `program`'s grammar folded "the last statement may
  have no trailing terminator" into `statement_line` itself as a third
  alternative, tried on *every* repetition iteration rather than only at
  the very end — so `a AND b;` (with `AND` lexing as an ordinary `NAME`,
  since `reduce.tokens`' keywords are lowercase-only, and implicit
  multiplication by juxtaposition out of scope per MA08 §4) silently
  parsed as three separate no-terminator statements (`a`, `AND`, `b;`)
  instead of failing as a syntax error. Fixed by moving the terminator-
  less case to a single `[ statement ]` *outside* the repetition, so it
  can only ever match once. Documented in `reduce.grammar`'s own `program`
  comment; regression test:
  `bare_juxtaposed_names_with_no_operator_is_rejected`.
