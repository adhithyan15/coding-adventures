---
category: Repo policy / workflow reminders
---

# A retrieval line that says the table turns a drivable review into a sight lesson

**What went wrong.** The revisit pass (`atomsNeverRevisited` 134 → 1) added short
spoken recall questions to later review lessons. One of them was "And the two
nouns **in the table**: what did Latin *casa* and *liber* first name?", added to
ES-C44-repaso-articulos. That lesson contains a Markdown table, and "the table"
is an `artifact`-anchored sight cue in `SIGHT_CUE_RULES` (`src/modality.ts`).
The cue counts when the document really has a table. So the lesson went from
`voice` to `sight`, chapter 81 lost its drivable prefix, and
`chaptersWithoutDrivablePrefixByContent` rose 8 → 9. Every validator and
duration gate stayed green. Only the gap-report diff showed it.

The same pass hit two other silent regressions:

- The examples *aunque es difícil, me gusta* and "as in *llave*" leaked
  *difícil* and *llave*, which later lessons teach as headwords, and forward
  references rose 1,173 → 1,175. See "the most natural example sentence is the
  one most likely to be in script debt".
- Plain "verb", "noun", "participle" and "case" in the new questions raised the
  metalanguage report's technical uses before introduction 6,586 → 6,596.

**Fix.** Reword the question so it names the words rather than pointing at the
page ("And *casa* and *libro* themselves: …"). Keep example sentences to words
the track has already taught. Use the term's `plainAlternative` ("word" for
*verb*) until the track introduces the term.

**Next time.** After any prose edit to lessons, diff the whole gap-report
`summary` before and after (`npm run -s report -- --format json`). Do not stop
at the metric you were moving. `forwardReferences`, `drivableLessons`,
`chaptersWithoutDrivablePrefixByContent` and `metalanguage` all move on
one-line edits that pass every other gate. Before writing a recall line, grep it
for the `SIGHT_CUES` phrases: *see the*, *look at*, *shown below*, *the table*,
*the chart*, *column*, *written above*.
