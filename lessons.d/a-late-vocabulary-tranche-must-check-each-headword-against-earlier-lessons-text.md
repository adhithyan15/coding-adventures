---
category: Testing & coverage
---

# A late vocabulary tranche must check each headword against earlier lessons' text before drafting

**What went wrong.** The Spanish B1 tranche (1,330 headwords, chapters 512–777) was drafted from a frequency list, filtered only against the headwords already taught. Its first generation pushed Spanish's forward-reference count from 622 to 1,342. That count is an exact ratchet in `tests/continuity.test.ts`, which may only fall.

Frequent words the course had never taught as headwords still appeared all over earlier lessons:
- in Spanish examples: *el año*, *pasar*, *la carretera*;
- inside phrases taught whole: *el correo* in *el correo electrónico*, *cuesta* in *¿Cuánto cuesta?*;
- as English homographs in prose: *familiar*, *plan*, *capital*, *social*.

Once a late lesson teaches such a word, every earlier use becomes a forward reference. The tranche's own examples also used words that later B1 chapters teach.

**The fix.**
- 223 headwords were replaced with words no earlier lesson uses, and the example lines that borrowed later B1 words were reworded.
- A checker mirrors `measureContinuity`'s rule: lossless body minus etymology blocks, whole word, plain prose at four or more letters, emphasis at any length.
- Every candidate headword was run through the checker before drafting.

**Do differently.** Before drafting a late tranche, build an index of every earlier lesson's forward-reference text and screen each candidate headword against it. Also screen each draft's examples against the tranche's later headwords. A word the course has used but never taught cannot be taught late without either rewording the earlier lessons or breaking the ratchet. Pick words the corpus has not used yet.
