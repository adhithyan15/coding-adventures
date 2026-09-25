---
category: Repo policy / workflow reminders
---

# Moving a letter lesson after its anchor word must not add path prerequisites, and must keep every chain and revisit the letter used to carry

Five Hindi letter lessons were moved to sit right after the word that
holds their letter (gentle writing, HL-C443). The first attempt failed the
suite five ways, and each failure is a rule worth knowing:

1. **Path order.** The letter lesson was given the anchor word as a
   prerequisite and required the anchor's atom. `validateSharedSpine` then
   demanded the anchor precede the letter in `curriculum.json`, which it does
   not: the letter lessons live in the early script-recognition path segment.
   Reading order (sequence) and path order are different. A warm-up line that
   *names* the word is enough; do not link the two.
2. **Transitive chains.** ऋतु's only prerequisite was the ऋ letter lesson,
   whose own prerequisite was हाथ. Removing the letter from ऋतु silently cut
   HI-C38-aankh off from the हाथ atoms it requires. Before dropping a
   prerequisite, re-home anything that flowed through it.
3. **Revisits.** The anchor word had been one of the two revisits of the
   letter's recognition atom. Once it came first, ण had one revisit, and
   Hindi lost pre-A1 on reinforcement. Add the atom to a later lesson that
   already shows the letter (here, the meeting lesson reading प्रणाम), and add
   the letter lesson as that lesson's prerequisite.
4. **Duration.** One added warm-up sentence pushed a letter lesson's computed
   duration to 301 s, over the 300 s ceiling. Keep added lines short.
5. **Pinned intent.** `tests/corpus/hindi/script-order.test.ts` pinned the
   old "letter before word" order. When the rule changes, update the pin and
   say why; script closure is untouched because a romanized headword is
   exposure, not load-bearing.

Also: never write "just" in lesson prose (banned-words ratchet).
