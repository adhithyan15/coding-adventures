# warrant-corpus-conformance-v1

The shared conformance cases every Warrant corpus lane runner must pass
(VER04i section 9.1, backlog WD-100). One JSON file per category; `index.json`
lists them with case counts and SHA-256.

| File | Cases | What a lane runs |
|---|---|---|
| `json.json` | strict reader + canonical writer | `input_hex` bytes through the reader then the renderer |
| `tagged.json` | tagged-value decoder + stream encoder | `input_hex` through reader, decoder, encoder |
| `containment.json` | containment helper | `root` and `child` as text, with the `flavor`'s path rules |
| `label-digest.json` | manifest digest | `manifest` object to its SHA-256 |

Input bytes are lowercase hex, never JSON strings (a JSON string cannot carry
invalid UTF-8, a lone surrogate or a BOM). `.gitattributes` (`* -text`) keeps
every byte exact on Windows checkouts.

Do not edit the JSON by hand. The case tables live in
`code/scripts/warrant_corpus_conformance.py`, whose independent reference
implementation derives every expected outcome:

```text
python3 code/scripts/warrant_corpus_conformance.py          # check
python3 code/scripts/warrant_corpus_conformance.py --write  # regenerate
```

Cases are only ever added within `-v1`; changing an expectation means `-v2`.
Lanes: Python and TypeScript run all four categories. The Go lane (WD-103) merged before
the fixture existed and adopts it in a follow-up.
