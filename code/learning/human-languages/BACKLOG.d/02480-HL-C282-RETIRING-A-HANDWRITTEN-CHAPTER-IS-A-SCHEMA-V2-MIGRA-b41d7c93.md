## HL-C282 — Retiring a handwritten chapter is a schema-v2 migration, not a prose carry

`handwritten_parity.py` measures the right thing and measures it well, but its
number is the **smaller half** of the job, and planning a retirement off that
number alone will underestimate it by an order of magnitude.

French chapters 3, 4 and 5 are now generated. Getting there cost:

| work | size |
|---|---|
| prose blocks carried (what parity counts) | 13 |
| lessons migrated v1 -> v2 (what parity does not count) | 20 |

`renderBookChapter` throws `generated books require schema version 2` for any
lesson in a target chapter. So a chapter cannot be flipped until **every** one
of its lessons has typed knowledge atoms, per-block `hl-knowledge` boundaries,
a declared duration, a spine node, skills/modes/strands, and a curriculum-path
placement. That is the actual reason these chapters never moved, and it is
invisible to the parity gate.

### The prose is usually not missing — it is mis-homed

This was the most useful finding and it should change how the next slice is
estimated. Almost none of the 13 blocks had to be written. The owner's prose was
already in the lessons, sitting under headings `classifyBlock` does not map, so
it rendered as a plain `\subsection*` instead of a coloured box:

- `## Across the family — thanks, two different metaphors` (should be `culture`)
- a `### A goodbye that *sees*` nested **inside** an etymology block
- the `rien`/`nada` and `trabajar` cross-language comparisons welded onto the
  tail of an etymology or a grammar lens

So carrying prose across is mostly renaming and splitting headings, and the
writing survives verbatim. Budget the v2 migration, not the prose.

### Three traps worth writing down

1. **The atom budget is the binding constraint, not the gates.**
   `maxNewAtomsPerLesson` is 3 and `maxNewAtomsPerChapter` is 12. Chapter 3 has
   seven lessons and landed on exactly 12. A block may introduce nothing and
   still assess — that is the release valve, and it is what keeps `parler`'s
   five candidate atoms down to three without splitting the lesson.
2. **Curriculum path order is the spine ladder, not the book order.**
   `FR-C03-practice` depends on every chapter-3 lesson, but the node holding
   `merci`/`de rien` (`FR-PATH-012`) sorts **after** the node holding
   `comment ça va` (`FR-PATH-011`). Placing practice on 011 produced
   `curriculum-prerequisite-order` errors. A chapter's practice lesson belongs
   on the LAST path node the chapter touches, and its `spine_node` has to follow.
3. **Latin Modern Roman has no `À`, `Ü`, `Ê`, `Ï`, `Ë` or `Ô`.**
   Handwritten `.tex` wrote these as LaTeX escapes (`\`{A}`), so
   `glyph-coverage.test.ts` never saw them. A generated chapter emits the
   character itself and the gate goes red. Check lesson sources for precomposed
   accented capitals and for stray IPA **before** flipping, not after.

### Where French stands

- **Retired (generated): 3, 4, 5.** Parity 60 -> 47.
- **Prose-ready, still handwritten: 1, 2** — both already at gap 0, and
  `FR-C01-bien`'s font gap is already fixed. They need only the v2 migration:
  24 lessons, of which the four `FR-W01-salut-*` writing lessons are v2 already.
  Chapter 1 is the one the owner opened and complained about, so it is the
  highest-value next slice.
- **Still holding prose: 7 (1), 8 (1), 9 (2), 10 (2), 11 (4), 12 (4), 13 (8),
  14 (8), 15 (8), 16 (9).** Chapters 13-16 are the heavy ones and their lessons
  hold no mapped blocks at all (`md 0`), so those are genuine writing, not
  re-homing.
- **Chapter 6 is not in the parity table at all** and has never been checked;
  it is handwritten with no prose blocks, which the script itself warns is a
  weaker claim than "safe".
