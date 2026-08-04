# coding-adventures-idl-parser

IDL (Interactive Data Language) parser backed by
`code/grammars/idl/idl.grammar`, compiled to Rust and statically linked into
the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Where this fits

This is the second crate of IDL's frontend — MA-12c from
[`MA12-idl-language.md`](../../../specs/MA12-idl-language.md), the spec that
fixed IDL's one genuinely new grammar/evaluator problem (a keyword-argument
calling convention over a procedure/function split, §3) before any lexer or
parser code existed. The crate layout mirrors MA12 §6's rollout:
`idl-lexer` (MA-12b, merged) → `idl-parser` (this crate, MA-12c) →
`idl-runtime`/`idl-repl` (MA-12d) → `idl-to-semantic-ir` (MA-12e).

Unlike every array-family frontend already in this repo (APL/J/Q/Scilab/
MATLAB), IDL's *surface* is an Algol/Fortran-family imperative grammar —
statements, `PRO`/`FUNCTION` definitions, `IF`/`FOR`/`WHILE`/`REPEAT` blocks,
an infix operator-precedence cascade with word operators (`EQ`/`AND`/...) —
closer in shape to this repo's `algol-parser`/`dartmouth-basic-parser` than
to any array-family parser (MA12 §5). So `idl.grammar` is written to IDL's
own imperative shape, not forked from an array sibling (contrast
`scilab-parser`, forked from `matlab.grammar` at the grammar-source level).

## Scope

Covers the MA12 §4-scoped first cut: `PRO`/`FUNCTION` definitions (both
terminated by the generic `END`), the procedure-call statement and
parenthesised function calls with positional, keyword (`KEYWORD=value`), and
boolean-shorthand (`/KEYWORD`) arguments, `IF...THEN...ELSE` (single-
statement and `BEGIN...ENDIF/ENDELSE/END` block forms), `FOR v=lo,hi[,step]
DO`, `WHILE expr DO`, `REPEAT...UNTIL`, `BREAK`/`CONTINUE`, a generic
`BEGIN...END` block, `RETURN`/`RETURN, expr`, assignment (including
subscripted targets), the full expression precedence cascade (arithmetic,
matrix product `#`/`##`, word comparison/logical operators), array literals,
and the full subscript surface (plain, 2-D, ranged, strided, `*`-wildcard,
negative-from-end).

## The two genuinely new disambiguations (MA12 §3)

`idl-lexer` (MA-12b) deliberately left two glyphs lexically unconditional —
`SLASH` is always plain division, `EQUALS` is always one plain `=` — because
MA12 §3 frames both as parser-level, grammar-position concerns: telling them
apart requires knowing which *production* is currently being parsed, not
anything visible in raw characters or a flat token list. This crate resolves
both entirely through grammar structure, with **no lookahead predicate of
any kind**.

### 1. `/BOOLEAN` keyword shorthand vs. division

```
arg = keyword_arg | bool_keyword_arg | expr ;
bool_keyword_arg = SLASH NAME ;
```

`arg` is the only production anywhere in this grammar that references a
bare, argument-leading `SLASH`. This works with no predicate at all, for a
structural reason: nowhere in `expr`'s own precedence cascade does `SLASH`
ever appear as anything but a **binary** operator (`multiplicative`'s own
repetition), consumed only *after* a left operand is already parsed — IDL
has no unary `/`. So `expr` can never itself succeed starting exactly on a
`SLASH` token. At an argument position:

- `PLOT, x, /YLOG` — the current token at that arg's slot IS a `SLASH`.
  `expr` cannot match here at all, so the only alternative that can ever
  succeed is `bool_keyword_arg`, unambiguously the boolean-keyword shorthand.
- `x = a/YLOG` — this is `assignment_stmt`'s RHS `expr`, not an argument
  position at all; `multiplicative` consumes the `SLASH` as ordinary
  division after `a` is already parsed, exactly like `a + b`.
- `PLOT, x, a/YLOG` — a positional argument that is *itself* a division: the
  current token at that arg's slot is NAME `a`, not `SLASH`, so `arg`'s
  `expr` alternative matches and `multiplicative` handles the division the
  same way. `bool_keyword_arg` is never even attempted (its own leading
  token isn't present).

Confirmed by this crate's own tests: `slash_boolean_keyword_shorthand_inside_a_procedure_call`,
`slash_is_ordinary_division_in_an_assignment_rhs`,
`slash_is_ordinary_division_as_a_positional_call_argument`, and
`boolean_keyword_shorthand_in_a_function_call_too` (the same `arg`/`arg_list`
production is shared, unmodified, between the command-style procedure call
and a function call's parenthesised argument list — MA12 §3 item 2's "mixed
freely...in both").

### 2. `=` as assignment vs. keyword-bind

```
assignment_stmt = NAME [ index_suffix ] EQUALS expr ;   -- statement-level
keyword_arg     = NAME EQUALS expr ;                     -- inside arg_list
```

Both productions have the identical `NAME EQUALS expr` shape around the
`=` — genuinely indistinguishable in isolation, exactly as MA12 §3 says.
What tells them apart is which rule the parser is *currently inside*:
`assignment_stmt` is only ever reached from `statement` (never from inside
an argument list), and `keyword_arg` is only ever reached from `arg` (only
ever inside an argument list). No shared "read `=` and decide later"
production exists — the grammar's own call graph is the disambiguation.

Confirmed by `equals_is_keyword_bind_inside_a_call_argument_list` and
`equals_is_ordinary_assignment_at_statement_level`.

### The procedure-call statement needs no special lookahead either

`statement`'s three NAME-led alternatives — `procedure_call_stmt`
(`NAME COMMA arg_list`), `assignment_stmt` (`NAME [index_suffix] EQUALS
expr`), `expr_stmt` (`expr`) — are ordinary PEG ordered choice with packrat
backtracking. `COMMA` is not used as a statement-level separator anywhere
else in this cut (IDL's own separator is `&`, per `idl.tokens`), so
`NAME COMMA` at a statement boundary is unambiguously the procedure-call
shape.

**A disclosed, spec-consistent scope note**: a zero-argument procedure call
(real IDL: a bare `STOP`) is syntactically identical, at the CST level, to a
bare-variable-reference expression statement — both are just a lone `NAME`.
MA12 §3 itself frames the procedure-call-statement production as requiring
the comma+arg-list shape, so this grammar does not invent a zero-arg
`procedure_call_stmt` alternative to paper over the gap; a bare `NAME`
statement parses via `expr_stmt` either way, and resolving "is this a known
zero-arg procedure or a variable to auto-display" is deferred to a future
`idl-runtime`'s symbol table (MA-12d) — mirroring MA12 §1's own finding that
some "is this a call or a value" questions in IDL are answerable only with a
symbol table, not grammar alone (pre-5.0 IDL's `fish(5)` ambiguity).
Confirmed by `bare_name_with_no_comma_is_an_expr_statement_not_a_call`.

## Operator precedence — verified against the official reference, not guessed

`idl.grammar`'s expression cascade was checked against the NV5/L3Harris
*Operator Precedence* reference page (and a verbatim IRYA/UNAM mirror of the
same table) rather than assumed from Scilab's/MATLAB's own cascade shape.
Two genuinely non-obvious, confirmed facts it encodes:

1. **Unary `+`/`-`/`NOT` sit at the SAME documented precedence tier as
   BINARY `+`/`-`**, not at a tighter tier above `multiplicative` the way
   Scilab's/MATLAB's own `unary` sits. `unary` in this grammar therefore
   recurses *above* `multiplicative` (looser), so a unary prefix wraps the
   entire multiplicative term that follows it: `-a*b` parses as `-(a*b)`,
   confirmed by `unary_minus_binds_looser_than_multiplicative_and_power`.
2. **`^` (exponentiation) is LEFT-associative in IDL**, not right-
   associative like Scilab's/MATLAB's/Python's own `^`/`**`. The official
   reference states the general rule "operators with equal precedence are
   evaluated from left to right," and — since `^` sits alone at its own tier
   with no competing operator — that rule governs a `^` chain against
   itself: `2^3^2` evaluates to `(2^3)^2 = 64` in real IDL, not
   `2^(3^2) = 512`. `power` is written with left-recursive `{ }` repetition,
   confirmed by `power_is_left_associative`.

`NOT` sits in `unary` (tier 5), not alongside `AND`/`OR`/`XOR` in `logical`
(tier 7) — despite `idl.tokens`' own lexer-layer comment grouping all four
under one "logical/bitwise" keyword-list heading, the official precedence
table places the *unary* `NOT` and the *binary* `AND`/`OR`/`XOR` at two
different tiers.

Assignment (`=`) is **not** part of the expression cascade at all — unlike
Scilab's/MATLAB's own chainable `assignment = logical_or [ EQ assignment ]`,
real IDL has no assignment-as-expression and no chained assignment; `=` is
purely a statement-level construct here.

## `REPEAT`'s `ENDREP`-before-`UNTIL` order — confirmed, not guessed

`REPEAT BEGIN ... ENDREP UNTIL expr` — the block form's `ENDREP` closes
`BEGIN`'s own block **before** the trailing `UNTIL expr` loop-condition
clause, confirmed against real IDL documentation rather than assumed from
the task brief's own paraphrase. Confirmed by
`repeat_until_block_form_endrep_precedes_until` and the regression
`repeat_until_reversed_order_is_a_syntax_error` (proving the reversed order
is rejected, not silently accepted either way).

## No `$` (continuation) handling in this grammar — a disclosed scope boundary

`idl-lexer` emits `CONTINUATION` (`$`) as an ordinary, un-suppressed token
and does not swallow the following `NEWLINE`. MA12 §5 assigns tracking `$`
(and paren/bracket balance) to `idl-repl`'s own "continuation scanner" — a
raw-text-level, pre-tokenization step that joins physical lines before
`tokenize_idl` ever runs. Consistently, `CONTINUATION` is not referenced
anywhere in `idl.grammar`; a bare `$` reaching this parser is a syntax error
by construction. This is MA-12d's job, not this crate's — `grammar-tools
cross_validate` reports the resulting "Token 'CONTINUATION' defined but
never used" warning, which is expected and documented, not a bug.

## Recursion-depth guard: six shapes, measured independently

`idl.grammar` has six structurally distinct self-referential recursion
shapes — parenthesised nesting, nested `IF`/`ENDIF` blocks (the other four
block constructs — `FOR`/`WHILE`/`REPEAT`/generic `BEGIN`/`END` — share the
identical `statement -> ... -> block_body -> statement_line -> statement`
reachability, confirmed by direct rule-graph inspection, so none of the
other four was separately measured), nested function-call arguments
(`f(f(f(...)))`), nested subscript indexing (`a[a[a[...]]]`, measured
independently despite an apparently-identical three-wrapper-frame shape to
call-argument nesting, to confirm the rule-graph symmetry actually holds at
the native-stack level too), a unary prefix chain (`- - - ... 5` /
`NOT NOT ... x`), and nested array literals (`[[[...]]]`) — each measured
independently (binary search, uncapped parser, default-stack worker thread,
debug build, one fresh subprocess per data point), per MA12 §6's own
directive and the "measure, don't assume one shape's floor bounds the
others" methodology `apl-parser`/`j-parser`/`scilab-parser`/`maple-parser`
each independently established.

The genuine surprise (mirroring `scilab-parser`'s own): the unary prefix
chain tolerates by far the *most* nesting levels (199) of any measured
shape, yet has the *lowest* rule-frame floor (212) — its persisting
per-level cost is exactly one rule-frame (`unary` itself, confirmed by the
near-1:1 nesting-to-frame ratio), cheap enough per level to reach 199
levels, yet its own call path costs more native-stack bytes per crossing
than the other shapes' higher per-level rule-frame counts would suggest.
`MAX_RULE_DEPTH = 148` sits about 30.2% below the binding 212 floor, safely
below all five other rule-frame floors (291, 249, 266, 273, 282) too. Full
measurement tables and reasoning in `MAX_RULE_DEPTH`'s own doc comment
(`src/lib.rs`).

## Usage

```rust
use coding_adventures_idl_parser::parse_idl;

let ast = parse_idl("PRO plot_it, x, y, COLOR=color\n  PRINT, x, /QUIET\nEND\n");
assert_eq!(ast.rule_name, "program");
```

`parse_idl` panics on a malformed source string; use `try_parse_idl` for the
`Result`-returning form, or `create_idl_parser` directly if you need the raw
`GrammarParser`.

## What this crate does NOT do

- No `idl-runtime`/`idl-repl` (MA-12d) evaluation — this crate only produces
  a `GrammarASTNode` CST.
- No `idl-to-semantic-ir` (MA-12e) lowering.
- No `$`-continuation joining (see above — that's `idl-repl`'s job).
- No changes to `idl-lexer` (MA-12b, already merged) — `idl.tokens` is
  untouched; this crate consumes its token stream as-is.
- No structures, pointers, objects, `LIST`/`HASH`, `COMMON` blocks,
  `CASE`/`SWITCH`/`FOREACH`, `_EXTRA`/`_REF_EXTRA` keyword inheritance, or
  the legacy pre-5.0 `( )` subscript form — all explicitly deferred by
  MA12 §4, unchanged here.

## Where this fits next

`idl-parser` is MA-12c of IDL's frontend crates, consuming the token stream
from `idl-lexer` (MA-12b) against `code/grammars/idl/idl.grammar` to build
the `GrammarASTNode` CST a future `idl-runtime` (MA-12d, not yet started)
will walk to evaluate over `array-runtime`'s `Array` value model plus a new
`IdlValue::{Num, Str}` enum and an `IdlCallable` procedure/function
representation with keyword-aware argument binding (MA12 §3), and a future
`idl-to-semantic-ir` (MA-12e) will lower into
[`SIR22`](../../../specs/SIR22-array-matrix-semantic-ir.md)'s array/matrix
domain.
