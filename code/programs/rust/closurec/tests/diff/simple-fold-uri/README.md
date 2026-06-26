# `simple-fold-uri` — global `encodeURI` / `decodeURI` constant-fold

End-to-end fixture proving that, at `--compilation_level SIMPLE`, the typed
constant-fold pass collapses the global whole-URI escapers `encodeURI(str)` and
`decodeURI(str)` (ECMAScript §19.2.6.4 / §19.2.6.2) to a string literal when the
single argument is a string literal.

These are the *whole-URI* siblings of `encodeURIComponent` / `decodeURIComponent`
(see `simple-fold-uricomponent`). The only difference is their treatment of the
URI reserved/structural delimiters `; , / ? : @ & = + $` and `#`:

| call                     | result      | note                                       |
|--------------------------|-------------|--------------------------------------------|
| `encodeURI("a b")`       | `"a%20b"`   | space → `%20`                              |
| `encodeURI("a/b?c=d")`   | `"a/b?c=d"` | reserved delimiters KEPT (vs `…Component`) |
| `encodeURI("é")`         | `"%C3%A9"`  | each non-ASCII UTF-8 byte percent-escaped  |
| `decodeURI("a%20b")`     | `"a b"`     | `%20` not reserved → decoded               |
| `decodeURI("%2F")`       | `"%2F"`     | `/` reserved → escape KEPT (vs `…Component`)|
| `decodeURI("%C3%A9")`    | `"é"`       | emitted as `"é"`                      |
| `decodeURI("%E0")`       | *unfolded*  | truncated multi-byte → `URIError` → decline|

## Soundness

`encodeURI` / `decodeURI` are *free identifiers* — a local binding could shadow
the global — so we fold the **bare identifier** only, never a member access
(`window.encodeURI` is left alone). A string literal's value is a Rust `&str`
(whole Unicode scalars), so the bytes emitted are exactly the UTF-8 bytes V8
encodes; there is no lone-surrogate input (the only `encodeURI` throw) to hit.
`decodeURI` DECLINES the fold on exactly the two `URIError` inputs — a malformed
`%XX` escape and a `%`-decoded byte run that is not valid UTF-8 — so a runtime
throw is never folded into a value.

## Files

- `flags.txt` — CLI flags (`--compilation_level SIMPLE --js input/a.js`).
- `input/a.js` — seven `var` bindings flowing into `report(...)` so each stays
  referenced past remove-unused-vars and the fold is observable.
- `expected.stdout` — the byte-exact SIMPLE output:

  ```text
  var a="a%20b";var b="a/b?c=d";var c="%C3%A9";var d="a b";var e="%2F";var f="\u00e9";var g=decodeURI("%E0");report(a,b,c,d,e,f,g);
  ```

The integration test `tests/diff_simple_fold_uri.rs` runs the binary against
these flags and asserts byte-exact stdout, the per-binding folds (including the
reserved-preservation distinction and the declined `URIError` call), and a
regression guard that the typed SIMPLE pipeline ran (not the WHITESPACE_ONLY
fallback): zero `encodeURI(` calls and exactly one `decodeURI(` call remain.
