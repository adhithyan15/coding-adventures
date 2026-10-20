---
category: Repo policy / workflow reminders
---

# Generated vocabulary must be linted for every banned word the corpus gate counts, not only the ones a brief names

**What went wrong.** The Latin B1 vocabulary briefs banned the word "just",
and every spec passed the brief's lint. The full suite then failed
`banned-words.test.ts`, whose corpus-wide debt pin may only fall: three notes
said "simply" (*cum* "is simply when", *onerāria* "or simply", *odiōsus* "means
simply annoying"). The test bans four words: *simply*, *just*, *obviously* and
*as you know*.

**Fix.** Reword the three notes. Add all four words, and the glyphs the font
lacks (ǭ, the combining ring below), to the shared spec lint script, so drafters
see them before generation.

**Next time.** When a brief lists banned phrasing, copy the list from the test
(`tests/banned-words.test.ts` `BANNED`), not from memory, and lint every spec
with it before generating.
