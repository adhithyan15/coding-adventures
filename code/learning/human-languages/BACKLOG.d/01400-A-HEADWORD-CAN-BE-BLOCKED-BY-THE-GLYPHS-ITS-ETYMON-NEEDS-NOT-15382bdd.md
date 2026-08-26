## A headword can be blocked by the glyphs its ETYMON needs, not by anything about the word

`la aldea` was screened, cleared and dropped from the Spanish A1 tranche 6
shortlist for a reason no existing check names: **the word it descends from
cannot be printed.**

The Academy's dictionary gives the etymology as Andalusi Arabic *aḍḍáyʿa*, from
classical *ḍayʿah*. That transcription needs `ḍ` (U+1E0D) and `ʿ` (U+02BF).
Neither has ever appeared in a Spanish `.tex` file, so neither has ever been
rendered by the font the Spanish book compiles with, and XeLaTeX drops a glyph
it has no outline for **silently** -- the page prints a different word than the
source says, and only `scan_latex_log_warnings.py` in CI notices.

The word itself is unimpeachable: `ald-` collides with nothing taught, the root
is unspent, and it would have paired well with `la alfombra` and `la almohada`
as a third noun carrying a fused Arabic article. It was replaced by `la cumbre`.

### Why this is worth recording rather than shrugging at

The three ways out are all bad, and it is worth being explicit that they were
considered:

1. **Print a simplified transcription** (`ad-daya`). This asserts a form no
   dictionary gives, in a lesson whose entire subject is where the word comes
   from. A vocabulary lesson that fudges its own etymon is worse than no lesson.
2. **Write the etymon in Arabic script.** Only six Arabic letters have ever been
   rendered in a Spanish book, and none of the four this word needs is among
   them. The same failure, one script over.
3. **Raise the `missing_character` baseline.** This blesses a character the
   reader never sees. It is the move the corpus already refused once, for
   U+0325 in a reconstructed Proto-Indo-European form.

So the honest action was to drop the word, and the honest place to record it is
here, because **nothing reports this class**. The glyph check runs on generated
`.tex` and can only fire once a lesson has been written. An author choosing
candidates has no way to know that a perfectly good headword is unavailable
until they have done the etymology research and drafted the prose.

### What would fix it

Either a vendored face for the Spanish book that covers the transliteration
range the Academy's dictionary actually uses -- dot-below consonants and the
`ʿayn`/`hamza` marks are the whole of it, and there are perhaps a dozen -- or a
documented, checkable house transliteration for Semitic etymons that stays
inside the existing repertoire and says in the lesson that it is simplified.

The first is better. Spanish has several hundred Arabic loans and this will keep
happening; `la alfombra` only got through because *alḥánbal* happens to need
`ḥ`, which the corpus had already printed.

### The general form

A content item can be blocked by a property of its **supporting material**
rather than by any property of itself, and the gate that would catch it runs
too late in the pipeline to guide the choice. When a check operates on a
generated artifact, ask what an author would need to know **before** generating
it, and whether anything tells them.
