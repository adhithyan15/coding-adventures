# Changelog

All notable changes to `wolfram-runtime` are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/) and this project uses
[Semantic Versioning](https://semver.org/).

## [0.11.0] — 2026-06-20

The **W-14** deliverable (MA04 §17): Wolfram's **conditionals** and **type
predicates**, lowered onto the same substrate as the rest of the lane. `Which` and
`Switch` join the `WolframBackend` held set (alongside `If`, the W-7 iteration
heads, and the W-8 scoping heads) so that **only the selected branch is ever
evaluated** — a non-taken branch (which might error or have a side effect) never
runs. `Switch`'s form matching reuses the W-13 `same_element` comparator, so it
agrees with `MemberQ`/`Union` on what "the same" means, and recognises `Blank[]`
(the lowering of `_`) as the catch-all default. The eager `Boole` and the
`NumberQ`/`IntegerQ`/`StringQ`/`ListQ`/`TrueQ` predicates are thin matches over the
`IRNode` kind. Like every head since W-5 these are plain `Head[args]` applications,
so there is **no grammar change**; only the `wolfram-runtime` builtin handler table
and the held set grow. `EvenQ`/`OddQ` (W-9) are left unchanged.

### Added (conditionals — held)

- **`Which[c1, v1, c2, v2, …]`** — evaluate conditions left to right; return the
  value paired with the **first** condition that reduces to `True`. Held: only the
  selected value is evaluated. No true condition → `Null` (the evaluated answer);
  an **odd** argument count (dangling final condition) → left unevaluated.
  (`Which[False, 1, True, 2]` → `2`; `Which[False, 1]` → `Null`;
  `Which[2 > 1, "a"]` → `"a"`.)
- **`Switch[expr, form1, v1, …, _, default]`** — evaluate `expr` once, then match
  it against each **literal** `formi` by structural equality (W-13 `same_element`);
  `Blank[]` (`_`) matches anything as the default. Held: only the selected value is
  evaluated. No match → left unevaluated; an **even** argument count (final
  unpaired form, or missing `expr`) → left unevaluated.
  (`Switch[2, 1, "a", 2, "b", _, "z"]` → `"b"`; `Switch[5, 1, "a", _, "z"]` → `"z"`.)

### Added (conditionals — eager) and type predicates

- **`Boole[cond]`** — `True` → `1`, `False` → `0`; any other (non-boolean)
  argument is left unevaluated. (`Boole[2 > 1]` → `1`; `Boole[1 > 2]` → `0`.)
- **`NumberQ[x]`** — `True` for a real number (`Integer`/`Rational`/`Float`).
- **`IntegerQ[x]`** — `True` only for an exact integer (`IntegerQ[2.0]` is `False`).
- **`StringQ[x]`** — `True` for a string literal.
- **`ListQ[x]`** — `True` for a `List[…]` (reuses `is_list`).
- **`TrueQ[x]`** — `True` only for the literal `True` symbol; total — `False` for
  everything else (including a free symbol), never unevaluated.

### Security / robustness

- `Which`/`Switch` evaluate **exactly one** branch via a single `vm.eval`, so a
  non-selected branch cannot double-evaluate, error, or produce a side effect.
- Odd-arity `Which` and even-arity `Switch` (malformed pair lists) are detected
  *before* any `chunks_exact(2)` walk and left unevaluated — no index can run past
  the end of the argument list, no panic. The predicates and `Boole` reject
  arity ≠ 1 the same way. No new unbounded-recursion or growth surface is added
  beyond the single selected-branch `vm.eval`, which the W-4 fuel machinery bounds.

## [0.10.0] — 2026-06-19

The **W-13** deliverable (MA04 §16): Wolfram's **list set / multiset operations**,
lowered onto the *same* substrate as the rest of the lane — the W-9 list machinery
(`list_elements`, `apply(sym(LIST), …)`, the `MAX_LIST_LENGTH` cap) and the W-9
canonical-order comparator `canonical_cmp`, reused both to *sort* the unique
outputs of `Union`/`Intersection`/`Complement` and to define **element-equality**
(two nodes are the same element iff `canonical_cmp` ranks them `Equal`). Like every
head since W-5 these are plain `Head[args]` applications, so there is **no grammar
change**; only the `wolfram-runtime` builtin handler table grows. `Count` (W-9,
predicate form) is left as-is.

### Added (list set operations)

- **`Union[a, b, …]`** — the **sorted**, duplicate-free union of the element lists
  (`Union[{1, 2}, {2, 3}]` → `{1, 2, 3}`; `Union[{3, 1, 2, 1}]` → `{1, 2, 3}`, so a
  single argument doubles as sort-and-unique). DoS-capped at `MAX_LIST_LENGTH` —
  the deduped accumulator is refused (form left unevaluated) before it can exceed
  the cap, symmetric with `Join`/`Flatten`.
- **`Intersection[a, b, …]`** — the **sorted** elements present in *every* argument
  list (`Intersection[{1, 2, 3}, {2, 3, 4}]` → `{2, 3}`).
- **`Complement[all, x, …]`** — the **sorted** elements of `all` not in any of
  `x, …` (`Complement[{1, 2, 3, 4}, {2, 4}]` → `{1, 3}`).
- **`DeleteDuplicates[list]`** — first-occurrence-order dedup, **order-preserving**
  and deliberately *not* sorted (`DeleteDuplicates[{3, 1, 1, 2, 3}]` → `{3, 1, 2}`,
  contrast with `Union`'s `{1, 2, 3}`).
- **`MemberQ[list, elem]`** — `True`/`False` whether `elem` is an element of
  `list` (`MemberQ[{1, 2, 3}, 2]` → `True`; `MemberQ[{1, 2, 3}, 9]` → `False`).
- **`Tally[list]`** — `{element, count}` pairs in first-occurrence order
  (`Tally[{a, a, b, a}]` → `{{a, 3}, {b, 1}}`). The distinct-element count is capped
  at `MAX_LIST_LENGTH`.

### Notes

- **Element-equality reuses the W-9 comparator** (`same_element(a, b) ≡
  canonical_cmp(a, b) == Equal`): deterministic, consistent with `Sort`, and
  panic-free for `NaN` (built on `f64::total_cmp`). The type-tag tie-break keeps
  distinct numeric subtypes of equal magnitude separate, so `2` and `2.0` are
  **distinct** elements — matching Wolfram (`Union[{2, 2.}]` keeps both).
- **Two ordering families**: `Union`/`Intersection`/`Complement` sort their
  outputs; `DeleteDuplicates`/`Tally` preserve first-occurrence order.
- **DoS / cost**: outputs never exceed the sum of input lengths (already bounded by
  the W-4 input/token caps); each head re-asserts `MAX_LIST_LENGTH` defensively.
  Membership is a linear `canonical_cmp` scan (no hashing — `IRNode` carries an
  `f64` and is not value-`Hash`-keyable), so the heads are worst-case quadratic in
  the (bounded) input — a documented simplicity trade, never unbounded.
- **No grammar change**: lexer, parser, and grammar files are untouched; only the
  builtin handler table grows.

## [0.9.0] — 2026-06-19

The **W-12** deliverable (MA04 §15): Wolfram's **string builtins**, lowered onto
the *same* substrate as the rest of the lane — the string atom is already
`IRNode::Str(String)` (the W-4 lexer produces it, the printer renders it), and
`StringSplit`/`Characters` reuse the W-9 list machinery (and its
`MAX_LIST_LENGTH` cap). Like every head since W-5 these are plain `Head[args]`
applications, so there is **no grammar change**; only the `wolfram-runtime`
builtin handler table grows. The `<>` infix sugar for `StringJoin` is **deferred**
to a future grammar-change lane item.

### Added (string builtins)

- **`StringJoin[a, b, …]`** — concatenate string arguments (`StringJoin["a","b"]`
  → `"ab"`; `StringJoin[]` → `""`). DoS-capped at the new `MAX_STRING_LENGTH`
  (the running total uses `checked_add`; an over-cap join stays unevaluated
  before any allocation).
- **`StringLength[s]`** — number of **characters**, not bytes
  (`StringLength["héllo"]` → `5`).
- **`StringTake[s, n]`** — first `n` chars (`n < 0` → last `|n|`); **`StringTake[s,
  {m, n}]`** — 1-based inclusive character range. `StringTake["hello", 3]` →
  `"hel"`, `StringTake["hello", {2, 4}]` → `"ell"`, `StringTake["hello", -2]` →
  `"lo"`.
- **`StringDrop[s, n]`** — drop the first `n` chars (`n < 0` → drop the last
  `|n|`). `StringDrop["hello", 2]` → `"llo"`.
- **`StringSplit[s]`** — split on runs of whitespace; **`StringSplit[s, sep]`** —
  split on a literal string separator. Both drop empty fields and return a `List`
  of strings. `StringSplit["a b  c"]` → `{"a","b","c"}`, `StringSplit["a,b,c",
  ","]` → `{"a","b","c"}`.
- **`StringReplace[s, a -> b]`** — replace **every** non-overlapping literal
  occurrence of `a` with `b`; accepts a single rule or a `{r1, r2, …}` list of
  rules applied in sequence. `StringReplace["banana", "a"->"o"]` → `"bonono"`.
- **`ToString[expr]`** — the Wolfram surface form of `expr` via the existing
  `print_wolfram` printer; a bare top-level string renders as its **raw content**
  (no quotes), so `ToString[123]` → `"123"` and `ToString["hi"]` → `"hi"`.
- **`Characters[s]`** — list of single-character strings (`Characters["ab"]` →
  `{"a","b"}`).

### Unicode by character, never by byte

Every length, index, and slice goes through `s.chars().count()` / a
`Vec<char>` — **no byte index is ever taken** — so a multi-byte character (`é`,
an emoji) counts as exactly one position and `StringTake`/`StringDrop` can never
slice through a UTF-8 boundary (the `byte index N is not a char boundary` panic
is structurally impossible). `StringLength["héllo"]` is `5`; `StringTake["héllo",
2]` is `"hé"`.

### Safety / DoS

- New **`MAX_STRING_LENGTH`** cap (mirrors `MAX_LIST_LENGTH` = 1,000,000) bounds
  the two string-*growing* heads, `StringJoin` and `StringReplace`.
- `StringReplace` rejects an **empty pattern** (`"" -> x`, which would match at
  every position — unbounded expansion) and scans **non-overlapping
  left-to-right** (so `"a" -> "aa"` does not re-scan the inserted text; linear,
  terminating). Its output length is bounded by `MAX_STRING_LENGTH`.
- `i64::MIN` indices are handled via an `i128` magnitude (no `i64::abs`
  overflow); out-of-range / non-integer / non-string inputs leave the form
  **unevaluated** rather than panicking — the W-5/W-9 fail-soft contract.

### Tests

28 new unit tests in `builtins.rs` (each head's happy path, the Unicode cases,
the DoS caps, and the malformed-input/unevaluated paths) plus 3 end-to-end tests
in `lib.rs` (full lex→lower→eval→print, Unicode, and a malformed-input
session-survival case). `cargo clippy` clean; all `wolfram-runtime` +
`wolfram-repl` tests green.

## [0.8.0] — 2026-06-19

The **W-11** deliverable (MA04 §14): Wolfram's **pure (anonymous) functions** —
`Function[…]`, the slot forms `#`/`#n`/`##`, and the `&` postfix — the single
most-used functional idiom, so a higher-order builtin can take an inline lambda
instead of a named definition. This is the first runtime change since W-5 to
require a **grammar + lexer change** (regenerated `_grammar.rs`, mirroring W-6).

### Added (pure functions)

- **`Function[x, body]` / `Function[{x, y}, body]`** — named-parameter pure
  functions. Applying substitutes the args for the named params in the body, via
  the **same `vm.rs::substitute`** user functions, the W-7 `Table` index, and W-8
  scoping already use. `Function[x, x^2][5]` → `25`; `Function[{x,y}, x+y][3,4]`
  → `7`. A single-symbol param is normalised to a one-element list at lowering,
  so every named function is uniformly `Function[List(params…), body]`.
- **Slot forms `#`, `#1`, `#2`, …** (`#` ≡ `#1`) lowering to `Slot[n]`, and
  **`##`** (`SlotSequence`) lowering to `SlotSequence[1]`. A `##` in an argument
  position **splices** all the call's args into that argument list.
- **The `&` postfix** (`(#^2)&`, `(#1+#2)&`) turning the preceding expression
  into a slot-based `Function[body]`. `&` has a **low precedence** — looser than
  every arithmetic/comparison operator but tighter than `,` — so `#^2 &`,
  `# + 1 &`, and `Mod[#,2]==0 &` are all pure functions of the *whole* body. A
  pure function may be applied immediately (`(#^2)&[5]`), and the apply suffix
  chains (`f&[1][2]`, `f&[[i]]`).
- **`Mod[a, b]`** — a minimal integer modulo (divisor-signed remainder), the
  only new builtin W-11 needs (for the canonical `Mod[#,2]==0 &` even-predicate).

### How it composes

Application is a **rewrite rule on `Backend::rules()`**: its predicate matches a
*reducible* `Function[…][args]` (well-formed record, matching arity) and the
transform substitutes args → params/slots and returns the body for the VM to
re-evaluate. Because the rule fires inside `vm.eval`, it composes for free with
every W-5/W-9/W-10 higher-order builtin — they already re-apply `f` through
`build_canonical_application` + `vm.eval`:

- `Map[#^2 &, {1, 2, 3}]` → `{1, 4, 9}`
- `Select[{1, 2, 3, 4}, Mod[#, 2] == 0 &]` → `{2, 4}`
- `Nest[# + 1 &, 0, 3]` → `3`

### Safety

Gating reducibility in the **predicate** (not the transform) is what prevents an
arity-mismatched / malformed `Function[…][args]` from re-matching the rule and
looping forever (a self-DoS) — a non-reducible form falls through to
`on_unknown_head` and stays unevaluated. A pure function substitutes its body
once per application (linear in the body size); self-referential recursion is
bounded by the evaluator's existing recursion handling exactly as a
self-referential `Define` is.

### Grammar (regenerated `_grammar.rs`)

New tokens `HASH` (`#`), `SLOTSEQ` (`##`, longest-match before `#`), `AMP` (`&`,
longest-match after `&&`); a `slot` atom; and a low-binding `amp` postfix level
(`amp = comparison AMP { AMP } { amp_apply } | comparison`). The `_grammar.rs`
for the lexer and parser were regenerated via the Rust grammar-tools CLI — never
hand-edited.

## [0.7.0] — 2026-06-17

The **W-10** deliverable (MA04 §13): the functional-iteration combinators — the
point-free heads every functional-programming session reaches for, lowered onto
the *same* substrate as W-5/W-9 (the `Map`/`Apply` application path
`build_canonical_application` + `vm.eval`, and the W-5 `list_elements` accessor).
All are plain `Head[args]` applications — **no grammar change** — and all are
eager (non-held), so the `WolframBackend` held set is untouched.

### Added (functional-iteration combinators)

- **`Nest[f, x, n]`** → `f` applied to `x` `n` times: `f[f[…f[x]…]]`. A symbolic
  `f` builds the literal nest (`Nest[f, x, 3]` → `f[f[f[x]]]`); a defined `f`
  reduces at each step. `Nest[f, x, 0]` is the identity (`x`).
- **`NestList[f, x, n]`** → `{x, f[x], f[f[x]], …}` — the `n + 1` intermediate
  results, including the seed.
- **`Fold[f, x0, list]`** → the left fold `f[…f[f[x0, l₁], l₂]…, lₙ]`. With
  `Plus` it totals (`Fold[Plus, 0, {1,2,3}]` → `6`); left-associative
  (`Fold[Subtract, 10, {1,2,3}]` → `4`). An empty list returns the seed.
- **`FoldList[f, x0, list]`** → `{x0, f[x0,l₁], f[f[x0,l₁],l₂], …}` — the running
  accumulations, including the seed (`FoldList[Plus, 0, {1,2,3}]` → `{0,1,3,6}`).
  An empty list returns `{x0}`.

Each combinator re-applies `f` through the **exact** `Map`/`Apply` path
(`build_canonical_application(f, args)` then `vm.eval`), so any callable resolves:
a built-in (`Plus`), a bridged head, or a user `SetDelayed` function
(`g[a_] := a + 1; NestList[g, 0, 3]` → `{0,1,2,3}`). A non-callable `f` is *not*
an error — each `f[acc]` simply stays unevaluated (`Fold[f, 0, {1,2}]` →
`f[f[0,1],2]`).

### Security / DoS

- **Iteration count `n` is capped** (`Nest`/`NestList`): `nest_count` reads `n` as
  an exact non-negative integer and refuses any `n` exceeding `MAX_LIST_LENGTH`
  (1,000,000) *before* the loop, so a tiny input like `Nest[f, x, 10^9]` cannot
  drive a billion `vm.eval` calls.
- **Result-list size is bounded**: `NestList`'s `n + 1` allocation is bounded by
  the capped `n`; `FoldList`'s `len + 1` allocation is bounded by a defensive
  `MAX_LIST_LENGTH` check on the (already source-bounded) input length. `Nest` and
  `Fold` hold only the scalar accumulator and add no result-size surface.
- Every malformed form (negative/non-integer `n`, an over-cap `n`, a non-list
  third argument to `Fold`/`FoldList`, the wrong arity) is **left unevaluated** —
  echoed back, never a panic — following the W-5 convention.

### Tests

26 new tests (14 unit in `builtins.rs`, 7 integration through the public
`eval`/`WolframSession` surface, plus edge/DoS/regression cases): the symbolic
`Nest`/`NestList` shapes, `Fold`/`FoldList` over `Plus`/`Subtract`, the degenerate
`n = 0` / empty-list cases, a user `SetDelayed` function as `f`, negative /
non-integer / over-cap `n`, non-list fold target, wrong arity, non-callable `f`,
and W-4..W-9 regression guards.

## [0.6.0] — 2026-06-17

The **W-9** deliverable (MA04 §12): list-manipulation builtins — the reordering,
concatenating, flattening, filtering, counting, and summing heads every
list-processing session reaches for. Lowered onto the *same* substrate as W-5
(the `list_elements` accessor, the `Map`/`Apply` predicate-application path, and
the canonical `Add` fold). All are plain `Head[args]` applications — **no grammar
change** — and all are eager (non-held), so the `WolframBackend` held set is
untouched.

### Added (list-manipulation heads)

- **`Sort[list]`** → ascending in the subset's documented total canonical order
  (`canonical_cmp`): numbers (by `f64` magnitude) < symbols < strings < compound
  expressions; total and stable, so it never panics and is reproducible across
  runs. Pure-numeric lists sort numerically (`Sort[{3, 1, 2}]` → `{1, 2, 3}`).
- **`Reverse[list]`** → the list reversed.
- **`Join[a, b, …]`** → two or more lists concatenated. The combined length is
  capped at `MAX_LIST_LENGTH` (checked with `checked_add` before allocating); a
  non-list argument leaves the form unevaluated.
- **`Flatten[list]`** → every nested sub-list spliced in at **all** levels;
  **`Flatten[list, n]`** → only the top `n` levels of nested sub-lists. Output
  length capped at `MAX_LIST_LENGTH`, recursion bounded by the (token-capped)
  input nesting. A negative/non-integer depth, or a non-list, stays unevaluated.
- **`Select[list, pred]`** / **`Count[list, pred]`** → keep / tally elements where
  `pred[e]` evaluates to the `True` symbol. The predicate is applied through the
  **same** path as `Map`/`Apply` (`build_canonical_application` + `vm.eval`), so a
  built-in predicate, a user `SetDelayed` function, or a bridged head all work.
  Function-predicate `Count` is the documented simplification versus full Wolfram
  pattern-matching `Count` (MA04 §12.3).
- **`Total[list]`** → the sum of the elements, folded onto the canonical `Add`
  head (consistent with W-7 `Sum` over a range); an empty list totals to `0`.

### Added (parity predicates)

- **`EvenQ[n]`** / **`OddQ[n]`** → `True`/`False` on integer parity (so
  `Select`/`Count` are testable; the W-5/W-6 surface had no predicate head).
  `rem_euclid(2)` classifies negatives correctly; a non-integer argument is
  `False` (matching Wolfram), wrong arity stays unevaluated.

### Safety / DoS (MA04 §12.4)

- `Join`/`Flatten` outputs are bounded by `MAX_LIST_LENGTH` (= `MAX_RANGE_LENGTH`,
  1,000,000), checked before allocation; `Flatten` recursion is depth-bounded.
- The size-non-increasing heads (`Sort`, `Reverse`, `Select`, `Count`, `Total`)
  add no new allocation source — their output is at most the source-bounded input.
- Every malformed form (non-list arg, non-callable predicate, bad depth, wrong
  arity) is **left unevaluated** — echoed back, never a panic — per the W-5
  convention.

### Tests

- Unit tests for each head over a real VM, plus the malformed/edge cases
  (oversize/negative depth, non-list, unbound predicate, extreme parity).
  `Select`/`Count` predicate tests run over a real `WolframBackend` so `EvenQ`
  resolves.
- End-to-end integration tests through `eval`/`WolframSession` for every
  acceptance example in the brief, a user-defined predicate, and a regression
  guard that W-4..W-8 behaviour is unchanged.

## [0.5.0] — 2026-06-17

The **W-8** deliverable (MA04 §11): local scoping — the three Wolfram heads that
bind named locals over a body. `With`, `Module`, and `Block` are lowered onto the
*same* substrate as W-7's iteration index: held heads + the `vm.rs::substitute`
primitive. No new evaluator, no opcode, no grammar change.

### Added (local-scoping heads)

- **`With[{x = e, …}, body]`** → `body` with each local bound to its **evaluated**
  RHS, substituted in and re-evaluated. Lexical and immediate, parallel binding
  (each RHS sees the surrounding scope, so a decl may reference an outer binding).
  So `With[{x = 3}, x^2]` is `9` and `With[{a = 1, b = 2}, a + b]` is `3`.
- **`Module[{x, y = e}, body]`** → lexically-scoped locals. An initialised decl
  (`y = e`) binds like `With`; an **uninitialised** decl (`x`) is α-renamed to a
  fresh gensym `x$nnn` (mirroring real Wolfram) so it stays undefined and cannot
  resolve to — or be captured by — a same-named global. `Module[{a = 1, b = 2},
  a + b]` is `3`.
- **`Block[{x = e}, body]`** → temporarily binds `x` over `body`. For the
  substitution-based subset a self-contained body is observably identical to
  `With`; `Block[{x = 5}, x + 1]` is `6`. (See §11.3 for the dynamic-scope
  simplification.)

### Binding mechanism (MA04 §11.2–§11.3)

- The three heads are **held** (added to the `WolframBackend` decorator's
  `hold_heads` set, union with the inner held set and W-7's iteration heads) so
  the declaration list and body arrive unevaluated.
- Each decl's RHS is evaluated through `vm.eval`; the collected `name → value`
  mapping is applied to a **copy** of the held body via the same `substitute`
  used for user-function parameters and the W-7 index, then the result is
  evaluated. Because the session environment is never mutated, **locals do not
  leak** (`x` is still free after `With[{x = 3}, x]`) and never clobber a global.
- Uninitialised `Module` locals are gensym-renamed (a monotonic `AtomicU64`
  counter) — the documented capture-avoidance simplification in place of full
  α-renaming of every local.

### Robustness (MA04 §11.4)

- Malformed forms are left **unevaluated**, never a panic: a non-`List` first
  argument (`With[x, body]`), a `With`/`Block` local with no value
  (`With[{x}, body]`), a non-symbol assignment target (`f[x] = 1`), or the wrong
  arity. No new allocation source — the body is substituted once per scope entry,
  bounded by the W-4 input/token caps; nested scopes recurse over strictly
  smaller bodies.

### Tests

- W-8 acceptance values; no-leak and no-clobber guards; nested scoping; a decl
  referring to an outer binding; the gensym shadow of a global by an
  uninitialised `Module` local; and the malformed-form / wrong-arity guards.

## [0.4.0] — 2026-06-17

The **W-7** deliverable (MA04 §10): iteration constructs — the first Wolfram-lane
forms that introduce a *scoped local index*. `Table`, `Do`, `Sum`, and `Product`
bind a fresh variable `i` to each value of a range and evaluate a body once per
value, lowered onto the *same* `symbolic-vm` substrate (no bespoke loop opcode,
no new evaluator).

### Added (iteration heads)

- **`Table[expr, {i, imax}]`** / **`{i, imin, imax}`** / **`{i, imin, imax, di}`**
  → the list of `expr` evaluated with `i` bound over the range. So
  `Table[i^2, {i, 3}]` is `{1, 4, 9}` and `Table[i, {i, 2, 4}]` is `{2, 3, 4}`.
- **`Do[expr, {i, n}]`** → evaluate `expr` `n` times for side effects (e.g. a
  `Set` in the body), returning `Null`.
- **`Sum[expr, {i, imin, imax}]`** → fold `+` over the range
  (`Sum[i, {i, 1, 10}]` is `55`); an empty range sums to `0`.
- **`Product[expr, {i, imin, imax}]`** → fold `×`
  (`Product[i, {i, 1, 4}]` is `24`); an empty range is `1`.

### How the index binds

- The four heads are **held** — `WolframBackend::hold_heads` now returns the
  union of the inner `SymbolicBackend` held set (`If`, `Assign`, `Define`, …) and
  `{Table, Do, Sum, Product}`, so the body and iterator spec arrive unevaluated.
- Each iteration binds `i → value` with the **same `vm.rs::substitute`** that
  binds user-function parameters, then re-evaluates the body through the VM. The
  index stays *local* (it never leaks into the session), and nested `Table`s each
  bind their own index cleanly.
- The iterator-spec *bounds* are evaluated by the handler (the head is held, so
  `{i, 1+1}` and `{i, n}`-with-`n`-bound resolve correctly), while the body
  stays held until substitution.

### DoS surface

- The per-iteration count is **capped at `MAX_RANGE_LENGTH`** (the same bound
  `Range` uses), computed in `i128` *before* any allocation or looping — an
  oversize or extreme-span iterator (e.g. `Table[0, {i, 2000000}]`) is left
  unevaluated rather than hanging or exhausting memory. `Do` is capped
  identically (the cap bounds wall-clock work, not just memory), and the cap
  composes for nested `Table`. A malformed spec (`{i}` with no bound, a zero
  step, a non-integer/non-symbol binder, or a non-list spec) stays unevaluated —
  never a panic. See MA04 §10.3.

### Notes

- No grammar/lexer change: `Table[…]`/`Do[…]`/`Sum[…]`/`Product[…]` are ordinary
  `Head[args]` applications over list-literal specs the W-1 grammar already
  parses. W-7 touches only `wolfram-runtime` (`builtins.rs` + `backend.rs`).
- `Sum`/`Product` fold onto the canonical `Add`/`Mul` IR heads, so symbolic terms
  combine through the same engine as `1 + 2` (a symbolic body like
  `Sum[x, {i, 1, 3}]` yields `x + x + x`, the engine doing no further `3x`
  normalisation — consistent with W-4 behaviour).

## [0.3.0] — 2026-06-17

The **W-6** deliverable (MA04 §9): operator sugar for the W-5 Tier-1 heads. No
new evaluation logic and no new handler — each sugar form desugars in lowering
to the exact same head the W-5 built-in table already answers, so the sugar and
its head form produce byte-identical IR.

### Added (operator sugar)

- **`f /@ x` ≡ `Map[f, x]`** — lowered by the new `lower_mapapply` over the
  parser's `mapapply` rule.
- **`f @@ x` ≡ `Apply[f, x]`** — same path; `/@` and `@@` share one
  left-associative precedence level (`g @@ f /@ x` ⇒ `Map[Apply[g, f], x]` —
  parenthesise when mixing).
- **`x[[i]]` ≡ `Part[x, i]`** — `lower_postfix` gains an `LDBRACKET` arm that
  emits `Part`; a multi-index `x[[i, j]]` folds into nested parts
  `Part[Part[x, i], j]`, and `[[ ]]` chains/interleaves with `f[…]` application
  (`x[[1]][[2]]`, `f[x][[1]]`, `Range[3][[2]]`).

So `Plus @@ {1, 2, 3}` is `6`, `f /@ {1, 2}` is `{f[1], f[2]}`,
`{a, b, c}[[2]]` is `b`, and `{{1,2},{3,4}}[[1]][[2]]` is `2`, each identical to
its long head form. Negative/out-of-range `Part` and the `Map`/`Apply`
re-evaluation behaviour carry over from W-5 unchanged.

### Notes

- `Map`/`Apply`/`Part` are **not** run through the `Plus`→`Add`-style
  `canonical_head` bridge (they are not arithmetic heads), so they reach the
  `WolframBackend` decorator handler table verbatim.
- No new DoS surface: `/@`/`@@` inherit `Map`/`Apply`'s bounds (the
  already-materialised list); `[[ ]]` only reads one element; deep `[[…]]`
  chains are parsed iteratively (bounded by the W-4 per-statement token cap), not
  by grammar recursion. See MA04 §9.4.

## [0.2.0] — 2026-06-17

The **W-5** deliverable (MA04 §8): more built-ins & evaluation, layered onto the
*same* symbolic substrate W-4 uses — no bespoke evaluator, and no edit to
`symbolic-vm`'s shared handler table.

### Added

- **`WolframBackend`** (`backend` module) — a decorator over the shared
  `SymbolicBackend`. It answers `handler_for` from a small W-5 built-in table and
  delegates everything else (`lookup`/`bind`/`on_unresolved`/`on_unknown_head`/
  `rules`/`hold_heads`, and every W-4 handler) to the inner backend. This keeps
  the new surface local to the Wolfram lane while reusing the entire evaluation
  engine, the `Plus`→`Add` bridge, user-defined functions, and `/.`.
- **List/functional/control/numeric built-ins** (`builtins` module):
  - `Length[{…}]` — element count (`0` for an atom; argument count for a non-list
    head).
  - `First` / `Last` — first/last element; **empty list left unevaluated** (no
    panic).
  - `Part[expr, i]` — **1-based** indexing; `i = 0` is the head; negative `i`
    counts from the end; out-of-range / non-integer index left unevaluated.
  - `Append[{…}, x]` — a new list with `x` appended (values are immutable).
  - `Range[n]` / `Range[a, b]` / `Range[a, b, d]` — integer ranges, **DoS-capped**
    at `MAX_RANGE_LENGTH` (1,000,000) elements *before* allocation, so a tiny
    `Range[10^9]` is left unevaluated rather than exhausting memory.
  - `Map[f, {…}]` and `Apply[f, {…}]` — re-evaluate the constructed `f[…]` through
    the VM, routing the head through the same canonical bridge as W-4 lowering
    (`build_canonical_application`), so `Apply[Plus, {1, 2, 3}]` folds to `6`.
  - `N[expr]` — coerce exact `Integer`/`Rational` to `Float`, mapping over a list
    element-wise; symbolic and already-float values pass through.
- `MAX_RANGE_LENGTH` is re-exported.
- **`If` and the comparison/logical heads** (`==`, `!=`, `<`, `>`, `<=`, `>=`,
  `&&`, `||`, `!`) already evaluated through the shared backend in W-4; W-5 pins
  them with end-to-end tests.

### Notes

- No grammar/lexer change: every W-5 head is a function-call form the existing
  `head[args]` grammar already parses. The operator *sugar* (`/@` Map, `@@` Apply,
  `[[ ]]` Part) is deferred to W-6 (MA04 §2/§4).
- All new built-ins run inside the existing W-4 worker-thread `catch_unwind`, so
  an unforeseen handler panic still becomes a clean `Err` and the session is
  rebuilt.

## [0.1.0] — 2026-06-17

Initial release — the **W-4** deliverable of the Wolfram-language lane (MA04 §7).

### Added

- `WolframSession` — a persistent, string-in / string-out runtime that lowers the
  parsed M-expression `GrammarASTNode` from `wolfram-parser` to `symbolic-ir` and
  evaluates it with `symbolic-vm`'s `SymbolicBackend`. Variable bindings (`x = 5`)
  and user-defined functions (`f[x_] := x^2`) persist across `feed` calls; the
  `Out[n]` counter persists too.
- `WolframSession::feed` (string echo) and `eval_to_outputs` (structured
  `Output`s), plus a one-shot `eval` helper.
- **Lowering** (`lower` module): the surface→IR desugaring. The head-name bridge
  maps both the infix operators and the explicit head-applications
  (`Plus`/`Times`/`Power`/`Subtract`/`Divide`/`Minus`/`Equal`/`And`/…) onto the
  canonical IR heads (`Add`/`Mul`/`Pow`/`Sub`/`Div`/`Neg`/`Equal`/`And`/…), so
  `1 + 2` and `Plus[1, 2]` evaluate identically. n-ary `Plus`/`Times` are
  left-folded into binary chains the VM folds. `Set`→`Assign`, `SetDelayed`→
  `Define` (with `x_` parameters reduced to the bound symbol for the VM's
  symbol-based parameter binding). Pattern blanks (`_`, `x_`, `_h`, `x_h`) and
  rules (`->`, `:>`) lower to the `cas-pattern-matching` `Blank`/`Pattern`/`Rule`/
  `RuleDelayed` node shapes.
- **ReplaceAll** (`/.`): a synthetic `ReplaceAll` head is intercepted before VM
  evaluation and dispatched through `cas-pattern-matching::rewrite`. A rule's RHS
  bare references to LHS-bound pattern names are rewritten into the
  `Pattern(name, Blank())` reference form the matcher's `substitute` understands.
  Supports a single rule or a `List` of rules.
- **Pretty-printing** (`printer` module): renders the evaluated IR back to Wolfram
  surface notation (infix operators, `f[…]` application, `{…}` lists), with
  precedence-aware parenthesisation so the output re-parses to the same tree.
- **Trust-boundary hardening**, mirroring `maxima-runtime`: `MAX_INPUT_LEN` (64
  KiB) input cap; `MAX_STATEMENT_TOKENS` per-statement token cap measured on the
  real `wolfram-lexer` token stream (bounding parse-tree depth so deep nesting
  cannot overflow the stack on build or drop); evaluation on a bounded
  large-stack worker thread inside `catch_unwind`, with full session rebuild after
  any caught panic. `MAX_REWRITE_ITERATIONS` bounds `/.` rewriting.

### Notes

- Scope is the W-1 grammar subset (MA04 §4): explicit `*` required (no
  juxtaposition multiplication), no `[[…]]`/`;;`/`@`/`&`/`#` etc. `Simplify`/
  `Expand` and the full `cas-*` surface are W-6.
- Built on `symbolic-ir` 0.2, `symbolic-vm` 0.20, `cas-pattern-matching` 0.1.
