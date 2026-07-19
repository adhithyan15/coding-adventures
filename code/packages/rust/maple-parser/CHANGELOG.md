# Changelog

## [0.1.0] - 2026-07-19

### Added

- Initial grammar-driven Rust Maple parser (MA09 §2, task MP-3).
- `maple.grammar` (compiled ahead of time into the committed
  `src/_grammar.rs`), implementing the MP-1-scoped precedence cascade from
  MA09 §3: a `statement`/`expr` split (`statement = if_expr | assignment`,
  `expr = logical_or -> ... -> atom`) → assignment (`:=`, left-hand side a
  bare `NAME`) → the arrow operator (`->`, `Define`-right-hand-side only,
  via a dedicated `arrow_def` production) → `or` → `and` → `not` →
  comparison (`=`/`<>`/`<`/`>`/`<=`/`>=`, flat non-chaining) → additive →
  multiplicative (explicit `*` required, no juxtaposition production exists
  anywhere in the grammar) → unary minus → power (`^` only, right-assoc,
  no `**` synonym) → postfix function-call application (single call
  suffix) → atoms, plus square-bracket list literals and curly-brace set
  literals (both reusing one shared `arglist` production).
- Three deliberate, disclosed divergences from `reduce-parser`'s
  identical-looking shape, each with its own header comment in
  `maple.grammar` citing MA09's own text:
  1. **`if` and `:=` are statement-only, never nested inside an `expr`.**
     MA09 makes no equivalent claim to MA08 §3's explicit "if... returns
     whichever branch ran" for Maple, and real Maple's own conditional-value
     idiom is the `piecewise(...)` library call (an ordinary function call
     this grammar already parses for free), not embedding `if` where a
     value is expected — corroborated by MA09 §6's own citation split
     (Chapter 5 "Maple Statements" vs. Chapter 3 "Maple Expressions").
     Consequence: `x := if a then 1 else 2 end if;` and chained assignment
     `a := b := c;` are both syntax errors in this subset.
  2. **`assignment`'s left-hand side is a bare `NAME`, not a general
     call-shaped expression.** REDUCE's `h(l,m) := e` IS its general
     definition idiom, so `reduce.grammar`'s LHS is deliberately a full
     `logical_or` (bottoming out at a call). Maple's identical-looking
     `f(x) := e` means the narrower, EXCLUDED (MA09 §4) remember-table
     mechanism instead, so this grammar never lets it parse at all —
     `f(x) := 1;` fails at the terminator check, not at a later lowering
     stage.
  3. **`postfix` allows at most one call suffix**, not `reduce.grammar`'s
     repeated `{ LPAREN [arglist] RPAREN }` chain — MA09 documents no
     analogue of REDUCE's `a(5)`/`b(i,q)` array-subscript-read convention
     that motivated the repeated shape there.
- The `if`/`elif`/`else`/(`end if`|`fi`) conditional, MA09's one genuinely
  new grammatical shape relative to every CAS-family sibling in this repo:
  `if_expr = "if" expr "then" statement { "elif" expr "then" statement }
  [ "else" statement ] ( "end" "if" | "fi" ) ;`. Deterministic, ambiguity-free
  ordered choice between the two closing spellings (`end`/`if`/`fi` are
  three distinct KEYWORD values). Unlike `reduce-parser`'s `if`, there is
  **no dangling-else ambiguity** here at all, because every `if_expr`
  requires an explicit close before an enclosing `if`'s own `elif`/`else`
  can ever be reached.
- A bespoke `MAX_RULE_DEPTH = 150` recursion-depth cap. Six distinct
  self-referential productions were measured independently (parenthesised
  nesting, list-literal nesting, a `not` prefix chain, a unary-minus prefix
  chain, a power chain, nested `if`/`end if`) — every "flat chain of one
  operator" production written with EBNF `{ x }` repetition instead was
  confirmed, by reading `parser::grammar_parser`'s own `Repetition`
  implementation directly, to cost zero native stack regardless of width
  (the same engine-level fact `reduce-parser` already established, reused
  here rather than re-measured by a fresh throwaway probe). Set-literal
  nesting was proven structurally identical to list-literal nesting by
  direct inspection of the shared `arglist` production (`list_literal` and
  `set_literal` both wrap `arglist` identically, differing only in which
  bracket token is matched), so it was not separately measured — a provable
  identity, not an assumed shape resemblance.
- A genuine surprise once each shape's *nesting-count* crash floor was
  converted into *rule-frame* terms (the units `MAX_RULE_DEPTH` actually
  bounds): the `not` prefix chain tolerates by far the *most* nesting levels
  of the six (205 safe / 206 crash, alongside its near-twin the unary-minus
  chain) but has the *lowest* rule-frame floor (218 safe / 219 crash) —
  lower than nested-`if`'s 289/290, despite nested-`if` crashing at far
  fewer levels (137/138), and lower than parenthesised nesting's 298/299,
  which crashes at the fewest levels of all (23/24). Neither "the shape
  that tolerates the fewest levels must bind" nor "parenthesised nesting
  binds, since it does for nearly every sibling `*-parser` crate in this
  repo" holds here — both would have shipped a cap unsafe specifically for
  `not`/unary-minus prefix chains. `150` sits about 31.2% below the binding
  218 floor. Full measurement table and reasoning in `MAX_RULE_DEPTH`'s own
  doc comment (`src/lib.rs`).
- 45 tests covering every construct in the MP-1 surface grammar: function
  calls (including nested and multi-arg), assignment vs. equation (`:=` vs
  `=`) staying distinct, every comparison operator, the arrow-operator
  `Define` shape with two parameters, one bare parameter, and zero
  parameters, a regression test confirming a plain `f := x;` assignment
  does *not* spuriously produce an `arrow_def` node, regression tests
  confirming `->` never appears outside an assignment's right-hand side or
  nested inside arithmetic, a regression test confirming the remember-table
  spelling `f(x) := e` / `h(l, m) := e` is rejected outright, regression
  tests confirming `if` is not usable as an assignment's right-hand side and
  that chained assignment is rejected, list vs. set literal parsing as
  genuinely distinct productions (including empty `[]`/`{}`), boolean
  literals and keywords with their lowercase-only case sensitivity, a
  `bare_juxtaposed_names_with_no_operator_is_rejected` regression test
  mirroring `reduce-parser`'s own, arithmetic precedence/associativity
  (including the explicit-`*`-required check that `a ** b` does not parse as
  a single power expression), grouping, `if`/`elif`/`else` closed both ways
  (`end if` and bare `fi`, proven structurally equivalent) with elif-chain
  preservation and assignment-branching, nested `if` resolving with no
  dangling-else ambiguity, multi-statement programs, syntax-error rejection,
  and 4 depth-guard tests exercising all six measured shapes at once (deep
  adversarial input on an enlarged-stack thread returns a clean error for
  every shape, the cap trips before the native stack would overflow even on
  a default-stack thread for every shape, reasonable hand-written nesting
  for every shape stays well under the cap, and an exact boundary test —
  mirroring `j-parser`'s own per-shape boundary tests — proving the measured
  real-input headroom for every shape parses cleanly one level below the
  cap and trips cleanly one level at the cap).
- `code/grammars/maple/maple.grammar` validated with
  `grammar-tools validate-grammar` and cross-validated against
  `code/grammars/maple/maple.tokens` with `grammar-tools validate`.
