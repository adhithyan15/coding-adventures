## Chapter 19 becomes two chapters, and French owns no hand-written chapter but one

Chapter 19 (To Be, and the Past That Takes It) was hand-written LaTeX over
**four** schema-v1 lessons holding a six-cell paradigm, three stem etymologies,
the *être*-verb list with two exceptions, two agreement rules and the entire
pronominal system — roughly **nineteen atoms** against a ceiling of twelve.
HL-C286 flagged this chapter in advance as the last landmine in the French set.

The chapter's own structure supplies the seam — the verb, then the past built on
it:

| chapter | what it holds | atoms |
|---|---|---|
| **19 — To Be, and the Three Verbs Inside It** | the six forms, the three stems, suppletion, *ser*/*estar* | 10 |
| **20 — The Past That Takes *être*** | the verb list, both corrections, both agreement rules, the pronominals | 9 |

Twenty-two lessons replace four, and every French chapter after the split
renumbered by **+1** (old 20–36 → 21–37).

### The chapter's real subject, which one lesson could not carry

*Être* has no pattern because **it is three verbs**: **es-** from *esse* for the
present and infinitive, **fu-** from *fuī* for the *passé simple* — the same root
as English **be** — and **ét-** from *stāre*, "to stand," which supplied a whole
limb of participles and the imperfect. The name for that patching is
**suppletion**, and English does it too, in *go*/*went* and *am*/*be*/*was*.

That reframes the verb entirely. The honest answer to "why is *être* so
irregular?" is not "memorise it" but "because it is assembled from three verbs,
and there was never a pattern to find."

### The landmine, and why re-measuring mattered

`FR-C16-passe-compose-etre` was one of only **two** full paradigm grids in
French, and retiring it drops `info-dump`'s `fullParadigmGrids`. The pin was
lowered rather than deleted, with the reason beside it.

But the first draft of the replacement **scored 21 again**. `FR-C16-esse` had
lined the six present forms up against their Latin in a five-row table and built
a **new** full grid while removing the old one. An etymology lesson is still a
lesson, and a person-row table is still a paradigm. The pairs are prose now, and
the measured figure is **20**.

Computing *previous − 1* would have written down the right number for the wrong
reason and shipped a fresh grid underneath it.

`FR-C05-parler` is in chapter 5, generated long ago and never in scope, so the
**named fixture needed no inversion** — it still fires, and the assertion still
reads `toBe(true)`.

### Three defects only the gates and the page could show

- **A glyph that does not exist in the font.** Two lessons and a chapter title
  used capital **Ê** (U+00CA), which is *not* in Latin Modern Roman's charset
  even though lowercase **ê** is. It would have reached the reader as a missing
  glyph. Reworded to avoid the character.
- **Two rule statements over the ceiling.** *"the rule is about the sentence,
  not the verb"* is the *the-rule-is* shape `info-dump` flags, in two lessons.
  Rewritten as *"what decides it is the sentence"* — the ceiling of 30 held.
- **A metaphorical sight-cue.** *"once you have the idea the table mostly writes
  itself"* referred to a **textbook's** table of exceptions, but the classifier
  read it as pointing at the page and marked the lesson `sight`, breaking the
  chapter's hands-free run. Chapter 20 now prints *"all 10 lessons."*

### Where French now stands

**No hand-written French chapter remains.** Chapter 2, the last one outside this
scope, was retired on a parallel branch and met this work in the merge that
followed. Every chapter of the French book is now generated from its lessons.

