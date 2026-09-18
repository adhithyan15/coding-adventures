### Fixed — `HL-C402`: the genitive table was never accurate, and my first fix said it was

- No coverage change. `ML-A1-LEX-06` stays closed at 208/243 (86%).

#### Two attempts, and the first one failed the way the shard did

`HL-C402` recorded that `ML-C78-ude` (2700), `ML-R78-genitive-recall` (2710) and
`ML-C84-mukalil` (2920) all teach *"a noun that ends in a vowel takes
**-യുടെ**"*, with a two-row table. It proposed a third row — **പശു** →
**പശുവിന്റെ** — and flagged that row as *the one claim resting on morphology
rather than on a grep*. That kept it open across three chapters.

**The first attempt enumerated every genitive in the corpus and concluded the
table was accurate for everything the learner had seen by 2700**, with the
falsification arriving later at 3000 and 3210. On that basis it said the table
was merely *not exhaustive* and promised a third landing later.

#### That enumeration was keyed on Malayalam script, and the corpus does not keep all its genitives in script

| form | stem | stem ends in | ending | first at |
|---|---|---|---|---|
| **എന്റെ** | ഞാൻ (suppletive) | — | **-ന്റെ** | **80** |
| **നിന്റെ** | **നീ** | **a vowel** | **-ന്റെ** | **130** |
| *niṅṅaḷuṭe* | **നിങ്ങൾ** | **chillu — a consonant** | **-ുടെ**, no glide | **130** |

***niṅṅaḷuṭe* appears in the corpus only in romanization**, so a script-keyed
grep returns zero for it. That is
`lessons.d/a-census-keyed-on-one-of-a-construct-s-two-spellings-undercounts.md`,
repeated verbatim: *"a census keyed on one of a construct's two spellings
undercounts silently, and the number gets published."* The repo had the lesson
on file and I made the mistake anyway.

**So the table was never accurate.** **നിന്റെ** breaks the vowel row and
*niṅṅaḷuṭe* breaks the consonant row, **both at sequence 130** — 2,570 points
before `ML-C78-ude`, in a lesson `ML-R78` itself points the learner back to.

#### The second attempt: stop claiming the ending is predictable at all

- `ML-C78-ude` — the table rows carry no *"because"*, and the text says it
  plainly: *"Which of the two a noun takes is learned with the noun, not worked
  out from its last sound — and you have had the proof since your first chapter.
  **നിന്റെ** is the owner-form of **നീ**. **നീ** ends in a vowel, exactly as
  **അമ്മ** does, and it does not take **-യുടെ** at all."* What survives is the
  part that is reliable: the shape of the phrase.
- `ML-R78-genitive-recall` — its Grammar Lens header **"because it ends in"** is
  gone, and its Wrap-up now asks *"Can you tell which one a new noun will take
  by listening to its end? (**No** — **നീ** ends like **അമ്മ** and takes
  **നിന്റെ**.)"*

  **The first attempt fixed this file's drill and left its table** — eighteen
  lines above the file's own **എന്റെ**/**നിന്റെ** paragraph. That is the
  `ML-C97-bhangi` failure mode, inside the very lesson this shard named as the
  one to fix first, and the chapter-101 failure mode — refuted by its own file —
  one PR later.
- `ML-C84-mukalil` — Warm-up now says *"the one **അമ്മ** took."*

#### The forward promise was dropped, not kept

The first attempt added *"this book will show you another before long."* **No
lesson ever presents a third genitive ending as a third ending**:
`ML-C86-oppam` (3000) folds **-ിന്റെ** into *"the ending consonants take"*, and
`ML-C91-vila-panam` (3210) explains only the **ം → ത്ത** oblique. A promise the
corpus does not honour is worse than no promise, so it is gone.

#### A duplicate shard was filed and withdrawn

The first attempt also filed `HL-C403` against `ML-C83-keralathil`'s *"every
noun ending in **-ം**"*. **`HL-C400` is open and already covers that lesson with
the same quote**, and covers it better: it names four behaviours rather than
three, credits `ML-C70-um` and `ML-C93-angal`, and already establishes that
**സാരമില്ല** has carried its **മ** since chapter 3 — so the universal was false
when written, not falsified later as `HL-C403` claimed. `HL-C403` is deleted;
`HL-C400` stands.

**പശുവിന്റെ is asserted nowhere**, and no unattested form is used anywhere in
the fix.
