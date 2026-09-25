## HL-C424-76d7dbdd — Clearing Telugu pre-A1 is three blockers, not 79 words, and adding the words makes one of them worse

**Status: CLOSED (2026-09-25) — Telugu attains pre-A1.** The trap below was
avoided by building revisits into each vocabulary tranche (chapters 84-99, 80
words, each word lesson practising the two atoms before it and each tranche
closing on two review lessons), so reinforcement never reopened. Found by reading the per-track ladder the gap report now prints
(`#15904`), then reading the track's full blocker list rather than only its
worst one.

Telugu is the **nearest track in the corpus to a rung it has not cleared**, and
that makes it the cheapest real progress available — which is why it is worth
costing properly before authoring anything.

### What the ladder says, and what it omits

The ladder row prints the worst blocker only:

```
telugu   touches A2  complete none  working pre-A1  vocabulary 221/300 — vocabulary short 79
```

Read alone, that says "author 79 headwords". The full blocker list says
otherwise:

| criterion | short | detail |
|---|---:|---|
| vocabulary | 79 | 221 distinct headwords at or below pre-A1, against 300 |
| reinforcement | 50 | 50 atoms at or below pre-A1 revisited fewer than twice (46 etymology hooks waived) |
| atom-budget | 1 | `TE-C08-dayachesi` teaches 4 atoms against a budget of 3 |

All three must clear. §3.1 is a conjunction.

### The trap: the obvious fix makes the second blocker worse

Every new content lesson introduces at least one atom, and criterion 4 requires
**every** atom at or below the level to be revisited at least twice. So 79 new
headwords do not merely leave `reinforcement` at 50 — they add up to 79 more
atoms that each need two revisits, taking the reinforcement debt to roughly
**130 atoms / 260 revisit slots**.

A tranche of 79 vocabulary lessons would move `vocabulary` to 0 and
`reinforcement` from 50 to ~129. The track would be *further* from pre-A1 than
it is now, by the gate's own arithmetic, while the headline number looked
finished. That is the same shape as HL-C421: a real number, read as the whole
answer.

### What is actually needed

1. **~79 new pre-A1 headwords**, each introducing as few new atoms as it can.
2. **Two revisits for every new atom.** `practice` and `review` lessons are the
   instrument: `constants.ts` excludes them from `CONTENT_TYPES`, so they
   satisfy reinforcement **without** inflating the headword count or the atom
   budget. This is the load-bearing fact for planning the tranche.
3. **Two revisits for the 50 existing under-revisited atoms**, which are mostly
   `TE-LEX-C01-*` and `TE-GRAMMAR-C01-*` — chapter 1 material introduced once
   and never returned to.
4. **Fix `TE-C08-dayachesi`**, 4 atoms against a budget of 3.

### Adjacent, and deliberately not folded in

`ramp.chapters` reports **five** Telugu chapter-level budget violations, worst
`chapter 2 at 27 atoms against a budget of 12` across 18 lessons. Those are not
counted by §3.1's criterion 3, which measures per-lesson, so they do not block
pre-A1 — but a chapter carrying more than twice its atom budget is a pacing
defect on its own terms, and chapter 2 is where a Telugu reader starts.

### Why this is worth doing first anyway

Telugu (79) and Tamil (92) are the two tracks nearest the first structurally
complete level **any** non-pilot track would reach. Of the 23 tracks, only
Spanish has cleared a rung, and only A1. The other 22 are all blocked at
pre-A1, all on vocabulary as their worst criterion — so whatever shape this
programme takes for Telugu is the template for twenty-one more, exactly as
Spanish was meant to be.

Tamil's blockers have the same three criteria in the same order
(92 / 73 / 1), so the estimate transfers.

**Do not open this as a 79-word tranche.** The reinforcement arithmetic above is
the reason, and it is measured rather than argued.

---

### The atom-budget blocker, attempted and costed (2026-09-23)

`TE-C08-dayachesi` introduces **four** atoms against a budget of three:
`TE-LEX-…-01`, `TE-ETYMON-…-02`, `TE-PRAGMATICS-…-03`, `TE-SCRIPT-…-04`.

It is the **only** Telugu content lesson that introduces four, and `SCRIPT` is
the rarest kind in the track by an order of magnitude — 272 LEX, 80 ETYMON, 44
GRAMMAR, 14 PRAGMATICS, 2 SCRIPT across all `TE-C*` lessons. Telugu keeps script
knowledge in a dedicated `TE-S*` track owning `TE-SCRIPT-RECOG-*`.

**So the obvious fix is to stop introducing the script atom and practise the
already-taught vowel-sign atoms instead. Tried it; the validator refused it, on
two counts, and both are worth writing down.**

1. **The atom is load-bearing, not redundant.** `TE-C09-kshaminchandi`,
   `TE-C39-tii` and `TE-C74-and` all require and assess
   `TE-SCRIPT-C08-DAYACHESI-04` — which is why it already carries 3 revisits.
   Deleting it raised 14 errors across those three lessons
   (`schema-v2-knowledge-not-closed`, `schema-v2-unknown-practised-knowledge`,
   `schema-v2-block-knowledge-not-closed`). It is the "a Telugu vowel's length
   and quality ride on the consonant" idea, and three later lessons build on it.
2. **Availability is by PREREQUISITE CLOSURE, not sequence.**
   `TE-S120-vowel-sign-ee` (seq 55) and `TE-S08-vowel-sign-i` (seq 125) both
   precede `dayachesi` (seq 350), and practising them still failed with
   `schema-v2-practice-before-introduction`, because `dayachesi`'s prerequisites
   are `[TE-C01-avunu]` and the script lessons are not in that closure. A lower
   sequence number does not make an atom available.

#### What the fix actually is, and why it is cheap

The lesson is over budget because it does **two** jobs: teach దయచేసి, and teach
the vowel-sign mechanic. Splitting it is the honest fix, and the track's shape
makes it unusually clean:

- **Chapter 8 contains exactly one lesson**, so a sibling can be inserted
  without disturbing a chapter's internal order.
- **All three dependents name `TE-C08-dayachesi` directly** in their
  prerequisites, so if the new lesson is a prerequisite of `dayachesi`, every
  dependent still reaches the script atom transitively — no dependent needs its
  prerequisites changed.

Shape of the change: a new `writing`-type lesson in chapter 8 (sequence ~345,
prerequisites `[TE-C01-avunu]`) that introduces the script atom, carrying the
prose already written in `dayachesi`'s *"Script you'll notice"* block;
`dayachesi` gains it as a prerequisite, drops to three introduced atoms, and
**practises** the script atom. A `writing` lesson is exempt from
`CONTENT_TYPES`, so it adds no headword and does not move the vocabulary
blocker. Renaming the atom to match its new owner touches the three dependents;
keeping the old id avoids that at the cost of a misleading name.

**Not done here** because it is a lesson-authoring task — new prose, a
membership shard, path wiring, then regeneration — and it clears one blocker of
three on its own. Costed rather than guessed, so the next attempt starts from
the shape above instead of rediscovering the two validator refusals.

---

### The split, costed properly (2026-09-23) — it needs a placement decision, not a mechanical fix

The earlier note said the split was cheap because chapter 8 holds one lesson and
all three dependents name `dayachesi` directly. That part is right, and it fixes
the churn question: **put the new lesson BEFORE `dayachesi` and make it a
prerequisite, keep the atom id, and no dependent needs editing** — everything
requiring `dayachesi` reaches the atom transitively. Verified: all three
dependents require both `TE-PRAGMATICS-…-03` and `TE-SCRIPT-…-04`, so a later
lesson would have forced three edits; an earlier one forces none.

Sequence insertion is also clean — slots 344–348 are free, and script lessons
already interleave with word lessons (`TE-S164` at 331, `TE-S134` at 335,
`TE-S112` at 355 around `dayachesi` at 350).

**What is not settled is where the atom should live**, and that is a question
about Telugu's structure rather than about this lesson:

| option | cost | problem |
|---|---|---|
| `writing` lesson in chapter 8, on `TE-PATH-013` | cheapest; no dependent edits | that segment is `SPINE-POLITE-REQUEST-REPAIR`. Filing script content on a politeness node is the **same category error HL-C420 objects to** |
| script track (`TE-S…`, chapter 1, `TE-PATH-100`) | no dependent edits; established pattern; extension is `stage: pre-A1` | the extension's canDo is *"pick out each of these Telugu characters inside the words I already say"* — **character recognition**. This atom is a RULE about vowel signs, and every sibling atom is `TE-SCRIPT-RECOG-<n>` for one character |
| move `TE-PRAGMATICS-…-03` instead, to a new chapter-8 `grammar` lesson | right node, real content (the respectful `‑ండి` ending deserves its own lesson), `grammar` is exempt from `CONTENT_TYPES` so no headword | the `‑ండి` point must come **after** `dayachesi` pedagogically, so three dependents need their prerequisites widened |

The third is the best pedagogy and the only one with a real cost; the first two
are cheap and each files knowledge under a heading that does not describe it.

**Not chosen here.** Telugu has no home for a script *generalisation* as opposed
to a script *character*, and inventing one — or deciding this rule is really
character-recognition after all — is a curriculum judgement about the track. It
is the same class as HL-C420's "what does the shared spine mean by handle
travel" and HL-C418's "does A2 give reasons", and it should be argued on the
structure, not on one atom-budget point.
