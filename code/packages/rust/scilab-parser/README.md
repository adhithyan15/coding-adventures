# coding-adventures-scilab-parser

Scilab parser backed by `code/grammars/scilab/scilab.grammar`, compiled to
Rust and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Where this fits

This is the second crate of Scilab's frontend — MA-10c from
[`MA10-scilab-language.md`](../../../specs/MA10-scilab-language.md), the spec
that first established *why* Scilab needs its own frontend at all rather than
a thin MATLAB-rewrite shim the way `octave-runtime` is (MA10 §1). The crate
layout mirrors MA10 §6's rollout: `scilab-lexer` (MA-10b) → `scilab-parser`
(this crate, MA-10c) → `scilab-runtime`/`scilab-repl` (MA-10d) →
`scilab-to-semantic-ir` (MA-10e).

`scilab.grammar` is **forked from** `code/grammars/matlab/matlab.grammar` at
the grammar-**source** level (copied, then diverged) — this crate does
**not** depend on `matlab-parser` at build time (MA10 §5). The grammar
*shape* (matrix literals, ranges, the operator-precedence cascade, indexing)
is a legitimate MATLAB-family inheritance; the *language* is not, which is
why the fork happens at the text level rather than the Rust-crate level.

## Scope

Covers the MA10 §4-scoped first cut: dense-matrix arithmetic, matrix literals
and ranges, comparisons (both not-equal spellings), logical operators,
`$`-based last-index indexing, assignment, `if/elseif/else/end` and
`select/case/else/end` (each with the optional `then` linker),
`while/end`/`for/end` (each with the optional `do` linker), `break`/
`continue`, `function ... endfunction`, and the eight `%`-prefixed special
constants and single-/double-quoted strings as ordinary expression atoms.

## Three things that are genuinely NOT MATLAB (MA10 §3)

### 1. `stmt_sep` — one new production, reused at six header sites

`if`/`elseif`/`select`/`case` take an optional `then` linker; `while`/`for`
take an optional `do` linker — every one of the six is, per MA10 §1 finding
4, individually replaceable by a bare comma or a newline. help.scilab.org's
own wording ("can be replaced by a carriage return or a comma") is read as
the linker keyword and the punctuation being ALTERNATIVES to each other, not
two pieces stacked — real Scilab accepts `if x then y = 1, end` with nothing
at all between `then` and `y`. So this collapses to one required (not
optional), four-spelling production, reused at all six sites:

```
stmt_sep = "then" | "do" | COMMA | NEWLINE ;
```

mirroring the "one new rule, reused at multiple sites" shape J's own
`verb_train` production takes (MA06 §3) or APL's reduce/scan operator
productions. A deliberate, disclosed simplification: `stmt_sep` accepts
*both* `then` and `do` uniformly at all six sites, rather than restricting
which linker is idiomatic at which site — narrowing that would need six
distinct nonterminals in place of one shared rule, and MA10 §3 states which
spelling is idiomatic, not that the other is a syntax error there.

### 2. `endfunction` is its own closing production, not unified with `end`

Real Scilab's historical/still-preferred function terminator is
`endfunction` (MA10 §1 finding 7) — `func_def` is the ONLY production in
this grammar that ever matches the `endfunction` token, and no production
anywhere lets a bare generic `"end"` close a function. `function ... end`
(using the wrong closer) is therefore a **syntax error by construction**, not
a discouraged style — confirmed by this crate's own
`endfunction_is_required_not_generic_end` test.

### 3. `$` replaces MATLAB's context-sensitive `end`-as-last-index trick

MATLAB's `end`-as-last-index is context-**sensitive** — `matlab-parser`'s own
pre-parse hook must retag every bracket-interior `end` to a `NAME` before its
grammar ever runs (see `matlab.grammar`'s own header comment). Scilab's `$`
needs **none** of that: `scilab-lexer` already lexes `$` as its own
always-unambiguous `DOLLAR` token in every position, so this grammar simply
adds `DOLLAR` as an ordinary `primary` alternative — the same tier as
`NUMBER`/`NAME` — which is exactly what makes `A($-1)` parse at all (`$` must
be usable as an ordinary operand to `additive`'s `MINUS`, the same way a
`NUMBER` can). `PERCENT_CONST` (the eight `%`-prefixed special constants) is
added at the identical tier for the identical reason.

## No `switch`/`otherwise`/`try`/`catch`/`return`/`global`/`persistent`/`lambda`

None of these tokens exist in `scilab.tokens` at all (MA10 §4 scopes them
out) — Scilab's own multi-way conditional is `select`/`case`/`else` (MA10 §1
finding 4), not `switch`/`otherwise`, so this grammar references neither
spelling anywhere.

## A known, expected `grammar-tools validate` false positive

`primary`'s `STRING` reference is flagged by the cross-validator ("Grammar
references token 'STRING' which is not defined in the tokens file") because
`scilab.tokens` *deliberately* leaves `DQ_STRING` un-aliased (so its own
`collapse_dq_string_escapes` post-hook can tell "still needs `""` collapsed"
apart from "already decoded") — unlike `matlab.tokens`, which declares
`DQ_STRING = ... -> STRING` and so registers "STRING" as a statically known
name. Both `STRING_PLACEHOLDER`- and `DQ_STRING`-derived tokens are
relabelled to `type_name = "STRING"` by `scilab-lexer`'s own post-tokenize
hooks *before* any token reaches this grammar, and
`parser::grammar_parser::GrammarParser`'s own `match_token_reference`
matches purely on a token's live `type_name` — so `STRING` here is
functionally correct at parse time regardless of what the static validator
can prove. Fixing the false positive at the source would require modifying
`scilab.tokens`, which this crate does not touch (MA-10b is already merged).
See `scilab.grammar`'s own header comment for the full explanation.

## Recursion-depth guard: seven shapes, measured independently

`scilab.grammar` has seven structurally distinct self-referential recursion
shapes — parenthesised nesting, a flat right-recursive power (`^`) chain, a
unary prefix chain (`- - - … x` / `~ ~ ~ … x`), chained assignment
(`x=x=x=…=5`), deeply nested `if`/`end` (`select`/`end` shares the identical
`statement -> if_stmt` reachability, so it was not separately measured — a
provable identity, not an assumed shape resemblance), function-call/cell-index
argument nesting (`f(f(f(…)))`; cell-index nesting shares the identical
`arg_list`-mediated reachability, so it was not separately measured either),
and matrix-literal nesting (structurally distinct from parenthesised nesting:
`matrix_literal` reaches `expr` through two extra rule-frames, `matrix_rows`
then `matrix_row`, that `group` does not pay; cell-literal nesting shares this
identical shape) — each measured independently (binary search, uncapped
parser, default-stack worker thread, debug build, one fresh subprocess per
data point), per [MA10](../../../specs/MA10-scilab-language.md) §6's own
directive and the "measure, don't assume one shape's floor bounds the others"
methodology `apl-parser`/`j-parser`/`maple-parser` each independently
established — the initial four-shape survey missed chained assignment and
argument nesting entirely and only reasoned about (rather than measured) the
unary prefix chain, a gap a security-review pass caught and closed.

The genuine surprise (mirroring `maple-parser`'s own): converting each
shape's *nesting-count* crash floor into *rule-frame* terms (the units
`MAX_RULE_DEPTH` actually bounds) shows chained assignment — which tolerates
far more nesting levels (162) than parenthesised nesting (18), nested `if`
(62), or matrix literals (15) — has the *lowest* rule-frame floor (179),
lower even than the power chain's 220, despite the power chain crashing at
far fewer levels (101). Neither "the shape that tolerates the fewest levels
must bind" (matrix literals, wrong here) nor "parenthesised nesting binds,
since it does for nearly every sibling `*-parser` crate in this repo" (also
wrong — parens has a higher rule-frame floor, 295, than chained assignment
here) holds in general. `MAX_RULE_DEPTH = 125` sits about 30.2% below the
binding 179 floor, safely below all six other floors (295, 220, 268, 289,
277, 219) too. Full measurement tables and reasoning in `MAX_RULE_DEPTH`'s
own doc comment (`src/lib.rs`).

**Known, disclosed limitation**: `while`/`for`/nested-`function` bodies form
the same `statement`-cycle shape as nested `if` (measured floor 268) but were
not independently measured — structurally closer to that shape than to the
ones that turned out to diverge (chained assignment, unary prefix), and 125
sits 143+ units below 268, so risk is assessed as low. This is a completeness
gap for a future audit to close, not a claim of exhaustive coverage.

## Usage

```rust
use coding_adventures_scilab_parser::parse_scilab;

let ast = parse_scilab("x = %pi * 2\nif x > 0 then y = 1, end\n");
assert_eq!(ast.rule_name, "program");
```

`parse_scilab` panics on a malformed source string; use `try_parse_scilab`
for the `Result`-returning form, or `create_scilab_parser` directly if you
need the raw `GrammarParser`.

## Where this fits next

`scilab-parser` is MA-10c of Scilab's frontend crates, consuming the token
stream from `scilab-lexer` (MA-10b) against
`code/grammars/scilab/scilab.grammar` to build the `GrammarASTNode` CST a
future `scilab-runtime` (MA-10d, not yet started) will walk to evaluate over
`array-runtime`'s `Array` value model plus a new `ScilabValue::{Num, Str}`
enum (MA10 §2), and a future `scilab-to-semantic-ir` (MA-10e) will lower into
[`SIR22`](../../../specs/SIR22-array-matrix-semantic-ir.md)'s array/matrix
domain.
