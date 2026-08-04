# coding-adventures-derive-repl

An interactive Read-Eval-Print loop and the `derive` binary for Derive (a
subset). Wraps a persistent
[`coding_adventures_derive_runtime::DeriveSession`](../derive-runtime) and
adds the console behaviours a Derive worksheet user expects. See
[`code/specs/MA07-derive-language.md`](../../../specs/MA07-derive-language.md).

## Usage

```sh
cargo run -p coding-adventures-derive-repl --bin derive
```

```text
Derive (on the shared symbolic stack) — one statement per line, type QUIT to exit.
#1: x := 5
#1: 5
#2: x + 1
#2: 6
#3: DIF(x^2, x)
#3: 2*x
#4: QUIT
```

## Behaviour

- **`#n: ` prompts** — Derive's own numbered expression-history convention
  (MA07 §5), not Wolfram's `In[n]:=`/`Out[n]=` split.
- **Line continuation** — a statement is complete once `(`/`[` bracket depth
  returns to zero; unlike `wolfram-repl` there is no string/comment state to
  track (this subset has neither — MA07 §4).
- **`QUIT`/`EXIT`** (case-insensitive) or Ctrl-D ends the session.
- **Non-fatal errors** — a surface error (parse failure, a malformed
  `Assign` LHS, …) prints and the session keeps working.

## Security: bounded line reads

`run` reads each physical line through a byte-oriented, cap-bounded
`read_bounded_line` (64 KiB) rather than `BufRead::read_line` directly —
carrying forward the exact fix `j-repl`/`apl-repl` needed after their own
`/security-review`: `read_line` has no length bound of its own, so a single,
arbitrarily long line (no embedded newline) would be fully buffered in
memory before `DeriveRepl::feed`'s own size check ever ran. See the module
doc comment for the full rationale (including the multibyte-character/
cap-boundary edge case this fix also closes).

## Tests

```sh
cargo test -p coding-adventures-derive-repl
```

Prompt/continuation behaviour, quit words, error recovery, persistent
bindings, an end-to-end worksheet program, and the full `read_bounded_line`
regression suite (oversized single line, exact-cap boundary, multi-chunk
overflow drain, multibyte-character straddle).
