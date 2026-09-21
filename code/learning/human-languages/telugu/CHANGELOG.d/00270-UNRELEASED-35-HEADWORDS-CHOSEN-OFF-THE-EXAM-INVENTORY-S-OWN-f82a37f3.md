## Unreleased — 35 headwords chosen off the exam inventory's own uncovered list

Chapters 74-80 add **35 Telugu headwords, seven chapters of five, one new word
per lesson**. They were not chosen by topic. Every one was picked off the
uncovered list in `core/exam-inventory-telugu-a1.json`, largest cluster first,
because an uncovered exam point names the missing word and a vocabulary count
does not.

**Telugu A1 exam coverage: 169/326 (52%) to 213/326 (65%).** 35 words closed
**44 points**, against an unchanged denominator — no point was added, removed or
reworded to move the number, and `partial` stayed at 0, so every probe still
names atoms that exist.

Three categories are now complete: **Prashnalu 9/9**, **Kriya visheshanamu 8/8**,
**Nishedhamu 3/3**.

### Every number re-measured against the merged tree, not derived

    telugu A1 exam coverage        169/326 (52%) -> 213/326 (65%)
    corpus exam-point backlog      686 -> 642            (-44, exactly this tranche)
    telugu pre-A1 headwords        186/300 -> 221/300    (shortfall 114 -> 79)
    telugu lessons                     301 -> 336
    telugu atoms taught                404 -> 449
    atoms never revisited               16 -> 13         (improved)
    reinforcement window misses       1017 -> 1014       (improved)
      of which R4 (recognition at distance) 320 -> 281
    forward references                  10 -> 10         (held)
    never-taught glyphs                  0 -> 0          (held)
    script-closure violations            9 -> 9          (held)
    distinct Telugu characters used     64 -> 64         (held)

### The seven chapters, and the node that makes them count

`level-gate.ts` reads a lesson's level from the **spine node** that claims it, so
good words hung off an A1 node move the pre-A1 count by zero. All seven pre-A1
nodes are in rotation, one per chapter, and the measured +35 confirms it.

    74 And, But, Or               SPINE-RESPOND-BASIC          mariyu kaanii leedaa enduku endukantee
    75 Today, Yesterday, When     SPINE-TAKE-LEAVE             ivaala ninna eppudu-vastaaru mundu samvatsaram
    76 Very Good, Very Bad        SPINE-CHECK-WELLBEING        caalaa-manci cedda mancidi sulabham kashtam
    77 Where You Are From         SPINE-EXCHANGE-NAMES         nunci pattanam deesam bhaaratadeesam muppai
    78 Yes or No, and I Do Not Know SPINE-POLITE-REQUEST-REPAIR avunaa telusaa naaku-teliyadu artham-kaaleedu unnaaraa
    79 He, She, and Whose         SPINE-MEET-GREET             sneehitudi maa atanu nannu gaaru
    80 Wanting, Allowing, Wishing SPINE-COURTESY-THANK         naaku-kaavaali ani vellavaccu lopaliki-randi subhaakaankshalu

### Why 35 words closed 44 points

Because the list was read before the chapters were designed. The clusters that
paid, in order:

* **Joining two ideas (8 points from 5 words).** The track had no word for *and*,
  none for *or*, none for *but*, and no way to answer *why*. That one chapter
  closed coordination, the causal clause, disagreement, the counter-argument and
  the two-alternative question.
* **He, she, whose (8 from 5).** Four of Telugu's six persons were never taught.
  `atanu`/`aame`/`vaallu` plus `maa`/`mana` closed the paradigm, the third-person
  verb endings, the object pronoun, the accusative `-ni`, the genitive on a full
  noun, the stem change in front of an ending, and `-gaaru`.
* **Yes or no (6 from 5).** Every question the track taught was a wh-question.
  The `-aa` particle closed the yes-or-no type, and with it *do you know*, *I
  don't know*, *I didn't understand*, *is he there*, and the negative
  conjugation.
* **Where you are from (5 from 5).** `nunci`, a country with a name, and the tens
  past twenty finally made the opening question of an A1 speaking paper
  answerable: name, age and origin, which had one of its four points covered.

### Eight points moved and were NOT claimed

Half a pair buys nothing, which is the rule the inventory started with. Each of
these keeps `probe: null` and had its note rewritten to say what landed and what
did not: `TE-A1-N-03` (a country has a name now; no town or state does),
`TE-A1-X-102` (two of four settlement words), `TE-A1-POS-02` (`maa` and `mana`
landed; `vaari` did not), `TE-A1-P-07`, `TE-A1-CO-13`, `TE-A1-X-09`, and the two
below.

**`TE-A1-DEM-03` was left uncovered for a script reason, and it is the one worth
naming.** The adnominal pair is `ii` and `aa`; `ii` needs the independent vowel
letter EE (U+0C08), which occurs **zero** times anywhere in the track. Teaching
it would put a glyph on the page the corpus has never shown and raise
`script-closure` off zero. Every one of the 35 words is spelled from characters
already in use, so the point stays open and `TE-A1-F-44`, which depends on it,
stays open with it. Closing them needs a script lesson, not a vocabulary lesson.

### The append that would have added debt, and what it cost to avoid

Written the obvious way — each lesson requiring the one before it — the tranche
measured **1143** reinforcement misses against a prior 1017. Decomposed: 66 on
the 45 new atoms, and **60 on OLD atoms whose R3/R4 windows a longer track newly
judged**. The second half was pre-existing debt becoming visible, not a
regression.

Three changes brought it to 1014:

* **Grammar atoms joined the retrieval chain.** Left out of it, all ten were
  introduced once and never retrieved — the entire R1 miss list on the first
  pass.
* **Each lesson retrieves at the R2 and R3 intervals**, not only the two
  immediately before it: the word from eight lessons back and from twenty-five.
* **Each lesson retrieves one OLD word** whose R4 window this position can fill,
  and *names that word's lesson as a prerequisite* — which is what makes the
  retrieval both legal and true. Telugu's prerequisite chain reaches only 148 of
  its 336 lessons, so an atom taught in chapter 8 is not otherwise available in
  chapter 74, and the validator was right to refuse it.

Every declaration is backed by a `[YOU RECALL: ...]` line that actually asks for
the words, so the front matter is not bookkeeping.

### Three ratchets that had to be held rather than reseated

* **Forward references (cap 12).** Making `vaccu`, `randi`, `teliyadu`,
  `kaavaali`, `eppudu` and `caalaa` headwords turned every earlier passing use of
  them into a forward reference, taking the track to 13. The fix was fuller
  headwords that are real utterances — `vellavaccu`, `lopaliki randi`, `naaku
  teliyadu`, `naaku kaavaali`, `eppudu vastaaru?`, `caalaa manci` — which is how
  this track already heads lessons. Back to 10.
* **Cross-chapter prose references (HL-C102, telugu capped at 46).** The first
  draft named a chapter number 29 times. All 29 now name the thing instead —
  "you already have", "your first lesson" — as that gate's own comment asks.
* **Rule statements (HL09, corpus ceiling 30).** Two Grammar Lenses tripped it
  with *always* and *the rule is ... every ending*. Both are now observations.
  Fixing the second also removed a plain error: it claimed `abbaayi` and
  `pustakam` become themselves.

