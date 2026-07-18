# coding-adventures-reduce-repl

An interactive Read-Eval-Print loop and the `reduce` binary for Reduce (a
subset). Wraps a persistent
[`coding_adventures_reduce_runtime::ReduceSession`](../reduce-runtime) and
adds the console behaviours an interactive Reduce user expects. See
[`code/specs/MA08-reduce-language.md`](../../../specs/MA08-reduce-language.md).

## Usage

```sh
cargo run -p coding-adventures-reduce-repl --bin reduce
```

```text
Reduce (on the shared symbolic stack) — one statement per line, type QUIT to exit.
> x := 5;
5
> x + 1;
6
> h(l, m) := l + m;
h
> h(2, 3);
5
> QUIT
```

## Behaviour

- **A plain, non-numbered prompt (`> `)** — MA08 §2/§5 are explicit that
  Reduce's own session transcript has no numbered-input convention the way
  Derive's `#n:` or Wolfram's `In[n]:=` do (even though *real* REDUCE's own
  interactive prompt actually is numbered, `1: `, `2: `, …) — so, unlike
  `derive-repl`, results here are never prefixed at all.
- **Line continuation** — a statement is complete once `(`/`)`, `{`/`}`
  (Reduce's list braces, MA08 §3 — NOT Derive's `[`/`]`), and `<<`/`>>`
  (group-statement delimiters, matched as a genuine two-character pair, so
  a bare comparison `<`/`>` never falsely triggers it) are all balanced;
  like `derive-repl` there is no string/comment state to track (this
  subset has neither — MA08 §4).
- **`QUIT`/`EXIT`** (case-insensitive) or Ctrl-D ends the session.
- **Non-fatal errors** — a surface error (parse failure, a malformed
  `Assign` LHS, …) prints and the session keeps working.

## Security: bounded line reads

`run` reads each physical line through a byte-oriented, cap-bounded
`read_bounded_line` (64 KiB) rather than `BufRead::read_line` directly —
carrying forward the exact fix `derive-repl`/`j-repl`/`apl-repl` needed
after their own `/security-review`: `read_line` has no length bound of its
own, so a single, arbitrarily long line (no embedded newline) would be
fully buffered in memory before `ReduceRepl::feed`'s own size check ever
ran. See the module doc comment for the full rationale (including the
multibyte-character/cap-boundary edge case this fix also closes).

## Tests

```sh
cargo test -p coding-adventures-reduce-repl
```

Prompt/continuation behaviour (including the `<<`/`>>` vs bare `<`/`>`
disambiguation), quit words, error recovery, persistent bindings, an
end-to-end Reduce program, and the full `read_bounded_line` regression
suite (oversized single line, exact-cap boundary, multi-chunk overflow
drain, multibyte-character straddle).
