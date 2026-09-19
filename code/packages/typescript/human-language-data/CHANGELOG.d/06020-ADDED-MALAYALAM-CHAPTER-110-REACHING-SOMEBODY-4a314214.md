### Added — Malayalam chapter 110, reaching somebody

- `ML-A1-LEX-38` closes. Malayalam A1 coverage 217/243 -> **218/243**, and the
  track **crosses 90 per cent** (218/243 is 89.7, which rounds up). 25 points
  unmapped.

#### Five Spanish points behind one Malayalam point

`A1-NE09-01`, `-02`, `-03`, `A1-NE09-06` and `A1-NE11-01` — information and
communication media, written correspondence, the telephone, the internet and
dictating an e-mail address, and the postal service. The largest single payoff
left in that inventory.

#### The note's "Nothing" held, but only a word-bounded census proves it

**കത്ത്** (*a letter*) looks taught: grepping for the string matches inside
**അകത്ത്** (*inside*, `ML-C95`). Bounded against the Malayalam block it returns
**zero**. Count the token, not the substring.

The lesson turns that near-miss into teaching — it shows the learner the overlap
so it cannot mislead them later, leaning on `ML-C95`'s own advice to *read the
ends and not the fronts*.

#### Four of the five words cost no script debt

**കത്ത്**, **തപാൽ** and **ഇമെയിൽ** use only glyphs already shown and directly
owned. **ഇന്റർനെറ്റ്** deepens the already-open chillu-*r* from 11/11 to
**12/12** and opens nothing new; `shown` stays **68** and the owned overlap
stays **59**.

#### The telephone is taught by ear, and the chapter says why

**ഫോൺ** needs the chillu **ൺ**, which `ML-S131` teaches at sequence 144 but
`data/scripts/malayalam.json` omits — so writing it trips `uncovered-glyphs`, and
`integration.test.ts` turns that queue into a hard failure.

`HL-C398` records why a curriculum chapter must **not** simply add the entry:
every listed chillu is pinned to a **sourced** stroke-order animation, the
inventory states outright that *"a citation cannot be guessed"*, and script
inventories are a different owner's surface by design (HL24). Fabricating the
citation and crossing that boundary were both refused.

So the word is given in **romanization**, with the lesson telling the learner
plainly that its written shape comes when its letter does — chapter 88's
precedent for *eighty*, and `ML-C07-numbers-6-10`'s before that.

#### A draft was thrown away to get there

The first version bought a new letter for the job: `ML-S148-letter-pha` taught
**ഫ**, the breath-partner of **പ**, which appears in **no headword anywhere** in
the corpus. Validate then failed on the *chillu*, not on **ഫ** — and once the
word could not be printed, a letter bought to write it had no purpose. That
lesson was **deleted** rather than kept for appearance. The chapter is five
lessons, not six.

`HL-C398` is updated with this second occurrence: one number and one everyday
noun now taught by ear, plus one script lesson written and discarded. Two words
in, the cost is demonstrated rather than predicted.

#### Where chapter 108 pays off

Saying an e-mail address aloud needs the **full stop**, taught with the other
marks. The lesson also says honestly that the mark before the domain still has
no Malayalam name here — the same call the punctuation chapter made about the
marks' own names.

#### Verification

`npm run validate` 21/21, all twelve gates, the full suite (145 files, **2097
passed**, 1 skipped), `check-book-compile.sh --strict malayalam`, all six LaTeX
warning counters at zero, and the append-only shard check empty.
