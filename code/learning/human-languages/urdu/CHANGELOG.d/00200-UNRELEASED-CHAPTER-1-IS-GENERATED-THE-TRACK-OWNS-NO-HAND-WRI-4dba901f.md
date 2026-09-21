## Unreleased — chapter 1 is generated; the track owns no hand-written chapter (HL-C287)

`book/chapters/ch01-greetings-and-responses.tex` was the last hand-written
chapter Urdu held, and the only one left in the whole book. It is now generated
from its six lessons, so `handwritten_parity.py --check urdu` answers "already
retired, nothing handwritten remains" and exits 0. Urdu now owns no
hand-written chapter at all; measured against the merged tree, four tracks
still do — french, german, kannada and marathi.

- **The migration was the work.** `UR-C01-salam` was still schema v1, which
  declares no knowledge atoms, and `book.ts` refuses to generate a chapter from
  a v1 lesson — so the flip could not even be attempted first. The lesson now
  carries v2 frontmatter, `spine_node: SPINE-MEET-GREET` (the path segment it
  was already in), an `hl-knowledge` directive on every block, and one atom,
  `UR-LEX-SALAM`. **One** new atom, not three: the chapter policy allows three,
  and the etymology block introducing the headword is the shape
  `UR-C01-shukriya` already uses.
- **Four headings were re-pointed by PREFIXING, never by rewriting.**
  `Notice the direction` -> `Script — notice the direction`;
  `Meaning and history` -> `The word, taken apart` (the corpus's own name for
  an etymology block); `Use it` -> `Guided Practice — use it`, printed as
  "Your turn — use it"; `Quick recall` -> `Wrap-up Recall`. A per-character
  non-ASCII census across every edited lesson shows the only additions are em
  dashes and one `ū`, and **zero** Urdu or IAST characters lost.
- **The reported parity gap of 23 was 22 parts instrument.** `checkpoint`x6,
  `rootweb`x4, `scriptstep`x6 and `usage`x7 are environments the generator
  cannot emit, and `unportable_blocks` charges for them with no markdown side
  to subtract — but their prose was already in the lessons, under headings that
  classify to `script`, `input`, `guided-production` and `recall`. Sized instead
  by counting what the chapter TEACHES: 17 distinct Urdu tokens in the `.tex`,
  **0** owned by no lesson, 6 sections against 6 lessons. Recorded as HL-C287.
- **Five things the hand-written chapter taught and the lessons did not say are
  now authored into the lessons**, found by a content-word census of the two
  `.tex` files rather than by the block gap: the name **nūn ghunna** for `ں`;
  that **shukriyā** is an ordinary everyday modern expression and not a
  literary relic; that the safest reply to **salām** is the same word back; the
  instruction to cover the romanization before saying **salām**; and the
  contrast that made **nahī̃**'s etymology a point — unlike everything before it
  in the chapter, it is inherited Indo-Aryan rather than Persian-Arabic.
- **One paragraph is genuinely replaced, not carried.** The hand-written
  chapter opened on a note about its own construction ("builds one miniature
  exchange in dependency order... every step stays under five minutes"). The
  generator opens every chapter on `chapteropening` instead, so chapter 1 now
  matches the other seventeen. Its two substantive claims survive in the
  lessons that make them — `UR-W01-shukriya-first-three` still says tracing is
  enough, and `UR-C01-shukriya` still says copying with a model is.
- **The Hindi cousin is no longer printed in Devanagari.** `UR-C01-nahin`
  showed **नहीं**; the Urdu book's font covers no Devanagari, so generating the
  chapter turned four codepoints into `glyph-coverage` failures. The
  hand-written chapter had given the cousin in romanization only and said why —
  "without making Hindi script a prerequisite" — so the lesson had drifted and
  the hand-written chapter was right. This was invisible while the chapter was
  hand-written, because nothing rendered the lesson.
- **`UR-LEX-SALAM` is reinforced, not merely introduced.** `UR-C01-shukriya`
  already printed *salām* in both its practice and its recall; it now declares
  the atom, so the reuse is visible to the reinforcement windows.

Counters, each re-measured against the tree rather than derived:

- Urdu's lesson-content budget moves **88 -> 89** — `UR-C01-salam` becoming
  measurable, not a new lesson. Idioms, senses and culture claims are unchanged
  at 2 / 4 / 4.
- `atomsTaught` **185 -> 186**; `atomMeasurementBlindLessons` **1 -> 0**, which
  clears the track's `measurement-blind` finding outright.
- `firstWritingPracticeAt` **1 -> 0** clears the `writing-ramp` finding. This is
  the migration making existing content visible, not new content: lesson one
  always showed the four letters of **سلام**, but the heading did not classify
  as a `script` block, so `isWritingPractice` could not see it.
- `reinforcementWindowMisses` **240 -> 243** — a legitimate RISE. A newly
  declared atom is a newly measured one, and `UR-LEX-SALAM` is revisited once,
  so its R2/R4 windows are now counted as missed. The debt existed before; the
  gate could not see it while the lesson declared nothing.
- The `grouped-shards` handwritten ratchet is untouched, per its own in-file
  directive. `handwritten.d` holds **34** entries with this change in — counted
  from the directory on the merged tree, not derived. It was 38 when this
  branch was cut; Arabic ch2, German ch3 and a French chapter landed on main
  in between, so subtracting one from a remembered number would have been
  wrong by three.

Verified: human-language-data 124 test files / 1730 passing; all eleven
`check:*` gates; language-ladder 39 files / 442 passing; the whole Urdu book
compiles under XeLaTeX with zero overfull boxes, and the six changed pages were
read as rendered PDF to confirm the Nastaliq shapes, joins and the nasal
`hā̃` diacritic survive typesetting. (`check:compile` needs bash 4's `mapfile`
and macOS ships 3.2, so this went through `book-cli.js
--materialize-compile-inputs` and latexmk directly.)

