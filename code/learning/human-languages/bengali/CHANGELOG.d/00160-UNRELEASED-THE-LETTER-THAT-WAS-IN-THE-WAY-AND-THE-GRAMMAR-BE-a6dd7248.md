## Unreleased — the letter that was in the way, and the grammar behind it

Thirty-five lessons in seven chapters (31-37), twenty-eight of them items, close
**seventeen A1 exam points**. Three columns close outright — definiteness 1/3 to
**3/3**, deixis 0/3 to **3/3**, possession 2/3 to **3/3** — and pronouns reach
7/8.

The ratio comes from a **letter**. ট was shown in the corpus and taught nowhere,
and six points sat behind it: a learner could not say *the* shirt, *a* shirt,
*another* shirt, *this* shirt or *what is this?*, because none of those can be
written in Bengali without it. Teaching one consonant unblocked a whole grammar.

### Every number re-measured against the merged tree, not derived from a delta

    bengali A1 exam coverage                 121/244 (50%) -> 138/244 (57%)
    bengali lessons                                   158 -> 193
    bengali atoms taught                              169 -> 197
    bengali atoms never revisited                       4 ->   3
    bengali forward references                          4 ->   8   (see below)
    bengali forward prerequisites / reviews           0/0 -> 0/0   (held)
    bengali script closure violations                  21 ->  21   (held)
    bengali never-taught glyphs                        11 ->   9   (ট and থ)
    bengali headwords without romanization              0 ->   0   (held)
    bengali reinforcement window misses               208 -> 249
      R1 (1-3)                                          38 ->  38
      R2 (5-15)                                         47 ->  49
      R3 (20-60)                                        85 ->  90
      R4 (80-250)                                       38 ->  72
    lessons at or over the computed 300s ceiling         0 ->   0
    four-column tables the narrator must refuse         44 ->  44   (held)
    corpus rule statements                              31 ->  31   (held)

### Seventeen points from twenty-eight items, and the ratio is one letter

ট is taught first, as the fourth corner of a square whose other three the book
already owned: ত dental voiceless, দ dental voiced, ড retroflex voiced. The
classifier টা follows it, and then একটা, আরেকটা, এটা and the stacking rule all
follow the classifier. Six points, one consonant.

The animacy split does the same work a second time: one question about a noun —
is it alive? — chooses the plural ending in chapter 33 and the object ending in
chapter 36, so N-06, N-05 and KAR-01 are three points on one rule.

### What is taught, one item per lesson

    ch31  ট, টা, একটা, আরেকটা          the letter, the ending Bengali has
                                       instead of an article, and what it
                                       unlocks
    ch32  এই, ওই, এই …টা, এটা          two distances, both ends of the phrase
                                       at once, and *what is this?*
    ch33  জীব/জিনিস, রা, গুলো, লাল জামা  one question, two plurals, and where an
                                       adjective stands
    ch34  সে, তিনি, ও, তার             the third person, graded for respect and
                                       sorted by distance
    ch35  আমরা, তোমরা, তারা, আমাদের     six persons on one ending, and the rule
                                       that the back slot holds ONE suffix
    ch36  কে, আমাকে, তোমাকে, কে?        the object case, and chapter nine's
                                       dative frame finally filled
    ch37  থ, এখানে, ওখানে, কোথায়        the ninth cell of a three-by-three grid

### Two spine omission ledgers go empty

BN-C20-tta-classifier realizes GRAMMAR-THE, so SPINE-DEFINITE-REFERENCE omits
nothing; BN-C26-kothay realizes QUESTION-WHERE, so SPINE-ASK-LOCATION omits
nothing. Both are declared through `conceptAliases` in the track's
`curriculum.d/_meta.json`, because the classifier is not an article and the
lesson keeps its own name for it.

### The script debt did not move, and two glyphs were bought

Every candidate was tested against the union of taught glyphs before it was
written, and the worked examples were tested too — প্রাণী was the first draft's
word for "living thing" and it hides a ণ the track has never taught, so the
lesson says জীব instead.

ট and থ are **consonants**, and HL-C212 established that Wikimedia Commons holds
a stroke-order animation for every Bengali independent vowel and for **no
consonant at all**. So neither lesson claims a pen path. Both are written the
way this track's other twenty-six consonant lessons are written — place of
articulation, the square the letter completes, and a Unicode Bengali chart
citation. A guessed ductus is not an option, and none is guessed here.

Still blocked on a letter, and named in the inventory beside the point:

    কারণ   "because"        needs ণ  -> ADV-08 stays null
    অনেক   "much, many"     needs অ  -> Q-05 stays null
    মানুষ  "person"         needs ষ  -> not needed by any open point

### The reinforcement number, decomposed

    created by this tranche      0   no atom introduced by any of the 35 new
                                     lessons misses a window it was long enough
                                     to have
    exposed by its added length 41   all on PRE-EXISTING atoms, and 34 of the 41
                                     are R4 alone: thirty-five more lessons made
                                     R4 judgeable for older atoms that had never
                                     been 80 lessons from the end of the track
    atoms never revisited        4 -> 3

Every chapter opener retrieves the two preceding items by name and carries a
bulleted recall line for an item two chapters back, which is the R2 reach; every
chapter review carries a distant band that reaches four and five chapters back,
which is the R3 reach.

### Forward references: one false positive fixed, four made attributable

    4 -> 9 -> 8

BN-C05-kaj-kora takes করা apart as ক + রা and emphasised the second syllable,
which the detector read as the plural suffix this tranche now teaches. The
Bengali is still on the page; the emphasis is not. That is the same false
positive and the same remedy as the বা case in chapters 27-30.

The remaining four are **not** false positives and are **not** new debt. They are
places where the corpus already put a word in front of the reader and taught it
nowhere at all; teaching it — late — is what made the earlier use measurable:

    BN-C07-jaowa        তিনি      101 lessons early, taught at ch34
    BN-C09-sahajjo-kora আমাকে     100 lessons early, taught at ch36
    BN-C13-sbagotom     আমাদের     68 lessons early, taught at ch35
    BN-C15-jama         লাল জামা   44 lessons early, taught at ch33

An untaught preview counts as nothing; a taught one counts as a forward
reference. The number rising is the debt becoming attributable, not appearing.

### Left uncovered, with the reason written into the inventory

    BN-A1-JOIN-07   যে … সে — the correlative itself, now that the pronoun exists
    BN-A1-JOIN-10   the purpose suffix; the verbal noun is already taught
    BN-A1-ADV-08    কেন and কারণ as items; কারণ cannot be printed at all (ণ)
    BN-A1-QUE-05    কখন is glossed inside যখন's table and in ch36's, not taught
    BN-A1-PRON-08   the reflexive নিজে
    BN-A1-Q-01/03   the numerals stop at five, and no Bengali digit is taught

