## The negation, the joining words and the pointing words -- 114/292 to 164/292

Thirty-five lessons in seven chapters (29-35), one new item per lesson, chosen by
**points-per-item** against the uncovered list in
`core/exam-inventory-italian-a1.json` rather than by topic. Italian was measured at
39% (114/292), among the lowest in the corpus, and this closes **fifty points from
thirty-five items** -- 164/292, 56%.

**Four columns close outright, two of them from flat zero.**

  * **I dimostrativi 0/3 -> 3/3.** `questo` and `quello` appeared nowhere in the
    93 lessons this file was measured against. It was a whole column, not a gap in
    one. The etymology pays for itself twice: *eccu(m) istum* and *eccu(m) illum*,
    where the `eccu-` is the same "behold" that is already inside `cosi` and the
    `ille` is the very word that became the article `il, la, lo` in Chapter 1. One
    Latin word, three Italian jobs, and the `qui / li` lesson names all three.

  * **I possessivi 0/3 -> 3/3.** `mio, tuo, suo`; `nostro, vostro`; and the rule
    Spanish does not have -- the Italian possessive KEEPS its article, with the
    singular-unmodified-kinship exception that takes it back. **`vostro` was
    already in the reader's mouth**: the very first lesson of the track traces
    `ciao` to the Venetian *s-ciao vostro*, glosses that *vostro* as "your", and
    never names it as a word one could use. This tranche spends it.

  * **L'avverbio 2/8 -> 8/8**, and **La frase semplice 5/7 -> 7/7**.

**Negation was one word, and it was the track's most consequential gap.** `non`
occurred in 2 of 93 files, both inside the frozen `non c'e male`, whose own lesson
said in as many words that its negation and its `c'e` belonged to later lessons.
Both halves are now paid, four lessons apart, and the second one quotes the
deferral. That single `non` lesson unlocked the negative declarative, disagreement
(`non sono d'accordo`, whose other half arrived one lesson earlier), `ne ... ne`,
`neanche`, `non capisco` and `non so`.

**The joining column goes 1 of 13 to 10 of 13.** Its one previously covered point
was covered sideways: the written-accent lesson had to gloss plain `e` as "and" in
order to contrast it with `e`. `ma`, `o`, `perche`, `quando`, `ne` and `che` were
all zero-occurrence words in this track.

### TWO POINTS WERE RE-PROBED RATHER THAN AUTHORED AGAINST

An uncovered note is a claim about the corpus on the day it was written, and the
corpus moves. Two of this file's notes were wrong, both in the same direction --
the thing was already taught, in another guise, and the note had not read the
lesson that taught it. Neither point cost a lesson, and both freed a slot.

  * **`IT-A1-PRON-02`** said the three plural subject pronouns "are never named".
    They are. The `essere` lesson prints *noi siamo / voi siete / loro sono* and
    then tells the reader to KEEP `loro` where *io sono* and *loro sono* would
    collide -- which is naming a pronoun and giving the condition for using it.
    `andare`, `parlare` and `capire` print the same set.

  * **`IT-A1-F2-13`** said neither `sapere` nor `conoscere` is taught. `conoscere`
    is: the `incontrare` lesson teaches it as *incontrare*'s partner -- the event
    against the acquaintance -- gives its etymology from *cognoscere*, and puts it
    in the past. Only `sapere` was missing, and the `sapere` lesson is where the
    two are finally set against each other.

The freed slots went to `signore / signora` (`IT-A1-F5-04`, plus the vocative
`IT-A1-SN-04` for no new word) and `oggi / ieri` (`IT-A1-ADV-02`, joined to the
`domani` the track had only ever used inside the farewell *a domani*).

### FIVE POINTS CLOSE WITH NO NEW WORD AT ALL

`Che buono!` (`che` four lessons old plus a `buono` that had lived inside
*buongiorno* since the first chapter and was never once predicated of anything);
the predicative frame `il caffe e buono`, whose halves had been on the page a
hundred lessons apart; `per` plus an infinitive, which gives the infinitive its
first job in a sentence since it became the track's citation form; the
demonstrative position rule; and the vocative.

### REINFORCEMENT, DECOMPOSED

Measured against the merged tree, not asserted:

```
reinforcementWindowMisses           279 -> 219
reinforcementMissesByWindow-R1       43 ->  43
reinforcementMissesByWindow-R2      124 -> 124
reinforcementMissesByWindow-R3      112 ->  52
reinforcementMissesByWindow-R4        0 ->   0
atomsNeverRevisited                   6 ->   2
atomsTaught                         246 -> 308
```

Adding thirty-five lessons makes **133** pre-existing atoms window-judged for the
first time (R1 1, R2 9, R3 41, R4 82) -- debt the added LENGTH exposes rather than
debt the tranche creates. **All 133 are paid**, and the tranche's own 62 atoms
create **zero** new debt in any window. Pre-existing R3 debt falls by 60 on top of
that.

The openers are arithmetic, not a search. Every lesson recalls the previous lesson
(R1, distance 1) and the lesson five back (R2, distance 5), which covers its own
atoms and the five existing lessons that the new length newly exposed. The R4 slot
walks a diagonal -- new lesson *k* recalls the lesson at position 27+*k*, always at
distance 80 -- which is the only assignment that reaches all 82 newly exposed
atoms, because lesson *k* can reach no position later than 27+*k*. Two R3 slots per
lesson cover the 41 newly exposed and 60 of the pre-existing.

R1 and R2 do not move, and cannot: their misses sit at positions 2-85, and a lesson
appended at position 107 or later cannot reach back inside a 15-lesson window.

### WHAT IT DELIBERATELY DOES NOT CLOSE

The specific-notion column is untouched at 12/78. It is 66 points of pure
vocabulary -- clothing, transport, the meals of the day, the workplace -- and it is
where the next tranche has to go; the ratio is why it was not this one. Punctuation
stays at 0 of 5 inside its column: not one mark is taught in 142 lessons, and every
mark on every page is used correctly. The infinitive-clause points and the
possibility point need `volere`, `potere` and `dovere`, which no lesson here
teaches. `IT-A1-F3-03` (`preferisco`) is a one-word fix and is named as one.

### THE INVENTORY IS UPDATED IN THE SAME COMMIT

Landing 62 atoms and leaving the probes at `null` would have kept `164/292` reading
as `114/292` with a completely green suite. Every one of the 50 probes names the
atom the tranche actually introduces, checked mechanically, and both halves of the
gate were falsified: a fabricated atom id fails "probes only atoms that EXIST" AND
the count, and nulling a probe fails the count.

Cross-chapter prose references name the THING rather than the number, per HL-C102,
so `italian` does not grow past its pinned 34. Five earlier lessons were edited in
the same commit to stop previewing words this tranche now owns: `mi chiamo`'s
pronunciation note said "the h in *chi*" where it meant *chiamo*, and three
lessons previewed `molto` inside *molto piacere* a hundred and thirty lessons
before the word had an owner.

