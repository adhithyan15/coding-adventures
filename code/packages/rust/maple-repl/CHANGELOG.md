# Changelog

## [0.1.0] - 2026-07-19

### Added

- Initial `maple-repl` crate (MA09 MP-4, front2 Wave 5): an interactive
  Read-Eval-Print loop and the `maple` binary, wrapping a persistent
  `MapleSession` (`maple-runtime`) — mirroring `reduce-repl`'s driver, per
  MA09 §2/§5's explicit template pointer ("matching Reduce's own
  unnumbered `reduce-repl`").
- A **plain, non-numbered** prompt (`"> "`, continuation `"... "`) — MA09
  §2/§5 are explicit that Maple's own session transcript has no
  numbered-input convention the way Derive's `#n:` or Wolfram's `In[n]:=`
  do, so no result is ever prefixed with an index.
- `(`/`)`-, `[`/`]`- (Maple's *list* literal, MA09 §3 — square brackets,
  not Reduce's curly braces), and `{`/`}`-aware (Maple's *set* literal, new
  to this language) bracket-depth line continuation, carried forward from
  `reduce-repl`'s identical shape.
- **New continuation tracking beyond any sibling CAS-family REPL**: `if` /
  `end if`|`fi` block-keyword balance. Real Maple's `if_expr` requires an
  explicit closer (unlike REDUCE's `if`/`then`/`else`, which needs none),
  so an entirely ordinary multi-line
  ```text
  if a > 0 then
    1
  else
    -1
  end if;
  ```
  would otherwise submit prematurely after the first line (no open
  bracket) and fail to parse. The word-scanner tracks `"if"` (opens),
  `"fi"` (closes), and the two-keyword `"end" "if"` sequence (closes as a
  single unit, not double-counted) by exact lowercase spelling, mirroring
  `maple.tokens`' own case-sensitive keyword rule; since this subset has no
  comments or string literals (MA09 §4), the scanner needs no comment/
  string-skipping state the way `matlab-repl`'s/`octave-repl`'s own
  keyword-block trackers do.
- Case-insensitive `QUIT`/`EXIT` to end the session, carried forward as a
  REPL-level convenience (not a language feature — real Maple's own exit
  is a library call, not a REPL keyword).
- Bounded single-physical-line reads (`read_bounded_line`, 64 KiB cap),
  carrying forward the `reduce-repl`/`derive-repl`/`j-repl`/`apl-repl`
  `/security-review` fix ("cap unbounded single-line read before
  continuation-buffer check") rather than reintroducing the same gap in a
  new REPL.
- The `maple` binary is declared directly in this crate's own `Cargo.toml`
  (`[[bin]] name = "maple"`), **not** a separate `code/programs/rust/maple`
  crate — verified against `reduce`/`derive`/`wolfram`'s own binaries,
  none of which has a `code/programs/rust/` entry either; each `-repl`
  crate is its own binary crate. See MA09 §5's own note on this.
- 25 tests: prompt/continuation behaviour (parens, list brackets, set
  braces, the new `if`/`end if`/`fi` block tracking including nested
  `if`s), quit words, error recovery, persistent bindings, an end-to-end
  Maple program, and the full `read_bounded_line` regression suite
  (oversized line, exact-cap boundary, multi-chunk overflow drain,
  multibyte-character straddle).
