## HL-C272 — Punjabi script closure is 0; what the runway did NOT fix

Punjabi's script-closure debt went from **40 violating lessons to 0**, its
never-taught glyphs from **8 to 0**, and its headwords lacking romanization from
**8 to 0**. The instrument was an 18-lesson recognition runway inserted into
chapters 2-13 — one or two lessons per chapter, at most three Gurmukhi pieces
each, each placed immediately before the first lesson that asks the reader to
decode them.

Two things are worth writing down, because neither is obvious from the number.

**The debt was not mostly "letters not taught yet". It was Sanskrit.** Before
any letter lesson existed, the single largest source of untaught glyphs in
chapters 2-5 was Sanskrit and PIE etymons *rendered in Gurmukhi* — ਨਾਮਨ੍,
ਅਸ੍ਤਿ, ਮਿਲ੍, ਰਹ੍, ਕ੍ਰੁ. Those forms carried the subjoining sign ੍, which no
lesson in the track ever taught, into five chapter-2-to-5 lessons. Sanskrit is
not written in Gurmukhi; putting those etymons in IAST is a scholarship fix that
happens to retire a large share of the closure debt for free. **A track whose
closure number looks structural is worth grepping for foreign-language forms
transliterated into the target script before anyone plans a script ladder.**

**Closing script closure COSTS drivability, structurally, and this is worth
knowing before the next track tries.** Punjabi went from 43% to 40% ear-drivable
and from 74 to 46 lessons reachable in chapter-prefix order.

The mechanism is a coupling between two modules that do not mention each other.
`script-closure.ts` credits a lesson with teaching a glyph only when the lesson
is `type: writing` or `delivery: script`. `modality.ts` says a `writing`-typed
lesson has a **pen core** by definition — "the whole lesson is the writing, so
there is nothing separable to set aside". And `modality-manifest.test.ts`
enforces that `delivery: script` is exactly the set of `type: writing` lessons,
in every track. Put together: **the only lessons that can retire a closure
violation are the ones modality is required to score as pen.** There is no way
to author a recognition lesson that both closes a glyph and keeps a voice core.

A first draft of this runway typed the lessons `reading` with `delivery:
script`, and the report showed drivability *rising* to 48% and prefix reach to
92. That was not a discovery, it was the manifest test catching a mislabel. The
40%/46 figures are the true ones.

What *can* be recovered is placement. The letters must arrive before the first
lesson that decodes them and **not one session earlier**; pushing every runway
lesson to the latest slot its own glyphs allow took prefix reach from 25 to 46
with closure still at 0. Any track doing this work should compute that latest
slot rather than putting the ladder at the head of each chapter.

### What is left

- **16 of the 40 new recognition atoms still miss their R3/R4 window.** Each
  recognition atom now gets its R1 neighbourhood (the runway lesson rehearses
  the three lessons before it), its R1/R2 (the early content lessons declare the
  letters their pages show), and, where a later FORMATION lesson exists for the
  same glyph, its R3/R4. Eighteen have no lesson 80-250 sessions downstream that
  puts the glyph back on a page — mostly ਣ ੜ ਘ ਝ ੍ ਟ ਧ ੱ ਈ ਵ, which have no
  handwriting lesson in chapters 14-36 at all. Writing those ten formation
  lessons is the cheapest remaining reinforcement win in the track.
- **Six Punjabi chapters now exceed the 12-new-atoms chapter budget** (the
  corpus count went from 23 chapters to 29). The runway was inserted into
  existing chapters rather than splitting them, because splitting renumbers
  every downstream chapter and its generated book. The honest fix is a chapter
  split, and it is now unblocked: `language-ladder/tests/bookhashes.test.ts` no
  longer pins per-chapter lesson counts.
- **Two lexical forward references survive and need a MOVE, not an addition.**
  `ਮੈਨੂੰ` is used in the chapter-9 liking lesson 147 sessions before
  `PA-C35-mainu` teaches it, and `ਨਾ` is used in chapter 5 160 sessions before
  `PA-C31-na`. The runway fixed their *letters*; the *words* still arrive far
  too late. The repair is to relocate those two lessons into chapters 9 and 5
  and renumber, which now costs a lesson-id rename plus its book, narration and
  path references.
- **`PA-C07-jana` cannot claim the recognition atoms its page shows**, because
  it sits in curriculum path node `0140-PA-PATH-011` while the runway lesson
  that introduces them sits in `0150-PA-PATH-011-B`. Splitting
  `PA-EXT-049` so the first runway lesson can move into node 0140 would let it.
- **Vocabulary is still 50/300 at pre-A1 and verbs 1/5.** This tranche spent its
  budget on closure and added no headwords.

### The pre-A1 verb floor cannot be reached by authoring, and here is the proof

Tamil reported that the canonical `VERB-*` concept ids are owned by A1 spine
nodes. Checked against `core/spine.d` for Punjabi, the situation is stricter
than that and it is worth stating exactly, because it means no amount of Punjabi
authoring will move the number:

**All 40 core `VERB-*` concepts are owned by eight spine nodes, and every one of
those nodes is `stage: A1` or `stage: A2`. Not one is pre-A1.**

    A1   SPINE-NAME-EVERYDAY-ACTIONS       15 VERB-* concepts
    A1   SPINE-SAY-WHAT-I-HAVE-AND-CAN-DO   2
    A1   SPINE-SAY-WHAT-I-LIKE              1
    A1   SPINE-SAY-WHAT-I-WANT              1
    A2   SPINE-SAY-WHAT-I-DO               24
    A2   SPINE-NEGATE-AND-ASK               1
    A2   SPINE-TALK-ABOUT-PAST              1
    A2   SPINE-TALK-ABOUT-FUTURE            1

A lesson's level is derived from the spine node it declares. Punjabi's sixteen
verb lessons already declare `SPINE-SAY-WHAT-I-DO` or
`SPINE-NAME-EVERYDAY-ACTIONS`, which is *why* they measure as A1+. Giving a
pre-A1 lesson a `VERB-*` concept tag would move that lesson to A1 or A2, not
move the verb into pre-A1. **The pre-A1 verb floor of 5 is unreachable for every
track until the shared spine gains a pre-A1 node that owns verb concepts** — a
`core/spine.d` change, not curriculum authoring. Any track reporting "verbs 1
against a floor of 5" is reporting the same structural fact, and no tranche
should spend effort on it before that node exists.
