## Unreleased — the track can say no, join two things, and ask for a repeat

Nineteen lessons in four chapters (27-30), fifteen of them items, close
**seventeen A1 exam points**. The negation column goes from 1/5 to **5/5**, the
first column in this track to close completely, and the joining column from 1/11
to 8/11.

The functional hole was named in one line by the inventory: "আমি বুঝি is taught,
in chapter 14; না is taught, in chapter 1; the two are never put together
anywhere in 139 lessons." Nothing in the book could deny a sentence, and nothing
could join two nouns, so a learner who owned four foods, four garments and five
colours could not say *tea and milk* and could not say *I do not know*.

### Every number re-measured against the merged tree, not derived from a delta

    bengali A1 exam coverage                 104/244 (43%) -> 121/244 (50%)
    bengali lessons                                   139 -> 158
    bengali atoms taught                              154 -> 169
    bengali atoms never revisited                       5 ->   4
    bengali forward references                          4 ->   4   (see below)
    bengali forward prerequisites / reviews           0/0 -> 0/0   (held)
    bengali script closure violations                  21 ->  21   (held)
    bengali never-taught glyphs                        11 ->  11   (held)
    bengali headwords without romanization              0 ->   0   (held)
    bengali reinforcement window misses                191 -> 208
      R1 (1-3)                                          38 ->  38
      R2 (5-15)                                         51 ->  47
      R3 (20-60)                                        88 ->  85
      R4 (80-250)                                       14 ->  38
    lessons at or over the computed 300s ceiling         0 ->   0

### Seventeen points from fifteen items, and the ratio is one word

না was taught in chapter one as the answer "no" and never used again. Three of
the seventeen points ride on it: it negates a verb (NEG-02, SEN-02), it is the
second half of কেননা, "because" (JOIN-08), and doubled and moved to the front it
is *neither … nor* (NEG-05). Ranking clusters by points-per-item before designing
anything is what put negation before vocabulary here.

### What is taught, one item per lesson

    ch27  না, নয়, নেই, the rising question   three negators for three kinds of
                                              sentence, and every frame in the
                                              book becomes a question
    ch28  আর, -ও, বা, না … না                 joining, marking, choosing, and
                                              refusing both
    ch29  কিন্তু, একমত, কেননা                 objecting, agreeing, explaining
    ch30  যে, যখন, আবার বলবেন, ধীরে           reporting a thought, placing it in
                                              time, and rescuing a conversation

Two of the fifteen introduce **no new vocabulary at all**. আবার বলবেন is আবার
from chapter seven, বলা from chapter eleven and the polite -বেন from chapter
twenty-two: the repair sentence was missing because nobody had assembled it, not
because it was hard.

### Two spine omission ledgers go empty

BN-C16-na-verb realizes VERB-NEGATE and BN-C16-polar realizes QUESTION-POLAR, so
SPINE-NEGATE-AND-ASK omits nothing; BN-C18-kenona realizes CONNECTIVE-BECAUSE, so
SPINE-SAY-WHY omits nothing.

### The script debt did not move, and that was designed rather than lucky

Every candidate word was tested against the union of taught glyphs before it was
written. আর, বা, কিন্তু, কেননা, যে, যখন, একমত and ধীরে need no sign the track has
not taught. Three words that would have covered the same points do:

- **কারণ** ("because") needs ণ, so **কেননা** carries JOIN-08 instead and কারণ is
  named in romanization. BN-A1-ADV-08 stays null for that letter alone.
- **এবং** ("and") needs ং, so আর carries JOIN-01.
- **বোঝা** ("to understand") needs ঝ, so *āmi bujhi nā* — the sentence the
  inventory named as the one a beginner needs most — is printed in romanization
  and the Bengali sentences are আমি জানি না and আমি ভাত খাই না.

Each of those names its own fix, and the fix is a script lesson rather than a
vocabulary lesson. Violations held at 21 and never-taught glyphs at 11.

### The forward references this tranche created, and the fix

Teaching বা as a headword retroactively made four earlier lessons forward
references: BN-C01-dhonnobad, BN-C05-ami-bangla-boli, BN-C08-bhaba and
BN-C11-poribar each spell a syllable **বা** in bold inside ধন্যবাদ, বাংলা, ভাবা
and পরিবার. forwardReferences went 4 -> 8 for that reason alone and is back at 4:
the Bengali is still on the page, the emphasis is not, and the detector reads
only emphasised runs.

### The reinforcement number, decomposed

    created by this tranche      0   (zero new atoms miss any window)
    exposed by its added length 24   (R4 14 -> 38, all pre-existing atoms)
    pre-existing misses cleared  7   (R2 -4, R3 -3)

Nineteen more lessons made R4 judgeable for 24 older atoms that had never been
80 lessons from the end of the track before. Every chapter opener retrieves the
two preceding items by name and carries a bulleted recall line for an item two
chapters back, which is the R2 reach — and no new atom misses R1, R2, R3 or R4.

### Left uncovered, with the reason written into the inventory

    BN-A1-JOIN-05    একটা … আরেকটা needs the classifier AND ট
    BN-A1-JOIN-07    যে … সে needs the third person of BN-A1-PRON-03
    BN-A1-JOIN-10    the purpose suffix; the verbal noun is already taught
    BN-A1-ADV-08     কেন and কারণ as items; কারণ cannot be printed at all
    BN-A1-QUE-05     কখন is glossed inside যখন's table, not taught as an item

### Verified

`npx vitest run` (133 files, 1910 passed), every `check:*` gate, and the Bengali
book compiled with XeLaTeX: exit 0, zero missing characters, four new chapters
read on the page as images rather than through `pdftotext`, which mangles
Bengali clusters.
