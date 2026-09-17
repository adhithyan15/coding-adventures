## HL-C393 — Malayalam names the future tense and never prints it

Found while surveying `ML-A1-Q-06`, `ML-A1-JOIN-07` and `ML-A1-JOIN-08`.
Recorded here because **the exam point is genuinely covered and the reader
still cannot read the form** — a shape no metric on this track can see, and one
worth having written down the next time coverage looks complete. The repair
itself belongs to the *when*-clause unit rather than to a later cleanup, for
the reason given at the bottom.

### What the corpus has

`ML-C32-pokuka` is the track's tense lesson, and it is a good one. It teaches
that Malayalam holds the stem still and swaps one piece, that the verb never
agrees with its subject, and that each of the three forms is therefore the
*whole* of its tense rather than the first row of a paradigm. Its table:

| when | Malayalam | the ending |
|---|---|---|
| now | *pōkunnu* | *-unnu* |
| already | *pōyi* | *-i* |
| not yet | *pōkuṁ* | *-uṁ* |

`ML-A1-V-05` — "the future tense" — probes `ML-CONCEPT-C32-POKUKA-02`, and that
concept is exactly what the Grammar Lens teaches. The point is not mis-marked.

### What it does not have

**That table is romanization only.** Extracting every Malayalam-script token
from the lesson body returns പോകുക, പോ, പോക്, ഉക and പോയി. The middle column
above is Latin from end to end, so പോകുന്നു and പോകും are named in the lesson
and written nowhere in it.

Across all 333 lesson files:

- **പോകും — zero.** **വരും — zero.** **പോകുന്നു / വരുന്നു — zero** before
  chapter 74.
- Every script word in the corpus that ends in **-ും** is something else: the
  coordinator of chapter 70 (വെള്ളവും, അരിയും, പാലും, ചായയും, കാപ്പിയും), or a
  frozen adverb (വീണ്ടും, തീർച്ചയായും, പോരും).

So a reader who has finished chapter 32 has been told the future ending, can
say the future aloud from the romanization, and **has never once seen a Malayalam
verb in the future tense written down.** On a track whose premise is reading the
script, that is the gap.

### Why it is easy to miss

This is the recurring shape, one turn further along than usual: a rule gets
taught, the rule is correct, and half of what it predicts is never handed over.
Here the missing half is not a second word — it is the *script side* of a form
the lesson already covers in speech. Coverage metrics cannot see it, because
the atom is introduced and the point is probed. Only reading the lesson body
for script tokens shows it.

`tests/glyph-coverage.test.ts` cannot see it either: every glyph in പോകും is
taught elsewhere. It is not a glyph debt. It is a **word** the reader can say
and cannot read.

### What the fix looks like

Print the column. `ML-C32-pokuka` should carry പോകുന്നു / പോയി / പോകും in the
script beside the romanization, the way every other word lesson on this track
presents a headword. The change is small and its blast radius is a duration
pin, since the lesson already sits at 275 seconds against the 240 ceiling's
raised allowance.

Do it **as the first step of** the unit that builds a subordinate clause on
the future stem, not after it.
`ML-A1-JOIN-07` needs വരുമ്പോൾ, which visibly contains വരും — and a reader
meeting വരും for the first time *inside* a longer subordinate form has been
handed the hard version first. That is the ramp inverted.

Chapter 74 already prints കഴിയും and പോകുന്നു, which makes it the first place
in the track where the endings of `ML-C32-pokuka`'s table appear in the script
at all. That is an accident of where ability landed, not a plan, and it does not
repair chapter 32 for a reader moving through in order.
