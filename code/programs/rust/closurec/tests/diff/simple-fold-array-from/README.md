# `simple-fold-array-from` — static `Array.from("…")` → array of code-point strings

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass collapses a static `Array.from(x)` call (ECMAScript §23.1.2.1)
to an array literal when `x` is a string literal and there is no `mapFn`.

| call                  | result            | note                                       |
|-----------------------|-------------------|--------------------------------------------|
| `Array.from("abc")`   | `["a","b","c"]`   | one element per code point                 |
| `Array.from("")`      | `[]`              | empty string → empty array                 |
| `Array.from("xy", f)` | *unfolded*        | a 2nd `mapFn` arg changes every element     |
| `Array.from(s)`       | *unfolded*        | a non-string-literal iterable is unknown    |
| `q.from("z")`         | *unfolded*        | only the bare global `Array.from` folds     |

## Soundness

`Array.from` on a string iterates it by **code point** — exactly the spread
`[..."…"]` behaviour — so a string literal folds to an array literal of
single-code-point strings, side-effect-free. Astral characters stay whole (one
element, not split into surrogate halves); that surrogate-pair case is covered
in the `closure-pass-constant-fold` unit tests so this fixture's output stays
ASCII. We decline: a second `mapFn` argument (its returns are unknown), any
non-string-literal first argument (array-likes / real iterables / identifiers /
numbers — their iteration result is unknown at compile time), and a shadowed
receiver (`q.from(...)`). Only the bare global `Array.from(...)` callee folds.

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — five `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a=["a","b","c"];var b=[];var c=Array.from("xy",f);var d=Array.from(s);var e=q.from("z");report(a,b,c,d,e);
  ```

The integration test `tests/diff_simple_fold_array_from.rs` runs the binary
against these flags and asserts byte-exact stdout, the two folds (`"abc"` and the
empty string), the three declines (mapFn / non-literal / non-global receiver),
and a regression guard that the typed SIMPLE pipeline ran (not the
WHITESPACE_ONLY fallback): exactly two `Array.from` calls (the two declines)
remain.
