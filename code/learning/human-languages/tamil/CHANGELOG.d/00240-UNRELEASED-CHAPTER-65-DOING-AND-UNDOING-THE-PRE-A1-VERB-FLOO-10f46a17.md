## Unreleased — Chapter 65: doing, and undoing (the pre-A1 verb floor)

Seven lessons appended after chapter 64 and chained from `TA-C64-harvest`, on
`SPINE-MEET-GREET`, opening round four of the pre-A1 nodes.

**What it closes.** Tamil was one verb short of HL09 §3.1's pre-A1
verb-vocabulary floor — four distinct verb headwords taught at or below pre-A1,
against five — while sitting on twenty-three verb lessons, because every one of
the other nineteen realises an A1 or A2 spine node. The criterion could not be
closed by teaching more verbs; it could only be closed by teaching verbs *at
pre-A1*. Five arrive here and the blocker is gone:

```
verb-vocabulary   4/5  ->  9/5   criterion CLEARED
vocabulary      155/300 -> 160/300  (shortfall 145 -> 140)
```

**The five, each beside the act that undoes it.** The chapter is built out of
pairs, because a verb met beside its opposite is two words for the price of the
attention one costs:

```
குடி       kuḍi      drink   — completes the pair சாப்பிடு had been holding alone
நட        naḍa      walk
நிறுத்து   niṟuttu   stop    — what undoes நட
திற        tiṟa      open    — the verb that belongs to கதவு
மூடு       mūḍu      close   — what undoes திற
```

Each is tagged `TA-VERB-*` rather than the canonical `VERB-DRINK` / `VERB-WALK`
/ `VERB-OPEN` / `VERB-CLOSE`, and that is deliberate rather than lazy: those
canonical concepts are owned by `SPINE-NAME-EVERYDAY-ACTIONS`, which is **A1**.
Claiming them would have moved these five lessons off the pre-A1 node and left
the pre-A1 floor exactly where it was. The namespaced tag still satisfies the
gate's `(^|-)VERB-` test, so the count moves and the level claim stays honest.

**The writing strand comes back for a word it already taught you to say.**
`TA-W21-read-kudi` sits third, after குடி was learned by ear and after நட came
between — the gloss-first, then glyph-by-glyph, interleaved shape. It is the
first lesson in the strand that introduces **no new letter at all**: க, ட, ு and
ி are all already the reader's, and the lesson's whole content is putting four
known pieces together. `TA-C65-stop` then opens by reading the word back, so the
reading is retrieved inside its R1 window rather than left to decay.

**Two content-safety decisions, recorded in the lesson source so a later editor
does not helpfully undo them** (HL24 §6):

- **குடி** is never drilled bare. Without an object the word carries the
  drinking-*habit* reading, so every prompt names the liquid — *taṇṇīr kuḍi*,
  *tēnīr kuḍi*, *pāl kuḍi* — and the lesson says plainly why. No person-noun is
  derived from it.
- **மூடு** is never inflected toward a person-noun. One form looks as though it
  would be that, and is not: it is an unrelated Sanskrit borrowing, and an
  insult. An ordinary-looking completion of the pattern would put it in a
  beginner's mouth.

Both notes **describe** their hazardous forms rather than quoting them, which is
a departure from the Spanish precedent in `ES-C403-pollo` and was made on a
finding from this branch's security review: the narration exporter passes any
non-directive HTML comment straight into `speech.text`, so a safety note that
spells out the form it is avoiding gets read aloud to the learner by the very
pipeline it was protecting them from. Three Spanish narration chapters (7, 403,
404) already carry the leak; this is corpus-wide exporter debt and wants a fix in
`parse.ts`, which is out of scope for a single-track content branch.

**One headword was swapped after the gates caught it.** The stopping lesson was
first written on நில், which is the shorter and more obvious word — until
`tests/corpus/tamil.test.ts` failed at eight forward references against a
ceiling of seven. `TA-C20-vaanilai` already glosses நில் in prose, as the verb
inside நிலை inside வானிலை, so teaching it here would have been teaching a word
the book had already spent. நிறுத்து is clean corpus-wide, is what a Tamil
speaker actually shouts, and keeps the payoff: the lesson cashes in வானிலை's
etymology by naming the standing-family the two share, without re-spending the
bare word.

**ஓடு was considered for "run" and rejected.** ஓ appears nowhere in the Tamil
corpus, so writing it would have taken `neverTaughtGlyphs` from 0 back to 1 —
reopening a closure the track had finished. The script inventory stays closed.

**No regressions.** Script-closure violations 29 → 29, never-taught glyphs
0 → 0, reinforcement shortfall 50 → 50 (the closing `TA-C65-doing-recall`
retrieves chapter 64's last two atoms so the longer track does not push them out
of their R2 window), forward references 8 → 7, cross-chapter prose references
unchanged. The book compiles with zero overfull, zero underfull and zero missing
characters.

