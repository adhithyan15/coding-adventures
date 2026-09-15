# A lesson's `gloss` and `etymology_hook` reach the generated `.tex`, so prose gates read them — and `npm run validate` does not

`standalone-book`'s cross-volume claim scan (the "the course"/"the curriculum" ban,
lessons above) runs over the **generated book surfaces**, not over lesson body blocks.
Frontmatter `gloss` and `etymology_hook` are rendered into those surfaces, so a phrase
that is forbidden in prose is equally forbidden in those two fields — and neither the
`banned-words` scan (which reads `blocks[].markdown`) nor the 18-test corpus validator
(`npm run validate`, i.e. `tests/integration.test.ts`) looks at them.

A review lesson shipped on this basis: it passed `validate` cleanly, and only the full
`vitest run` caught `this course` twice in its frontmatter. **`npm run validate` is not
the gate; it is one of 105 test files.** Run the whole suite before believing a new
lesson is clean, and remember that "learner-facing prose" for the purposes of any given
gate is whatever *that* gate's surface is — the corpus has at least three different
answers (block markdown, generated `.tex`, narration export).
