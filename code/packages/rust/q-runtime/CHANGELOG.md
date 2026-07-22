# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-22

### Added

- Initial release — **MA-11d** of the Q frontend (spec
  [`MA11`](../../../specs/MA11-q-language.md)): a tree-walking evaluator over
  `array-runtime`, making the Q lexer/parser (`q-lexer`/`q-parser`,
  MA-11b/MA-11c) executable.
- `Interpreter` (persistent workspace) + `feed`/`eval` entry points.
  Auto-print semantics: an assignment is silent, a bare `noun_expr` result
  auto-prints (mirrors `j-runtime`'s/`apl-runtime`'s own real-session
  convention, and real Q's own console behavior).
- **`QValue`** — this crate's value type: `Arr(Array)` or `Fn(Rc<Lambda>)`.
  Unlike `j-runtime`/`apl-runtime` (whose evaluators only ever produce a
  bare `Array`), Q's function literal is itself an ordinary noun value
  (MA11 §2/§3 bullet 1), so a bare array is not enough — this is the direct
  consequence of that headline novelty flowing through the whole evaluator.
- **`QFn::Lambda`** — the one genuinely new evaluator concept MA11 §2
  flags: a user-defined function literal (`{[x;y] stmt; stmt; ...}`), with
  named parameters, a multi-statement body, and (implicitly) whatever
  global bindings are visible at call time.
  - The bracket-omitted implicit `x`/`y`/`z` parameter names when no
    `[x;y]` list is present at all (a real Q convenience, MA11 §3 bullet 1
    / §4). `q.grammar` simply omits the `param_list` child in this case, so
    an *absent* node (not an empty one) is the signal `build_lambda` reads.
  - Calling a function value uses the **same** juxtaposition dispatch site
    (`Interpreter::apply_monadic`/`apply_dyadic`) as every primitive verb —
    no separate "call a lambda" code path (MA11 §3 bullet 1: "no new
    application production, only a new way to produce a callable value").
  - Assignment inside a function body is local to that call only
    (MA11 §4): a call-local frame is pushed onto an environment *stack*
    (`RefCell<Vec<HashMap<String, QValue>>>`) for the duration of the call
    and popped again via an RAII `FrameGuard`, mirroring
    `j-runtime::eval::DepthGuard`'s "guard owns the undo" pattern.
  - **Nested function literals are a clean, explicit error** (MA11 §4: "no
    nested function literals to begin with"), checked at the moment a
    `function_literal` node is evaluated while already inside an active
    call — not just when the inner literal happens to be invoked, since
    even a merely-constructed-and-returned nested literal would need real
    lexical closure capture (which this crate deliberately does not
    implement) to behave correctly if later called independently.
  - First-class functions: a parameter bound to a `Fn` value can itself be
    applied inside the callee's body (`apply:{[g] g 5}`) — a natural,
    disclosed generalization of "a function literal is an ordinary noun
    value... assignable, passable" (MA11 §3 bullet 1), distinct from the
    explicitly-deferred nested-*definition* case above.
- **16 primitive verbs** (MA11 §4's full table): `+` flip/add, `-`
  negate/subtract, `*` first/multiply, `%` reciprocal/divide (always
  true/float division, no integer-division special case), `!` til
  (monadic-only, **0-based** — matches J's `i.`, never APL's 1-based `⍳`;
  dyadic `!` is a clean, explicit "not yet implemented" error, MA11 §4's
  own deferral), `,` enlist/join, `#` count/take, `_` floor/drop, `&`
  where/min, `|` reverse/max, `~` not/match (deep equality, producing a
  scalar — the one primitive in this crate whose dyadic meaning is *not*
  elementwise), `=` `<` `>` `<=` `>=` `<>` comparison (Q's own not-equal
  spelling, never `~=`/`#`).
  - Exactly 12 of the 16 map onto `array_runtime::ops::BinOp` for their
    *dyadic* meaning (`Prim::to_binop`); the remaining 5 (`!` `,` `#` `_`
    `~`) get hand-rolled logic in `builtins.rs` — the same count and
    structural split as `j-runtime`'s own 5 bespoke primitives (`$` `i.`
    `,` `#` `^`).
  - Genuinely new relative to J: several `BinOp`-mappable primitives (`+`
    `*` `&` `|`) have a *monadic* meaning that is not itself an elementwise
    scalar map (flip/transpose, first, where, reverse) — J's own 12
    `BinOp`-mappable atoms were all uniformly elementwise monadically, so
    this asymmetry falls straight out of Q's own primitive table rather
    than being invented here.
- **Adverbs**: `'` (each), `/` (reduce), `\` (scan) — reduce/scan are
  restricted to the 12 `BinOp`-mappable primitives exactly like
  `j-runtime::eval::require_scalar_binop`; each has a well-defined,
  non-redundant meaning only for the primitives whose direct application
  is *not already* elementwise scalar (this cut's flat, dense-array-only
  value model has no nested/boxed list type, so "apply per element" is
  never distinguishable from "apply directly" for the primitives where the
  direct form is already elementwise) — everything else is a clean,
  disclosed "no well-defined per-element meaning for this primitive" error.
  `q.grammar` only ever attaches an adverb to a bare `verb_primitive`
  (confirmed directly against the grammar file), so none of the three ever
  need to handle a user-defined `Lambda`.
- **Dual list-literal syntax** (MA11 §3 bullet 3 / §4): adjacent-numeric
  stranding (`1 2 3`) and explicit `(a; b; c)` both lower to the same
  vector value when every element is a plain scalar. A list literal
  containing a non-scalar (itself a vector/matrix) or function-valued
  element is a clean, disclosed error — this cut's `array_runtime::Array`
  value model has no nested/heterogeneous representation at all (MA11 §4:
  "arrays only, dense and numeric"), so such an element genuinely cannot be
  represented, rather than being silently flattened or truncated.
- Q-style display (`value.rs`): a plain ASCII `-` for negatives (unlike
  J's leading underscore) — Q spells a negative *literal* with an ordinary
  `-`, disambiguated from subtraction by whitespace (MA11 §3 bullet 2), so
  printing with plain `-` round-trips correctly through `q-lexer`'s own
  negative-literal fold hook, unlike J (whose lexer reserves ASCII `-`
  exclusively for the `MINUS` token).
- DoS guards: an independent recursion-depth guard in the evaluator
  (`eval.rs::MAX_DEPTH`, 512) — **genuinely reachable** through a
  legitimate, sufficiently long chain of already-defined-function calls
  across many separate top-level lines (unlike `j-runtime`'s identical
  guard, which per that crate's own doc comment is "never actually
  reachable through genuine parsed input" since J has no user-defined
  functions to chain calls through) — plus `builtins::MAX_ARRAY_LENGTH`
  (1,000,000) capping every primitive whose output size or work scales
  with a runtime-computed value (`!n`, take, join, and the flat
  stranded-literal/list-literal element counts), each checked *before*
  allocating or scanning.

### Notes

- No `array-runtime` substrate changes were needed (MA11 §2's own "zero new
  substrate" finding, confirmed directly against `ops.rs`'s current public
  API) — every primitive here is either `ops::elementwise`/`reduce`/`scan`
  wearing a new glyph, or hand-rolled logic local to this crate.
- `QFn` has **no** train-shaped variants (`Compose`/`Hook`/`Fork`) — Q has
  no trains and no `@` compose in this cut at all (MA11 §3/§4), so unlike
  `j-runtime::eval::JFn` (which needed a hand-rolled iterative `Drop` to
  avoid a native-stack overflow tearing down a deeply-boxed train), `QFn`'s
  variants are all leaf dispatches with no self-referential recursion —
  nothing here needed (or got) that treatment.
