# coding-adventures-idl-runtime

A tree-walking evaluator for [IDL](https://en.wikipedia.org/wiki/IDL_(programming_language))
(Interactive Data Language), the science/astronomy array language, over
[`array-runtime`](../array-runtime). Item **MA-12d** of the IDL frontend
(spec [`MA12`](../../../specs/MA12-idl-language.md)): the piece that makes
the IDL lexer/parser (`idl-lexer`/`idl-parser`, MA-12b/MA-12c) executable.

## Why IDL needed a keyword-aware call layer, not just a reused scope frame

IDL is the first language in this repo's array family whose call sites are
not purely positional and whose callables come in two kinds:

- A **function** is called in expression position, parenthesised, and
  returns a value: `y = SIN(x)`.
- A **procedure** is called in statement position, command-style, with no
  parentheses and no return value: `PRINT, x`.
- Both kinds accept **keyword arguments** mixed freely with positional ones
  (`PLOT, x, TITLE='flux', COLOR=255`) and the `/BOOLEAN` shorthand
  (`/YLOG` == `YLOG=1`).

MA12 §3 is explicit that the *definition* side of this — a named callable
with declared parameters, a multi-statement body, and a call scope frame —
is "the same shape Q's `QFn::Lambda` already established" (MA11 §2), so
this crate does not re-derive that base mechanism from scratch. What is
genuinely new, layered on top of it, is:

1. **Two separate namespaces.** Real IDL allows the same name to be both a
   `PRO` and a `FUNCTION` simultaneously; `idl-parser`'s own CST already
   distinguishes the two call sites (`procedure_call_stmt` vs. an
   expression's `call_suffix`), so this evaluator keeps two dispatch
   tables (`procs`/`funcs`) and routes each call site to its own table.
2. **Keyword-argument binding.** A call's mixed positional/keyword
   argument list binds to a callable's declared positional/keyword
   parameters — positional by index, keyword by name — with an omitted
   parameter (positional or keyword) left **genuinely unbound**, not
   defaulted to a sentinel (MA12 §3's own load-bearing rule, since real
   IDL's `N_ELEMENTS(kw) EQ 0` idiom tests exactly this).
3. **No automatic outer/global visibility inside a call.** Unlike Q (whose
   lambda falls back to the *global* frame for any non-parameter name),
   MA12 §4 defers `COMMON` blocks and states plainly that a routine's body
   "read[s]/write[s] only their parameters, keywords, and locals" — so a
   call gets a brand-new, *isolated* frame, never one stacked with
   fallback onto the caller's own locals.

See `eval.rs`'s own module doc comment for the full design and every
smaller decision (the `IdlCallable`/`ParamSpec` shapes, the `Flow` signal
threading `BREAK`/`CONTINUE`/`RETURN` up through nested control flow, and
exactly why `lookup`/`assign` only ever touch the environment stack's top
frame).

```rust
use coding_adventures_idl_runtime::Interpreter;

let interp = Interpreter::new();

// Assignment is silent; a bare expression auto-prints (Implied Print,
// confirmed directly against NV5 Geospatial's own documentation).
assert_eq!(interp.feed("x = 5\n").unwrap(), "");
assert_eq!(interp.feed("x\n").unwrap().trim(), "5");

let prog = "\
FUNCTION scaled, x, FACTOR=factor\n\
 IF N_ELEMENTS(factor) EQ 0 THEN factor = 1\n\
 RETURN, x * factor\n\
END\n\
PRINT, scaled(5)\n\
PRINT, scaled(5, FACTOR=3)\n\
";
let out = interp.feed(prog).unwrap();
assert!(out.contains('5'));
assert!(out.contains("15"));
```

## What it evaluates (MA12 §4's in-scope surface)

- **Arithmetic/comparison/logical expressions**, respecting `idl-parser`'s
  own confirmed precedence (unary `+`/`-`/`NOT` at the SAME tier as binary
  `+`/`-`, not tighter; `^` left-associative, not right).
- **Control flow**: `IF...THEN...ELSE` (both single-statement and
  `BEGIN...ENDIF/ENDELSE/END` block forms), `FOR v=init,limit[,step] DO`,
  `WHILE expr DO`, `REPEAT...UNTIL`, `BREAK`, `CONTINUE`, `RETURN` (with or
  without a value, validated against the enclosing routine's own kind).
- **Assignment**, including subscripted targets (`a[i] = expr`,
  `a[i,j] = expr`, range/wildcard targets), read/written against
  `array-runtime`'s own storage.
- **The full subscript surface**: plain, 2-D, ranged (`a[s0:s1]`,
  **inclusive of both endpoints**, confirmed directly against NV5
  Geospatial's own *Array Subscript Ranges* reference), strided
  (`a[s0:s1:n]`), wildcard (`a[*]`, `a[s0:*]`), and negative-from-end
  (`a[-1]`).
- **Array literals** (`[1, 2, 3]`) — always a genuine rank-1 array, even a
  one-element literal (MA12 §2: an IDL scalar is genuinely rank-0, distinct
  from a 1-element array).
- **`PRO`/`FUNCTION`** definitions with positional and keyword parameters
  (`KEYWORD=local_var_name`), command-syntax procedure calls, parenthesised
  function calls, keyword arguments, and the `/BOOLEAN` shorthand.
- **Strings** (`IdlValue::Str`): assignment, `PRINT`, `EQ`/`NE` equality,
  and keyword/positional argument values — MA12 §2's own scoped surface
  (no other string operators, no string arrays this cut).
- A small, **explicitly scoped** builtin surface — see `builtins.rs`'s own
  module doc comment for the exact list and the reasoning behind it:
  `PRINT`; `SIN`/`COS`/`TAN`/`SQRT`/`ABS`/`EXP`/`ALOG`/`ALOG10`;
  `INDGEN`/`FINDGEN`/`DINDGEN`/`LINDGEN`;
  `INTARR`/`FLTARR`/`DBLARR`/`LONARR`; `TOTAL`/`MIN`/`MAX`/`N_ELEMENTS`/
  `SIZE` (four modes); `TRANSPOSE`.

### Deferred (MA12 §4, unchanged here)

Structures, pointers, objects, `LIST`/`HASH`, `COMMON` blocks,
`_EXTRA`/`_REF_EXTRA` keyword inheritance, `CASE`/`SWITCH`/`FOREACH`, IDL's
typed numeric tower (every value is `f64`), and the wider intrinsic
library/graphics.

## Design decisions this crate had to make (and their justification)

### Case folding: identifiers are folded to uppercase

`idl-lexer`'s own README explicitly left this as an open MA-12d decision.
This crate decides **yes** — and verifies it, rather than guessing: NV5
Geospatial's own support documentation states directly that IDL "converts
procedure names to uppercase internally," and real IDL is documented as
case-insensitive for its whole language surface (variable names, routine
names). Every identifier — variable, `PRO`/`FUNCTION`, parameter/keyword
name — is folded via `eval::fold_case` (`str::to_uppercase`) at the point
it is first read off a `NAME` token, so `myVar`/`MYVAR`/`MyVar` are one
binding. See `eval.rs`'s own module doc comment for the full citation.

### Display/`PRINT` convention: verified facts vs. a flagged judgment call

Checked directly against NV5 Geospatial's *PRINT/PRINTF*, *Output of IDL
Variables*, and *Implied Print* reference pages this session (not assumed
from a sibling language's own convention):

- **Verified**: assignment produces no output; a bare, non-assignment
  top-level statement auto-prints via IDL's own *Implied Print* feature,
  which does **not** fire inside a routine body; array subscript ranges
  are inclusive of both endpoints.
- **Flagged as a judgment call**: the official docs explicitly defer
  default (non-`FORMAT`) numeric column width/precision to the platform's
  own `sprintf`, and do not spell out an exact byte-for-byte scheme (they
  also note `PRINT` and Implied Print use genuinely *different* default
  precision from each other, a distinction this cut does not reproduce).
  Rather than fabricate specifics this session could not verify, `value.rs`
  adopts the same "clean numeric echo" convention this repo's other
  array-family runtimes already use (plain ASCII `-`, no trailing `.0` for
  whole values, space-separated vectors, row-per-line right-aligned
  matrices) — see `value.rs`'s own `display` doc comment for the full
  breakdown.

### `AND`/`OR`/`XOR`/`NOT` are bitwise, not logical — a documented IDL gotcha, faithfully reproduced

MA12 §4 itself spells these "logical/bitwise." Checked directly against
NV5 Geospatial's *Bitwise Operators*/*Logical vs. Bitwise Operators* pages:
these four ARE the bitwise family (operating on an integer representation
of each operand); the genuinely short-circuit logical operators (`&&`,
`||`, `~`) are a different, explicitly out-of-scope family (MA12 §4). One
real consequence: `NOT 0` is `-1` and `NOT 1` is `-2` — **both** nonzero —
so bitwise `NOT` cannot invert a comparison's truthiness the way a logical
negation would. This is faithfully reproduced, not "fixed."

### `#`/`##` matrix product operand order — a flagged, moderate-confidence judgment call

`##` is confirmed (multiple sources) as IDL's standard/conventional matrix
product, mapped directly onto `array_runtime::ops::matmul(a, b)`. `#` is
documented as the reversed/column-oriented product ("the opposite of
normal matrix multiplication," with the resulting shape `[nrows(B),
ncols(A)]`) — derived here as `matmul(b, a)` (operand-swapped), which
matches that documented shape rule exactly. This was **not** independently
re-verified against a primary IDL source's own worked numeric example in
this session — flagged in `eval.rs`'s own comment for a later item to
confirm.

### 2-D subscript axis mapping (`a[i, j]`) — MA12 §2's own flagged, unresolved question

MA12 §2 explicitly leaves "does IDL's `a[i,j]` map to `array-runtime`'s
element `(i,j)` or `(j,i)`" as "a concrete lowering decision... confirmed
empirically... before relying on it." This crate maps the first subscript
to `array-runtime`'s column axis and the second to its row axis (the
literal reading of IDL's own documented `[column, row]` order) — **not**
independently re-verified against a real IDL session's `PRINT` output in
this session (none was available). Flagged directly in `eval.rs`'s own
`resolve_subscripts` doc comment for a later item to confirm.

## DoS guards

- An evaluator recursion-depth guard (`eval.rs::MAX_DEPTH`, 500) around
  every recursive `eval_*`/`exec_*`/call entry point — a **disclosed
  simplification**: unlike `q_runtime::eval::MAX_DEPTH` (empirically
  measured via binary search against a known native-stack floor), this
  constant is a reasonable, conservative default, not independently
  re-measured for this evaluator's own (different) recursion shape. See
  that constant's own doc comment.
- `builtins::MAX_ARRAY_LENGTH` (1,000,000) bounds every array construction
  (`*ARR`/`*INDGEN`), array literal, and subscript-range materialization
  whose size is runtime-computed, checked *before* allocating.

## Testing

```sh
cargo test -p coding-adventures-idl-runtime
```
