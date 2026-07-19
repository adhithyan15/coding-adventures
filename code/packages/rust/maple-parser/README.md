# coding-adventures-maple-parser

Maple parser backed by `code/grammars/maple/maple.grammar`, compiled to Rust
and statically linked into the crate.

The runtime path does not read grammar files from disk, which keeps it
suitable for a future WASM facade.

## Scope

Covers the MP-1-scoped subset fixed by
[MA09 §3](../../../specs/MA09-maple-language.md). Maple looks, on the
surface, like a close cousin of REDUCE (the same `:=` assignment spelling,
the same `and`/`or`/`not` keywords) but genuinely is not "REDUCE again":
Maple has three aggregate literal types where REDUCE/Derive each have one
(this subset covers two, list `[...]` and set `{...}`), and its `f(x) := e`
spelling means something narrower (a remember-table patch) than REDUCE's own
general-definition idiom of the identical shape — this parser's grammar is
the enforcement point for that distinction (see below).

## Statements vs. expressions — the one structural divergence from `reduce-parser`

`reduce-parser`'s own `expr = if_expr | group_expr | assignment` sits `if`/
`<<...>>` *above* assignment specifically because MA08 §3 makes an explicit
claim: REDUCE's `if` "returns whichever branch ran" and its group statement
"evaluates to its last statement's value" — both are genuinely
value-producing REDUCE expressions.

MA09 makes no equivalent claim for Maple's `if`, and real Maple's own idiom
for a conditional *value* is the `piecewise(...)` library function — an
ordinary call this grammar's `postfix` production already parses for free —
not embedding an `if` where a value is expected. So this grammar splits
cleanly into two nonterminals that never call back into each other:

```
statement = if_expr | assignment          (Chapter 5 — "Maple Statements")
expr      = logical_or -> ... -> atom      (Chapter 3 — "Maple Expressions")
```

Consequences: `x := if a then 1 else 2 end if;` and chained assignment
`a := b := c;` are both **syntax errors** in this subset — unlike
`reduce-parser`'s identical-looking `assignment`, which supports both. See
`maple.grammar`'s own "Design decision: statements vs. expressions" header
comment for the full reasoning and citations.

## `f(x) := e` (the remember-table spelling) does not parse at all

`reduce-parser`'s `assignment` left-hand side is a full `logical_or` (which
bottoms out at a call shape `h(l, m)`) *because* REDUCE's `h(l, m) := e` is
its general procedure-definition idiom. Maple's identical-looking
`f(x) := e` means something narrower and different — a remember-table
specific-value patch onto an *already-existing* procedure (MA09 §1) —
deliberately **excluded** from this subset (MA09 §4), not merely deferred.

So this grammar's `assignment` left-hand side is a bare `NAME` token, full
stop:

```
assignment = NAME ASSIGN ( arrow_def | expr ) | expr ;
```

`f(x) := 1;` fails to parse entirely: after `NAME "f"`, the next token is
`LPAREN`, not `ASSIGN`, so `assignment`'s first alternative fails outright;
`f(x)` alone then parses as an ordinary call-shaped `expr` statement via the
second alternative, but the leftover `:= 1` has nowhere to attach, so the
whole statement fails at the terminator check. This is a deliberate,
grammar-level enforcement of MA09 §4's exclusion — not a runtime concern
pushed onto a future `maple-runtime`.

## The arrow (`->`) operator: grammar-enforced placement

MA09 §3: `->` is "used only as a `Define` right-hand side in this subset."
`ARROW` appears in exactly one place in `maple.grammar` — `arrow_def`'s own
production, tried as `assignment`'s first alternative right after
`NAME ASSIGN`:

```
assignment   = NAME ASSIGN ( arrow_def | expr ) | expr ;
arrow_def    = arrow_params ARROW expr ;
arrow_params = NAME | LPAREN [ NAME { COMMA NAME } ] RPAREN ;
```

No other rule references `ARROW`, so `->` cannot appear as a bare statement,
nested inside arithmetic, or as a function argument — the grammar itself is
the enforcement point, matching this repo's stated preference for catching
invalid shapes at parse time. Backtracking (not a lookahead predicate) makes
this deterministic: `f := x;` tries `arrow_def` first, fails to find `ARROW`
after the bare name `x`, and cleanly backtracks to the plain `expr`
alternative — ordinary memoized PEG ordered-choice, no ambiguity.

## `if`/`elif`/`else` closed two ways, with no dangling-else ambiguity

Real Maple closes an `if` with either the two-keyword sequence `end if` or
the standalone `fi` ("if" reversed). `maple-lexer` emits `end` and `if` as
two independent `KEYWORD` tokens, so this grammar is where the composition
actually happens:

```
if_expr = "if" expr "then" statement
          { "elif" expr "then" statement }
          [ "else" statement ]
          ( "end" "if" | "fi" ) ;
```

Because every `if_expr` requires an explicit close, there is **no
dangling-else ambiguity** here at all — unlike `reduce-parser`'s own `if`,
which has to rely on ordinary recursive-descent order-of-attempt to resolve
`if a then if b then c else d`. A nested `if` inside a branch must run all
the way to its own `end if`/`fi` before an outer `if`'s own `elif`/`else`/
close is ever reached — there is structurally only one place an outer
`else` can attach.

Branches are `statement` (not the narrower `expr`), so an `if` can branch to
an assignment or a nested conditional (`if a then x := 1 else x := 2 end
if`) — matching MA09 §3's own generic `s1`/`s2`/`s3` labelling — while an
`if_expr` itself is still never reachable *from* `expr`.

## List vs. set literals — two productions, one shared `arglist`

`[a, b, c]` (list, ordered, duplicates kept) and `{a, b, c}` (set,
unordered, duplicates removed at a future `maple-runtime`) are syntactically
distinct productions that both reuse the same `arglist = expr { COMMA expr
}` production a function call's own argument list uses:

```
list_literal = LBRACKET [ arglist ] RBRACKET ;
set_literal  = LBRACE [ arglist ] RBRACE ;
```

Empty `[]`/`{}` fall out of the optional inner `arglist` for free. `{a, b,
c}` is the *same* bracket REDUCE uses for its own list literal — same
spelling, different meaning both times, the exact family-resemblance trap
MA09 §1 names by title; this grammar only assigns the bracket a *structure*
(which rule matched), the Set-vs-List semantic tag is a future
`maple-runtime`'s concern when it lowers to `Set[...]` vs `List[...]`.

## Precedence, loosest to tightest

```
statement = if_expr | assignment            (never reachable from `expr`)
  assignment (NAME :=, LHS is a bare NAME -- see above)
    RHS: arrow_def (params -> expr) | expr
  expr = logical_or
    → OR
      → AND
        → NOT (prefix)
          → comparison (= <> < > <= >=, flat non-chaining)
            → additive (+ -)
              → multiplicative (* /, explicit `*` required)
                → unary minus (prefix)
                  → power (^, right-assoc, no `**` synonym)
                    → postfix application  f(...)  (single call suffix)
                      → atom  (NUMBER, NAME, true, false,
                               [list literal], {set literal}, ( ... ))
```

`=` is Maple's *equation* operator — never assignment; `:=` alone owns that
role. `<>` is Maple's own not-equal spelling (not REDUCE's word-keyword
`neq`, not Wolfram's `!=`). `and`/`or`/`not`/`if`/`then`/`elif`/`else`/`end`/
`fi`/`true`/`false` are lowercase `KEYWORD` tokens, matched by literal
spelling (the mirror image of `derive-lexer`'s uppercase `AND`/`OR`/`NOT`).

## Usage

```rust
use coding_adventures_maple_parser::parse_maple;

let ast = parse_maple("f := (x, y) -> x + y;\nf(1, 2);\n");
assert_eq!(ast.rule_name, "program");
```

`parse_maple` panics on a malformed source string; use `try_parse_maple` for
the `Result`-returning form, or `create_maple_parser` directly if you need
the raw `GrammarParser`.

## Recursion-depth guard: six shapes, and a genuine surprise

`maple.grammar` has six distinct self-referential recursion shapes —
parenthesised grouping, list-literal nesting (proven structurally identical
to set-literal nesting by direct inspection of the shared `arglist`
production, so not separately measured), a `not` prefix chain, a
unary-minus prefix chain, a power (`^`) chain, and nested `if`/`end if`
(or `fi`) — each measured independently (binary search, uncapped parser,
default-stack worker thread, debug build, one fresh subprocess per data
point), per [MA06](../../../specs/MA06-j-language.md) §6's established
methodology. Every "flat chain of one operator" production written with
EBNF `{ x }` repetition instead (`logical_or`, `logical_and`, `additive`,
`multiplicative`, the `elif` chain, `arglist`) costs *zero* native stack
regardless of width — confirmed by reading `parser::grammar_parser`'s own
`Repetition` implementation directly (a plain iterative loop), the same
engine-level fact `reduce-parser` already established.

The genuine surprise: converting each shape's nesting-count crash floor into
rule-frame terms (the units `MAX_RULE_DEPTH` actually bounds) shows the
`not` prefix chain — which tolerates by far the *most* nesting levels of
the six (205, alongside its near-twin the unary-minus chain) — has the
*lowest* rule-frame floor (218), lower even than nested-`if` (289), which
tolerates far fewer levels (137). Assuming either "the shape with the fewest
tolerated levels must bind" (nested-`if`, wrong here) or "parenthesised
nesting binds, since it does for nearly every sibling `*-parser` crate in
this repo" (also wrong — parens has the *highest* rule-frame floor of the
six) would each have shipped a cap unsafe specifically for prefix chains.
`MAX_RULE_DEPTH = 150` sits about 31.2% below the binding 218 floor. Full
measurement table and reasoning in `MAX_RULE_DEPTH`'s own doc comment
(`src/lib.rs`).

## Where this fits

`maple-parser` is MP-3 of Maple's frontend crates, consuming the token
stream from `maple-lexer` (MP-2) against
`code/grammars/maple/maple.grammar` to build the `GrammarASTNode` CST a
future `maple-runtime` (MP-4, not yet started) will lower into
`symbolic_ir::IRNode` and evaluate with `symbolic_vm::VM` over the stock
`SymbolicBackend`. MP-4 will need to know this crate's two disclosed
divergences from `reduce-parser`'s shape when it lowers: `if`/`:=` are
statement-only (never nested inside an `expr`), and `f(x) := e` never
reaches the lowering stage at all, since it never parses.
