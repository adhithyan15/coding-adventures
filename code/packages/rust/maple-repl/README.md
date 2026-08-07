# coding-adventures-maple-repl

An interactive Read-Eval-Print loop and the `maple` binary for Maple (a
subset). Wraps a persistent
[`coding_adventures_maple_runtime::MapleSession`](../maple-runtime) and adds
the console behaviours an interactive Maple user expects. See
[`code/specs/MA09-maple-language.md`](../../../specs/MA09-maple-language.md).

## Usage

```sh
cargo run -p coding-adventures-maple-repl --bin maple
```

```text
Maple (on the shared symbolic stack) — one statement per line, type QUIT to exit.
> x := 5;
5
> x + 1;
6
> f := (x, y) -> x + y;
f
> f(2, 3);
5
> if x > 0 then
...   1
... else
...   -1
... end if;
1
> QUIT
```

## Behaviour

- **A plain, non-numbered prompt (`> `)** — MA09 §2/§5 are explicit that
  Maple's own session transcript has no numbered-input convention the way
  Derive's `#n:` or Wolfram's `In[n]:=` do, matching
  [Reduce](../reduce-repl)'s own unnumbered `reduce-repl` (the template
  this crate follows most closely).
- **Line continuation** — a statement is complete once `(`/`)`, `[`/`]`
  (Maple's *list* literal, MA09 §3 — square brackets, not Reduce's curly
  braces), and `{`/`}` (Maple's *set* literal, new to this language) are
  all balanced, **and** any open `if` has reached its own `end if`/`fi`
  closer. That last part is genuinely new relative to `reduce-repl`: real
  Maple's `if_expr` requires an explicit close (REDUCE's own `if`/`then`/
  `else` doesn't), so a plain multi-line
  ```text
  if a > 0 then
    1
  else
    -1
  end if;
  ```
  needs its own tracking, closer in spirit to `matlab-repl`'s/
  `octave-repl`'s keyword-block continuation than to any other CAS-family
  REPL here. Since this subset has no comments or string literals (MA09
  §4), the word-scanner needs no comment/string-skipping state, unlike
  those two.
- **`QUIT`/`EXIT`** (case-insensitive) or Ctrl-D ends the session.
- **Non-fatal errors** — a surface error (parse failure, a wrong-arity
  `diff`/`int` call, …) prints and the session keeps working.

## Security: bounded line reads

`run` reads each physical line through a byte-oriented, cap-bounded
`read_bounded_line` (64 KiB) rather than `BufRead::read_line` directly —
carrying forward the exact fix `reduce-repl`/`derive-repl`/`j-repl`/
`apl-repl` needed after their own `/security-review`: `read_line` has no
length bound of its own, so a single, arbitrarily long line (no embedded
newline) would be fully buffered in memory before `MapleRepl::feed`'s own
size check ever ran. See the module doc comment for the full rationale
(including the multibyte-character/cap-boundary edge case this fix also
closes).

## Tests

```sh
cargo test -p coding-adventures-maple-repl
```

Prompt/continuation behaviour (parens, list brackets, set braces, the new
`if`/`end if`/`fi` block-keyword tracking including nested `if`s), quit
words, error recovery, persistent bindings, an end-to-end Maple program,
and the full `read_bounded_line` regression suite (oversized single line,
exact-cap boundary, multi-chunk overflow drain, multibyte-character
straddle).
