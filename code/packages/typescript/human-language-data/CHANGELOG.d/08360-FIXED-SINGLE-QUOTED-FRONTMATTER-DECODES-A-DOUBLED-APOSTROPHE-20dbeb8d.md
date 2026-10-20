### Fixed — single-quoted frontmatter decodes a doubled apostrophe

- **`src/frontmatter.ts`** `unquote()` now turns `''` inside a single-quoted scalar into one `'`, the single escape YAML defines for single quotes. Backslashes stay literal, as before and as in YAML. Double-quoted scalars are unchanged.
- **Why.** 167 lesson `gloss:` lines across thirteen tracks opened with a quoted English word and went on past the closing quote (`gloss: "you're welcome" — …`), which PyYAML and every strict YAML parser reject. Wrapping each in single quotes makes it valid, but 58 of them contain an apostrophe, which YAML requires to be doubled. Before this change the reader shipped the doubled form verbatim. Now the reader and PyYAML agree on every one of the 167 glosses, and 158 of them read byte-for-byte as before. The other nine also ended in a quote, so the reader used to strip a stray quote from each end; they now read as written.
- No existing frontmatter value used `''` inside single quotes, so no other field changes.
- `tests/frontmatter.test.ts` pins the decoding.
