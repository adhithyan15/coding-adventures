## Unreleased — Tamil has no hand-written chapters and no closure debt left

Chapters 1-5 were hand-written LaTeX that the book generator skipped entirely,
so no lesson-level gate had ever touched them. They are now generated from their
lessons, and with the whole opening finally inside the pipeline the script-order
work that had been blocked behind it could land in the same pass.

```
hand-written chapters (tamil)     5 -> 0     (corpus 69 -> 64)
script-closure violations        21 -> 0     (corpus 357 -> 336)
never-taught glyphs                0 -> 0    (held)
headwords without romanization     0 -> 0    (held)
drivable reach                   208 -> 208  (held)
atoms never revisited             50 -> 54
lessons                          316 -> 317
```

### 1. Carrying the prose, then flipping the ledger

`handwritten_parity.py tamil` reported **5 blocks at risk**: one `culture` in
chapter 1 and four `cognates` across chapters 1, 2 and 4. Each was tracked to
where its content actually lives before anything was flipped.

* The two chapter-1 `cognates` tables — "hello" and "thanks" across the family —
  were **already carried**, verbatim, into `TA-C01-vanakkam-family-register` and
  `TA-C01-nandri-family-register`. The parity script could not see them because
  their heading, `## Across the family`, is not one of the four it classifies.
* The chapter-2 and chapter-4 `cognates` tables survived only as run-on prose
  inside `TA-C02-peyar` and `TA-C04-poy-varugiren`. **Both are restored as
  tables**, which is how they read in the LaTeX and is easier to scan than the
  sentence they had been folded into.
* The missing chapter-1 `culture` block was the "why Tamil won't just say I'm
  leaving" passage. Its farewell content is carried by `TA-C04-poy-varugiren`,
  and chapter 1 had deliberately deferred it with a "promise for later" note.
  That note is now a real `## Why it's said this way` section which explains the
  deferral to the reader instead of merely announcing it.

**`cognates` is dropped as an environment, not as writing.** The generator has no
`cognates` path, so those tables now render inside `cousinweb`, whose box title —
"The word, taken apart — its roots and family" — is the right home for a cognate
table. Nothing is lost but a violet border.

### 2. Schema v2, which is what had actually blocked the flip

`generate:books` refuses a schema-v1 lesson, and 32 of the 33 lessons in
chapters 1-5 were v1. `migrate_schema_v2.py --apply` handled 28; the four chapter
recaps needed a hand-chosen spine node first, and now sit on new
`TA-EXT-00{6,7}` / `TA-EXT-01{1,2}-CONSOLIDATION` extension nodes attached to
their own path segments, mirroring `TA-EXT-013-CONSOLIDATION`.

24 section headings across 16 lessons were renamed to headings `classifyBlock`
recognises — `The word` → `You'll want to know: <headword>`, `Across the family`
→ `The word, taken apart — across the family`, `Where the word fits` → `Why it's
said this way`, the recap sections to `What you've built — …` or `Guided Practice
— …` by whether they drill or summarise. The generator rejects an unknown block,
and that rejection was right: these were all real block types wearing local names.

Two content defects surfaced on the way and were fixed rather than carried:

* `TA-C01-nandri`'s pronunciation cue read *naṇ-ṛi* — retroflex, contradicting
  both its own romanization `naṉṟi` and its own next sentence naming the second
  *n* as **alveolar**. It reads *naṉ-ṟi* now.
* Three chapter-1 lessons held decomposed `n`+U+0323 sequences left by an earlier
  LaTeX carry. NFC-normalised.

Chapters 2-5 also carried a placeholder payoff whose note said "this legacy
schema-v1 chapter has no typed knowledge atoms yet" and whose summary printed a
lesson id at the reader (*"Complete TA-C04-practice, the canonical closing
checkpoint…"*). The migration made the note false, so all four have real
reader-facing payoffs and real `assesses` sets, at representativeness 1.00, 1.00,
0.83 and 0.56.

### 3. The chapters 1-6 script decision

Nineteen of the twenty-one closure violations sat in chapters 1-6, and every one
of them was the same defect: a Tamil word printed in **body prose or a recap
table**, always beside its own romanization, whose letters the script strand does
not reach for another thirty lessons. `வணங்கு` in chapter 1's etymology.
`உண்டு`. `நான் நலமாக இருக்கிறேன்`. A five-row recap table in `TA-C01-practice`
printing four words in Tamil on the same page as the sentence *"Nothing asks you
to read or write the other shapes."*

**The decision: chapters 1-6 are the sound-first opening, so Tamil script appears
there in exactly one place — the lesson's own romanized headword, which the
exposure rule already covers. Every other Tamil word in those chapters is printed
in romanization only, and gets its glyphs back later when the script strand
reaches it, one letter at a time.**

The alternative was measured and rejected. Retiring those nineteen by *teaching*
instead would need roughly thirty pen lessons inside the first six chapters,
because `script-closure.ts` credits a glyph only to a `type: writing` /
`delivery: script` lesson and `modality.ts` scores a `writing` lesson as pen **by
definition** — so a lesson that can retire a violation is a lesson that costs
drivability. That inverts the gentle ramp to buy back something the reader was
never asked for: these chapters promise, in their own words, that you can say
everything in them without reading a letter.

One exception is kept and is the point of the exposure rule: the headword stays
in Tamil, the way a child sees a shop sign years before reading it. So does
`வணக்கம்` in the chapter-1 recap — every one of its shapes *is* taught, by the
chapter's own tracing lesson, which is exactly what that recap's prose claims.

### 4. உ — the one Tamil letter with no lesson

The last two violations, `TA-C09-mannikkavum` and `TA-C32-saappidu`, both wanted
the independent vowel **உ**. Its *sign*, ◌ு, is taught in chapter 4; the letter
had no `TA-S` lesson anywhere in the track.

`TA-S125-letter-u` teaches it at sequence 492 — chapter 8, immediately after
`TA-S112-letter-e` and immediately before chapter 9 needs it. **Placed as late as
its own glyphs allow while still paying forward**, per the Punjabi finding: it
lands after chapter 8's voice-cored opening, so chapter 8's drivable prefix is
unchanged and the track's drivable reach holds at 208 while gaining a pen lesson.

Every other shape in `உங்கள்`, `உம்` and `உண்` was already taught at that point,
so this one letter makes all three fully decodable. **Closure violations: 2 → 0.**

### 5. Reinforcement, and an honest accounting

The v2 migration assigns one atom per lesson, so it added 33 atoms that nothing
else practised — `atomsNeverRevisited` went **50 → 82** and retrieval misses
**981 → 1109** the moment the migration landed. Wiring the five chapter recaps to
practise their own chapter's atoms, block by block rather than by frontmatter
claim, brought that back to **54** and **1084**.

That is still worse than where the track started, and the number says why: R3
(346) and R4 (285) are distance windows, and a chapter recap is three to ten
lessons away from what it reviews. Closing those needs a distant-review band —
the Gujarati shape, a chapter of zero-new-atom lessons returning material 90-plus
positions out. Filed as **HL-C246**; it is the next thing this track should do.

### Pins moved, and why

* `grouped-shards.test.ts`: `handwritten.d` 69 → 64. That literal exists to see
  exactly this flip; its own comment says so.
* `script-closure.test.ts`: the corpus ceiling, re-measured to **336**. `main` had already
  flipped this assertion from a floor to a ceiling for the same reason — the
  Marathi runway made a floor on debt fail because debt was paid — and kept a
  comparison canary (`violations > paceViolations * 5`) so the ceiling cannot
  pass on a degenerate zero. That version is taken whole; only the number moves,
  which is what its own comment asks of whoever changes it.
* `tests/corpus/tamil.test.ts`: `violations` pinned **at zero**, not re-pinned to
  a smaller count. There is no longer a backlog for a new violation to hide in.

