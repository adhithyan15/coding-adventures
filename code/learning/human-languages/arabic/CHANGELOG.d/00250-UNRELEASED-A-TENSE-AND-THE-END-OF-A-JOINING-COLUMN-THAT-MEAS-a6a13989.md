## Unreleased — a tense, and the end of a joining column that measured zero

Twenty-one lessons in five chapters (37-41), sixteen of them items, close
**seventeen A1 exam points**. The joining column goes from **0/11** to 6/11 —
it was the first column in this whole series to measure a flat zero — and
negation from 2/5 to **5/5**.

The lever was a **tense**, not vocabulary. The inventory said so in two lines:
لا أفهم is "one present-tense form away and that form does not exist anywhere",
and the present tense "needs a lesson of its own". One prefix released both.

### Every number re-measured against the merged tree, not derived from a delta

    arabic A1 exam coverage                  106/238 (45%) -> 123/238 (52%)
    arabic lessons                                    102 -> 123
    arabic atoms taught                               199 -> 215
    arabic atoms never revisited                      125 -> 109
    arabic forward references                          13 ->  13   (held)
    arabic forward prerequisites / reviews            0/0 -> 0/0   (held)
    arabic script closure violations                   57 ->  57   (held)
    arabic glyphs taught / shown / never taught  30/45/15 -> 30/45/15 (held)
    arabic headwords without romanization              37 ->  37   (held)
    arabic exposure-only lessons                        7 ->  13
    arabic reinforcement window misses                485 -> 545
      R1 (1-3)                                         137 -> 139
      R2 (5-15)                                        165 -> 168
      R3 (20-60)                                       146 -> 167
      R4 (80-250)                                       37 ->  71
    lessons at or over the computed 300s ceiling         0 ->   0

### The ratio comes from one prefix

AR-A1-V-05 is a single form that two other points were sitting behind, and one
of them — AR-A1-FUN-15 — is the whole repair gap this track had. Once the
present tense existed, the joining column came almost free: every one of its
function words is inside the fifteen-of-thirty letter set the ladder has taught.

    An-nafy (negation)                     2/5  -> 5/5
    Ar-rabt (joining and subordination)    0/11 -> 6/11
    Al-jumla (the sentence)                3/7  -> 5/7
    Az-zarf (adverbs)                      5/8  -> 6/8
    Al-fil (the verb)                      6/11 -> 7/11
    Al-wazaif (communicative functions)   33/54 -> 37/54

### What is taught, one item per lesson

    ch37  أكتب, لا أفهم                 the present tense, and the sentence it
                                        released
    ch38  و, أو, لكن, حسنًا             the commonest word in written Arabic,
                                        glossed once in chapter 1 and taught
                                        never
    ch39  ليس, ما, لا … ولا, هل         three negators for three kinds of
                                        sentence, and the particle that asks
    ch40  لماذا, لأن, أنّ, حينما        why, because, that, when — three of the
                                        four sharing a visible piece
    ch41  يا, كرّر من فضلك              calling somebody, and the repair kit

### Four spine ledgers move

VERB-PRESENT-HABITUAL leaves SPINE-SAY-WHAT-I-DO's omits; VERB-NEGATE and
QUESTION-POLAR empty SPINE-NEGATE-AND-ASK; CONNECTIVE-BECAUSE empties
SPINE-SAY-WHY; RESPONSE-OKAY leaves SPINE-RESPOND-BASIC's omits.

### The script debt did not move, and the exemption was used three times

Arabic is the messiest of the three tracks in this series on script — 57
violations, 15 never-taught glyphs, 37 headwords with no romanization — and none
of those four numbers changed. Every new lesson declares a romanization, and
every new body was written inside the taught letter set.

The headword exemption is used exactly three times, for the three headwords that
could not be: **لا أفهم** (فاء), **لماذا؟** (ذال and the question mark) and
**كرّر من فضلك** (فاء and ضاد). `exposureOnly` rose 7 -> 13, which is that choice
made visible rather than hidden. Every other appearance of those words is in
romanization.

Words rejected on the same test and named in the inventory beside the letter
that blocks them: عندما (دال), الذي (ذال), إلا (alif-hamza-below), أعرف (فاء).

### The reinforcement number, decomposed

    created by this tranche      0   after a fix; it was 3 first
    exposed by its added length 60   all pre-existing atoms
    atoms never revisited      125 -> 109, a fall of sixteen

Three new atoms — لا … ولا, أنّ and لماذا — missed R2 on the first draft, because
each was introduced late enough that nothing five to fifteen lessons later
retrieved it. Two recall lines in the chapter-41 review fixed all three, and the
decomposition was re-measured rather than assumed.

The two distant bands are what moved never-revisited by sixteen. AR-R38 runs و,
أو and لا … ولا over the pantry of chapters 33-36, and AR-R41 replays the
greeting, name and wellbeing exchanges of chapters 1-4 — untouched for more than
a hundred lessons — and then breaks them on purpose so the learner uses عفوا,
لا أفهم and كرّر من فضلك to get the conversation back.

### Left uncovered, with the reason written into the inventory

    AR-A1-V-04     the past-tense person suffixes; now the largest gap in the
                   file on its own, and a single closed table
    AR-A1-V-07     كرّر is the corpus's first imperative but is one fixed
                   exponent, not a form derived from a rule
    AR-A1-SEN-02   verb-subject-object order; buildable now, and still unsaid
    AR-A1-JOIN-04  the distributive, blocked on the numerals
    AR-A1-JOIN-06  الذي needs ذال
    AR-A1-JOIN-09  the purpose لـ needs a subjunctive nothing teaches
    AR-A1-JOIN-10  the maṣdar as a construction, shown repeatedly and unnamed
    AR-A1-JOIN-11  إلا needs alif-hamza-below

### Verified

`npx vitest run` (134 files, 1921 passed), every `check:*` gate, and the Arabic
book compiled with XeLaTeX: exit 0, zero missing characters, five new chapters
read on the page as images rather than through `pdftotext`, which mangles
Arabic clusters.

