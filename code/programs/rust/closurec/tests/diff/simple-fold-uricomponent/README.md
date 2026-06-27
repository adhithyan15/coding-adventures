# Fixture: `simple-fold-uricomponent`

End-to-end oracle for global `encodeURIComponent(…)` / `decodeURIComponent(…)`
folding on string literals at `--compilation_level SIMPLE`.

| File | Role |
|------|------|
| `flags.txt` | CLI args: `--compilation_level SIMPLE --js input/a.js` |
| `input/a.js` | six URI-component calls — five foldable, one declined |
| `expected.stdout` | The folded output (see below) |

The SIMPLE level runs the typed-AST optimization pipeline, whose
`constant-fold` pass folds a global `encodeURIComponent(str)` /
`decodeURIComponent(str)` call whose single argument is a string literal
(ECMAScript §19.2.6.5 / §19.2.6.3), modelled like the sibling
`parseInt`/`parseFloat` free-identifier folds:

- `encodeURIComponent("a b")` → `"a%20b"` — space becomes `%20`;
- `encodeURIComponent("é")` → `"%C3%A9"` — each UTF-8 byte is percent-escaped
  with uppercase hex;
- `encodeURIComponent("/")` → `"%2F"` — the URI **reserved** delimiters
  (`; , / ? : @ & = + $`) that `encodeURI` leaves intact ARE escaped here;
- `decodeURIComponent("a%20b")` → `"a b"` — the inverse;
- `decodeURIComponent("%C3%A9")` → `"é"` (emitted as `é`);
- `decodeURIComponent("%E0")` → **left intact** — `%E0` is a truncated
  multi-byte lead, an invalid UTF-8 byte run on which JS throws `URIError`; the
  fold declines rather than fold a runtime throw.

So the folded `expected.stdout` is:

```js
var a="a%20b";var b="%C3%A9";var c="%2F";var d="a b";var e="é";var f=decodeURIComponent("%E0");report(a,b,c,d,e,f);
```

Only the **bare global identifier** folds — a member access like
`window.decodeURIComponent(...)` is left for the runtime, as is any argument
that is not a string literal or a second argument. The same input under
`WHITESPACE_ONLY` keeps every call intact.

Regenerate the expected file after an intentional behavior change:

```sh
cargo run -- --compilation_level SIMPLE \
    --js tests/diff/simple-fold-uricomponent/input/a.js \
    > tests/diff/simple-fold-uricomponent/expected.stdout
```
