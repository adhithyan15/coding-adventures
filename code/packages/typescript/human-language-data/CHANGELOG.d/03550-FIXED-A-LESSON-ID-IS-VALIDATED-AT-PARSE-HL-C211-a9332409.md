### Fixed - a lesson id is validated at parse (HL-C211)

- `lessonId` is interpolated RAW into `\label{lesson:<id>}` and into the `% canonical-lessons:` header of every generated `.tex` — the sibling of the `chapters.json` label hole closed in HL-C209, and found by the security review of the very next tranche.
- Demonstrated: an id of `X}\write18{id}{` closed the brace and emitted a live control sequence into a file XeLaTeX compiles in CI. Builds run without `--shell-escape` so `\write18` is refused, but `\input` and `\openout` are not — an arbitrary local file read into a published PDF.
- `parseLesson` now rejects any id outside `/^[A-Za-z0-9]+(?:-[A-Za-z0-9]+)*$/`, the same shape `ACTIVITY_ID` has always enforced. All 2,823 corpus ids pass.
- Three existing tests had to move rather than be relaxed: two control-character tests were laundering a hostile id through `parseLesson`, which now refuses it, so they set the field directly and keep the render helper as their subject. `info-dump.ts` reads the RAW frontmatter id rather than the validated one, and that test now poisons the field the helper actually reads — which is the reason that helper still needs its own guard.
- One book-cli fixture was rewriting `id: TEST-C01-hello` into a non-ASCII id by a blanket `replaceAll`; narrowed to the headword, gloss and body it was actually testing.


