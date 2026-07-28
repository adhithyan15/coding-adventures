# coding-adventures-axiom-repl

An interactive Read-Eval-Print loop for Axiom (the MA-13-scoped
consumer-view subset), plus the `axiom` binary. See
[`code/specs/MA13-axiom-language.md`](../../../specs/MA13-axiom-language.md)
and [`axiom-runtime`](../axiom-runtime).

## Where this fits

```
axiom.tokens + axiom-lexer     (MA-13b)
axiom.grammar + axiom-parser   (MA-13c)
axiom-runtime                  (MA-13d)
axiom-repl                     (MA-13d, this crate) -- the `axiom` binary
axiom-to-semantic-ir           (MA-13e, next)
```

## Console conventions

- **`(n) -> ` prompts** — mirroring real Axiom's own numbered interactive
  prompt (MA13 §5, confirmed directly against the book: `(1) ->`,
  incrementing per computation step). Continuation while a statement spans
  multiple physical lines is shown as `   -> `.
- **Line continuation** — a statement is complete once `(`/`[` bracket
  depth returns to zero *and* no string literal is left open. A small state
  machine skips `--` line comments and `"..."` string contents before ever
  looking at a character for bracket-depth purposes, so a stray bracket
  inside either can never falsely extend (or end) continuation — Axiom,
  unlike Derive, has both comments and strings in its lexical surface this
  cut.
- **`)quit`** (also `quit`/`QUIT`, or Ctrl-D) ends the session. Real Axiom's
  own session commands are `)`-prefixed system commands (`)what`,
  `)clear all`, ...) — MA13 §4 defers that whole surface as session/tooling,
  not language surface; this REPL recognises only `)quit` as a
  console-layer convenience, adding none of the rest of that surface.
- A surface error prints (`Error: ...`) and the session continues.

## Two known REPL bug classes, checked and fixed from day one

Two bug classes have already been found and fixed in sibling REPLs in this
repo's history; both are avoided here from the start rather than risked and
re-discovered:

1. **Push-before-size-check ordering** (fixed in `reduce-repl`/
   `derive-repl`/`apl-repl`/`j-repl`) — `AxiomRepl::feed` checks the
   accumulation buffer's prospective size *before* ever calling
   `push_str`, so an arbitrarily large single physical line can never be
   copied in before the size check meant to bound it runs.
2. **Unbounded single-physical-line read before the continuation-buffer
   check** (fixed in `j-repl`/`apl-repl`) — `run` reads each physical line
   through `read_bounded_line` (capped at 64 KiB), not `BufRead::read_line`
   directly, which has no length bound of its own.

Both are covered by regression tests in `src/lib.rs` (see
`an_unterminated_buffer_is_submitted_once_over_the_size_cap` and the
`read_bounded_line_*`/`run_reports_an_oversized_line_cleanly_*` tests).

**A third, `axiom-repl`-specific issue was found and fixed in this crate's
own security review, before merge:** the continuation heuristic originally
rescanned the *entire accumulated buffer* from scratch on every physical
line fed to it, an O(n²) worst case across the lines of one long statement
(bounded by `MAX_INPUT_LEN`, so not severe, but real wasted,
attacker-influenceable CPU work). `scan_line` now updates the running
bracket-depth/open-string state *incrementally*, scanning only the
newly-fed line each time — O(n) total. See `scan_line`'s own doc comment,
and the cross-line-state regression tests (`a_string_open_across_several_
physical_lines_carries_state_correctly`,
`bracket_depth_carries_correctly_across_a_comment_on_an_intermediate_line`).

## Usage

```sh
cargo run -p coding-adventures-axiom-repl --bin axiom
```

```text
Axiom (on the shared symbolic stack) -- one statement per line, type )quit to exit.
(1) -> a : PositiveInteger
(1) true : Boolean
(2) -> a := 5
(2) 5 : PositiveInteger
(3) -> a := -1
Error: Cannot convert right-hand side of assignment -1 to an object of the type PositiveInteger of the left-hand side.
(4) -> Polynomial(Integer) has Ring
(3) true : Boolean
(5) -> f(x: Integer): Integer == x * x
(4) f
(6) -> f(6)
(5) 36 : PositiveInteger
(7) -> )quit
```

Note the `(n) ->` prompt counter (how many inputs have been *read*) and the
`(n)` result counter (how many statements have *succeeded*) intentionally
diverge after an error — the failed `a := -1` above advances the prompt from
`(3)` to `(4)` but is never itself assigned a result number, so the next
successful statement is `(3)`, not `(4)`. This mirrors real Axiom's own
step-history semantics (`%`/`%%(n)` refer back to *computed* results, MA13
§5), where a rejected input is not itself a history step.

## Tests

```sh
cargo test -p coding-adventures-axiom-repl
```

Covers: single- and multi-line statement continuation across parens,
brackets, and a `;`-block; string/comment-aware continuation (a bracket
inside either does not trigger continuation); prompt/continuation-prompt
switching; quit words; error recovery; persistent bindings and declared
domains across lines; both REPL bug-class regressions; and an end-to-end
`run` driver test.
