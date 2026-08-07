# coding-adventures-idl-repl

An interactive Read-Eval-Print loop, and the `idl` binary, for
[IDL](https://en.wikipedia.org/wiki/IDL_(programming_language)) (Interactive
Data Language). Item **MA-12d** of the IDL frontend (spec
[`MA12`](../../../specs/MA12-idl-language.md)). Structured like the sibling
REPLs (`q-repl`, `scilab-repl`) — a persistent `Interpreter` plus the
interactive behaviours a console needs — with one genuinely new piece: a
**continuation scanner**.

## The continuation scanner

MA12 §5 assigns this crate a responsibility both `idl-lexer` (MA-12b) and
`idl-parser` (MA-12c) explicitly disclosed as deferred to it: `$` (line
continuation) has no meaning anywhere in `idl.grammar` — a bare `$` reaching
`idl-parser` is a syntax error by construction, per that crate's own README.
This crate's scanner tracks, at the raw text level, before a chunk of input
is ever handed to `tokenize_idl`/`parse_idl`:

1. **Paren/bracket balance** (`(`/`)`, `[`/`]`).
2. **The `$` line-continuation character** — a trailing `$` means "this
   logical line is not finished; join the next physical line onto it,"
   distinct from bracket balance (a complete, balanced expression can still
   end in `$`).
3. **`BEGIN...ENDxxx`/`PRO`/`FUNCTION` block depth** — `IF...THEN BEGIN`,
   `FOR...DO BEGIN`, `WHILE...DO BEGIN`, `REPEAT BEGIN`, a generic
   `BEGIN...END`, and `PRO`/`FUNCTION` definitions (all closed by a bare
   `END` or their own matched terminator) can each span many physical lines
   when typed interactively.

Like `q-repl`, this scanner delegates to the *real* IDL lexer
(`coding_adventures_idl_lexer::try_tokenize_idl`) rather than a hand-rolled
character scan, tokenizing only the newly fed physical line (not the whole
accumulated buffer, avoiding the O(n²) whole-buffer-re-tokenization bug
class `q-repl`'s own `CHANGELOG.md` documents paying down once) and folding
its bracket/block-keyword/`$` tokens into persisted running counts.

### Two lessons this crate applies from the start, not discovers as a regression

- **The push-before-size-check ordering.** This repo already fixed a "the
  continuation buffer's cap is checked *after* growing the buffer" bug
  class once, across `reduce-repl`/`derive-repl`/`apl-repl`/`j-repl` (cited
  directly in `q-repl`'s own `MAX_CONTINUATION_BUFFER` doc comment, "task
  #80"). `IdlRepl::feed` computes the *prospective* buffer size and checks
  it against the cap **before** ever calling `push_str`.
- **Comments must still be stripped per physical line, before the join.**
  It is tempting to assume IDL's `;` comment — unconditional, unlike Q's
  whitespace-adjacency-dependent `/` — needs no special REPL-level
  handling at all. That assumption is wrong, and this crate's own test
  suite caught it before shipping: this scanner joins continuation lines
  with a single **space**, never a real `'\n'` (see below), so a
  comment's own `/;[^\n]*/` regex finds no real newline to stop at once
  two lines are joined — a `;` on an *earlier* line would otherwise
  swallow a *later* line's own closing bracket. `strip_trailing_comment`
  fixes this — simpler than `q-repl`'s own `blank_line_comment` (only a
  two-state single-/double-quote tracker is needed, not whitespace
  adjacency, since IDL's `;` is unconditional and its strings have no
  escape mechanism) but the same *kind* of fix, applied per physical line
  before anything else touches it.

### Why every continuation join uses a space, never a real newline

An earlier version of this scanner joined `BEGIN...ENDxxx`/paren/bracket
continuations with a real `'\n'` (reasoning that `block_body`'s own
`statement_line` production has an optional trailing `NEWLINE`, so
injecting one seemed harmless) and reserved the space-join for `$` alone.
That is also wrong, caught the same way: `idl.grammar`'s expression cascade
has no tolerance for a stray `NEWLINE` token appearing where an operator
expects its next operand, so splitting `PRINT, (1 + 2` / `+ 3)` across two
lines and joining with `'\n'` is a genuine parse error. A single space is
safe for every continuation reason at once. See `lib.rs`'s own module doc
comment for the full writeup (including the exact regression tests that
caught both of these before they ever shipped).

## Usage

```sh
cargo run --bin idl
```

```text
IDL (on array-runtime) — type quit to exit.
>> x = 5
>> PRINT, x
5
>> FOR i = 1, 3 DO BEGIN
...  PRINT, i
... ENDFOR
1
2
3
>> PRINT, 1 + $
... 2
3
>> quit
```

## Testing

```sh
cargo test -p coding-adventures-idl-repl
```
