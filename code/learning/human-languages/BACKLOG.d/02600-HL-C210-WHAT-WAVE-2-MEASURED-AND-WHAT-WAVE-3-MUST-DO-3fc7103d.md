## HL-C210 — What wave 2 measured, and what wave 3 must do

Twelve per-track agents ran twice against the Indic tracks. Wave 2's value is
less in the lessons it added than in four measurements that redirect the work.
Recording them so wave 3 does not rediscover them.

### 1. Closure is measured in READING ORDER — the first brief was wrong

Wave 1 told every agent "each glyph you teach removes closure violations."
False, and two tracks proved it independently: **Bengali added 35 lessons and
closure moved 65 -> 65; Urdu added 20 and moved 46 -> 46.** A glyph taught in
chapter 17 is still untaught for every chapter 1-15 lesson that shows it.

Wave 2 resequenced instead, and it worked: **Malayalam 19 -> 1** by moving 14
letter lessons earlier and writing 14 more so the first thirty characters land
inside chapters 1-5.

That was only possible because the 388 `[chapter, lessonCount]` pins in
`language-ladder/tests/bookhashes.test.ts` were removed. Before that, Malayalam
was forced into chapters 32+ and Bengali was blocked from a measured 65 -> 48.
**A test fixture was making pedagogical decisions.**

### 2. Urdu's remaining debt is PROSE, not ladder

Simulated the entire remaining alphabet taught at policy pace: **26 letters ->
40 violations; 32 letters -> 40.** The floor is 40 because eight glyphs
(ب چ ئ ز ح ظ ق ع) are first demanded at content lesson 27, inside chapter 7, and
no ladder reaches there without becoming the course.

So the lever is changing the example WORDS in those lessons, not teaching more
letters. Eight further glyphs already have pedagogy in content lessons that earn
no closure credit, and four "untaught" glyphs (ه ء ◌َ ◌ِ) are Arabic/Persian
etymon citations the cousin-layer rule fails to exempt — those are a rule gap,
not typos, and must not be "fixed" as content.

### 3. The fifth-return slab works, and should propagate

The corpus teaches glyphs and abandons them: a `SCRIPT-RECOG` atom appears in
about **2** lessons, a `LEX` atom in about **14**. Against the owner's
"reviewed constantly" directive that is a structural hole, not a per-track debt.

Gujarati paid it down and grew at the same time: **retrieval misses 339 -> 283,
R4 101 -> 43, atoms never revisited 5 -> 1**, while vocabulary went 52 -> 72 and
the ear-drivable share rose 56% -> 65%. The instrument was one coherent chapter
of **nine zero-new-atom lessons** returning material 98-104 positions out, plus
a named distant band per later chapter.

Malayalam did a cheaper version — adding a *Script check* block to five opening
checkpoints put 26 `SCRIPT-RECOG` atoms under a payoff that assessed nothing
before.

**Wave 3 should run the slab on every track that has grown past its R4 window.**

### 4. Interleaving costs drivability unless writing is detachable

Urdu's hands-free chapter-prefix reach fell **60 -> 44 lessons**, its chapters
over the 12-atom budget rose 23 -> 26, and payoffs under the representativeness
floor 80 -> 81. Letters carry atoms, and a `pen` lesson truncates the voice-only
prefix of the chapter it lands in.

Gujarati and Marwadi avoided this by keeping the letter work in a **detachable
`Writing —` block** so the lesson retains a voice core. That is the required
shape for any further resequencing, not an optional nicety.

### Ranked for wave 3

1. **Fifth-return slabs** on every track past its R4 window — serves "reviewed
   constantly" directly, adds no script debt, and is proven.
2. **Retrofit detachable writing segments** to the letter lessons Urdu moved, to
   recover the 16 lessons of hands-free reach.
3. **Urdu prose pass** — rewrite the example words behind the 40-violation floor.
4. **Gujarati glyph lessons** — words spellable inside the taught 43 are running
   out; **ઠ** alone unlocks numbers six to ten. Each placed before first use.
5. **Malayalam chapter 1 split** — the last violation needs ഇ ല ശ taught inside
   a chapter already at 12/12 atoms. Blocked on `chapter-references.test.ts`,
   which pins 46 Malayalam cross-chapter prose references that would silently
   rot. Its own PR, three parts.

### Known write-locks still standing

`tests/integration.test.ts` holds 7 per-chapter lesson-count pins in a shared
multi-language loop — the same class as the 388 already removed, and it forced
Urdu to edit a shared file this wave. `tests/verbs.test.ts` pins
`meanCoveredPercent`, a corpus-wide mean that moves whenever any track adds
canonical verb tags. Neither blocks authoring today; both will keep biting.

### One cross-track decision nobody may take alone

Tamil, Gujarati and Punjabi each hit the same wall: **the canonical `VERB-*`
concept ids are owned by spine nodes staged at A1/A2**, so a pre-A1 track that
claims them relocates its own lessons off pre-A1 and moves the gate by zero.
Three tracks independently reached "not mine to fix." It needs a shared-spine
decision.
