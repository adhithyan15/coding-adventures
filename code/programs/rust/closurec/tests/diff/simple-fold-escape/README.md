# `simple-fold-escape` — legacy global `escape` / `unescape` constant-fold

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass collapses the legacy global string escapers `escape(str)` and
`unescape(str)` (ECMAScript Annex B §B.2.1.1 / §B.2.1.2) to a string literal when
the single argument is a string literal.

These are the *legacy* siblings of `encodeURIComponent` / `decodeURIComponent`,
with one structural difference: `escape`/`unescape` operate on UTF-16 **code
units**, not UTF-8 bytes. A unit below `0x100` escapes to `%XX`; a unit `0x100`
and above escapes to `%uXXXX`. The unescaped set is the ASCII alphanumerics plus
the seven marks `@ * _ + - . /`:

| call                   | result            | note                                          |
|------------------------|-------------------|-----------------------------------------------|
| `escape("a b")`        | `"a%20b"`         | space → `%20`                                 |
| `escape("~/@")`        | `"%7E/@"`         | `~` escaped (NOT a mark); `/` and `@` kept    |
| `escape("é")`          | `"%E9"`           | U+00E9 is one code unit < `0x100` → `%XX`      |
| `escape("😀")`         | `"%uD83D%uDE00"`  | one astral scalar → two surrogate code units  |
| `unescape("a%20b")`    | `"a b"`           | the inverse                                   |
| `unescape("%E9")`      | `"é"`             | emitted as `é`                           |
| `unescape("%2F")`      | `"/"`             | EVERY escape decodes (vs `decodeURI`)         |
| `unescape("%uD83D")`   | *unfolded*        | lone high surrogate → no literal → decline    |

## Soundness

`escape` / `unescape` are *free identifiers* — a local binding could shadow the
global — so we fold the **bare identifier** only, never a member access
(`window.escape` is left alone). A string literal's value is a Rust `&str` (whole
Unicode scalars), so `s.encode_utf16()` is exactly the UTF-16 unit sequence V8
escapes. Neither builtin throws; `unescape` nonetheless DECLINES the fold on the
one shape it cannot represent — a result containing an **unpaired surrogate**
(e.g. `unescape("%uD83D")`), which has no Rust-`String` / string-literal form — so
no value is ever substituted lossily.

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — eight `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a="a%20b";var b="%7E/@";var c="%E9";var d="%uD83D%uDE00";var e="a b";var f="\u00e9";var g="/";var h=unescape("%uD83D");report(a,b,c,d,e,f,g,h);
  ```

The integration test `tests/diff_simple_fold_escape.rs` runs the binary against
these flags and asserts byte-exact stdout, the per-binding folds (including the
code-unit `%uXXXX` astral case and the declined unpaired-surrogate call), and a
regression guard that the typed SIMPLE pipeline ran (not the WHITESPACE_ONLY
fallback): zero `escape(` calls and exactly one `unescape(` call remain.
