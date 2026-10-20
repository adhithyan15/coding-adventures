## Chapter 1 is generated, and the duplicate writing ladder is retired (HL-C134, HL-C285)

`arabic/book/chapters/ch01-greetings.tex` is now generated from its fifteen
lessons. Getting there meant resolving something the flip exposed: **chapter 1
owned two complete writing ladders for the same five letters**, and the
hand-written `.tex` showed only one of them.

**Which ladder, and why.** `AR-W00` is interleaved — letter lessons at sequences
12, 14 … 22, 24 … 42, 44, 46, with word lessons sitting between them.
`AR-W01`–`AR-W03` ran 90, 100, 110, 120, 130, 140: six letter lessons back to
back with nothing between. That is the batching this curriculum removes
everywhere else, so `AR-W00` was kept and the six `AR-W01`–`AR-W03` lessons
retired. `AR-W03-write-salam` was titled *"سلام — your first whole written
word"* and arrived 126 sequence steps after the word had already been written.
**سلام is now written exactly once.**

Those six rendered nowhere in the book — not in a chapter, not in the answer key
— while the generated narration still scripted them for audio.

**Two teaching points were re-homed rather than dropped**, because the deleted
lessons were the only place either was taught:

- **The four-positional-shapes rule.** *"A letter wears up to four coats —
  isolated, initial, medial, final; the skeleton stays, the connectors change."*
  Chapters 4+ *assume* this (`try it isolated and final, feel it change`) and
  never state it. It now lives in `AR-W00-alif-mim-salam`, which already
  asserted *"Arabic letters join"* without ever saying what joining does, and
  which demonstrates every part of it inside **سلام**. New atom
  `AR-SCRIPT-JOINING-FORMS-04`.
- **The Phoenician letter-origin thread** for the first five letters. This runs
  through `AR-W04`, `W05`, `W07`, `W08`, `W10` and `W11`; without re-homing, the
  reader would have met it from chapter 2 onward with chapter 1's letters
  silently exempt. *sīn* ← *shin* "tooth", *lām* ← *lāmed* "ox-goad" → lambda →
  **L**, *alif* ← *ʾālep* "ox" → alpha → **A**, *mīm* ← *mem* "water" → mu →
  **M**, *bāʾ* ← *bēt* "house" → beta → **B**, and *alpha-bet* = ox + house. The
  three `AR-W00` lessons already declared these roots in frontmatter; only the
  prose was missing.

**What that cost, recorded rather than hidden.** The root ledger counts
*declaring lessons*, not prose, so five Phoenician roots plus `abjad-vowels`
dropped to zero payoffs: `neverSpent` 88 → 93, `underspent` 101 → 102. Padding
`roots:` onto downstream lessons would have moved the number without helping a
reader, so it was left honest and the pin updated with the reason. Narration
refusals 58 → 57 (a retired lesson took its wide table with it). Script closure
is **unchanged in total** (57 → 57) and chapter 1's violations are identical to
before; three chapter-2 lessons pick up glyphs that the deleted lessons had only
ever "taught" by naming them in a forward-reference sentence.

**Also in this flip.** Chapter 1's four remaining schema-v1 lessons (`al`,
`ṣabāḥ al-khayr`, `masāʾ al-khayr`, `shukran`) are migrated with hand-authored
atoms, one per teaching section. Chapter 1's atom ordinals are a chapter-wide
teaching-order sequence, so inserting the re-homed rule at slot 04 shifted the
later atoms; the sequence is now 01–22 with no gaps or duplicates.
`AR-PATH-006` and `AR-EXT-006-SCRIPT` held only the retired six and were removed,
with both shard directories renumbered contiguously and `SPINE-CHECK-WELLBEING`'s
segment ledger updated. `AR-C01-salam`'s pronunciation prose was refiled from
`## You'll want to know` to `## Sounds you'll need`, matching its two siblings.

**Chapter 2 is still hand-written.** Its parity gap of five `sounds` blocks is an
environment mismatch, not missing prose — see HL-C286.

