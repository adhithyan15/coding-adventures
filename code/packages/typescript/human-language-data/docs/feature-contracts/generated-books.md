# Generated books

Generate configured book chapters, or verify the committed output is current:

```bash
npm run build
npm run generate:books
npm run check:books
```

Both modes also refuse a generated chapter that prints a delivery cue's
brackets (`{[}YOU SAY: …{]}`, `{[}PAUSE 2s{]}`, `{[}REPEAT x2{]}`): the book
voice (`bookVoice` in `book.ts`) puts every cue into print form wherever it sits
in a paragraph or list item, including a cue wrapped across source lines, and a
bracket it cannot voice is a generator bug, not something to typeset.

`core/book-generation.d/targets.d/<language>-<NNNN>.json` declares one
generated chapter's language, number, output path, and script-rendering options.
Hand-authored chapters use the same identity under `handwritten.d/`; reference
appendices, glossaries, answer keys, indexes, and ordered script sets have their
own direct-record section directories. `_meta.json` holds only `version` and
`sourceBaseUrl`. No per-language slice or flat compatibility manifest exists.

A chapter declaration's title and label come only from that track's
`chapters.d/` capability ledger; the config is rejected if it tries to repeat
either field, and a declaration without canonical chapter metadata fails
closed. Exact identity gates require targets to equal the generated-book hash
owners, targets plus handwritten chapters to equal both the generated-narration
owners and chapter capabilities, and the owner languages to equal the registry.
Registry-derived glossary, answer-key, and index identities and generated
pronunciation-reference artifacts independently close the backmatter owner sets.
The generator orders schema-v2 lessons by `sequence`, writes the LaTeX chapter,
and records a stable FNV-1a fingerprint in
`core/generated-book-hashes/<language>.d/NNNN.json`. The fingerprint covers the
chapter's lessons
**and the capability fields the book prints** (`title`, `label`, `canDo`, and
`payoff.summary`), so an
edit to a chapter shard moves it. It does not cover `payoff.note`, which the book
deliberately does not print — a fingerprint covers what the artifact SHOWS. It
detects drift between book and app inputs; it is not a security hash.
Each `NNNN.json` owns exactly one chapter; `_meta.json` carries only the ledger
version, language, and algorithm. `check:books` folds the owners in memory and
rejects missing, stale, unexpected, unsafe, or mismatched files. No flat
language manifest is generated or tracked.
The config's `sourceBaseUrl` gives every lesson a stable canonical URL, so
absolute citations and relative prerequisite/reference links remain live after
the generated PDF is downloaded.
Non-Latin targets also declare `unicodeScript` and `scriptCommand`; the renderer
wraps matching Unicode runs in the book's existing font macro and uses each
lesson's `romanization` for a PDF-bookmark-safe short title.
