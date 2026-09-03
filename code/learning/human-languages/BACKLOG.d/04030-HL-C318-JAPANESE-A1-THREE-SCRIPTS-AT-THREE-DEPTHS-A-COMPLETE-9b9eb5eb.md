## HL-C318 — Japanese A1: three scripts at three depths, a complete repair kit, and the same empty joining column

`core/exam-inventory-japanese-a1.json` enumerates **179** A1 points and the corpus
covers **66**, 37%. Three findings here are worth more than the number, and one of
them is the most useful thing measured in this batch.

### "The script" is not one column, and pretending it is destroys the finding

    hiragana   31 of 46 basic signs taught
    katakana    2 of 46
    kanji       3 characters, plus 3 components

Average those and you get a number that describes nothing. Report them apart and
you get the actual state of the track: **a reader who can decode a hiragana
sentence still cannot read a menu, a name or a sign**, because every foreign word,
every loanword and every proper noun on a JLPT N5 reading paper is katakana, and
every content word is kanji.

So the inventory has three script categories, a fourth for what they share, and
the mixed-script fact as a point of its own (`JA-A1-HYO-01`) — because that is
what a single column would most obviously have lost. An ordinary Japanese
sentence runs all three at once, and a reader who has learned the three sets
separately still cannot read one until somebody tells them the division of
labour. Chapter 7's doorway dialogue does exactly that and is what the point
rests on.

**Generalisable rule: split a script column wherever the track's own depth
differs across the systems, and never average.** Japanese is the extreme case;
Urdu (Nastaliq plus Arabic numerals) and Serbian (if it is ever added) have the
same shape.

### Which proxy orthography points have no Japanese analogue at all

Three, dropped in two entries with separate reasons:

* **`A1-O1-04`, `A1-O1-05` — capitals and initial lower case.** Japanese is
  caseless in all three scripts. No kana and no kanji has a capital form, there
  is no sentence-initial marking and no proper-noun marking. **The Chinese
  inventory keeps these two points**, because pinyin has a case distinction and
  pinyin is taught in PRC schools and printed on beginner HSK material. It does
  NOT survive in romaji: romaji is not part of Japanese literacy in any
  comparable sense and appears on no JLPT paper at any level, so counting a
  romaji capital would be measuring a convention this project invented for its
  own transcriptions. The asymmetry between the two files is deliberate and is
  stated in both.
* **`A1-O1-06` — superscript letters in abbreviations.** No superscript
  convention in any script; the Japanese ordinal is the prefix `dai` plus a
  numeral, not a raised ending. Russian *derived* this same point (`RU-A1-L-09`,
  the hyphenated ordinal `1-y`), and saying so is what keeps "nothing to
  superscript" from reading as "superscripts looked foreign".

Everything else in the Spanish orthography column restates, including the two
accentuation points, which become the **dakuten** — a diacritic on an otherwise
complete sign, which is the demand the proxy was making.

### The exam anchor scores no production and no interaction, so the file says so

`exam-levels.json`'s japanese caveat is the most load-bearing of any track's:
the published CEFR indication on a JLPT score report covers "only the language
knowledge, reading, and listening competence JLPT tests", and "JLPT does not test
production (speaking and writing) or interaction."

An inventory that quietly reported reading-and-listening coverage would flatter
this track exactly the way HL20 §1 warns about. So `japanese/assessment-spec.md#a1`'s
four JF-Standard companion tasks are enumerated as points, one for one:

    complete a small form                                   ABSENT
    write a 30-40 character message for a named reader      ABSENT
    answer familiar personal questions                      ABSENT
    complete a simple rehearsed role-play                   COVERED
    (plus) interaction: taking an unrehearsed turn          COVERED

The second is the interesting one. The reader can write 31 hiragana, 2 katakana
and 3 kanji, which is **enough signs for a 30-character message** — what is
missing is a sentence to write, because there is no `desu`, no verb paradigm and
no particle. **A track that can write the letters and not the message** is a
precise description of where this corpus stands, and no percentage would surface
it.

### The complete CEFR A1 repair kit, and it is the only one in the corpus

Repair reads **7 of 8**. Chapters 9 and 10 teach every move:

    sumimasen                          apologise, and get attention
    wakarimasen                        report the failure
    mou ichido onegai shimasu          ask for a repeat
    mou sukoshi yukkuri itte kudasai   ask for slower speech
    koko                               point at the word you missed
    wakarimashita                      confirm the repair worked

and it teaches `mou`, `ichido`, `onegai shimasu`, `yukkuri`, `sukoshi` and
`itte kudasai` as separate atoms as well as as sentences, so the reader owns the
pieces rather than a formula. The last move is the one almost nobody teaches:
after a repair, the speaker needs to hear that it worked.

Measured against the rest of the batch:

    japanese   6 of 6 moves
    russian    2 of 6 (ya ne ponimayu, ya ne znayu) and NO word for sorry
    chinese    1 of 6 (shenme?) and NO word for sorry
    gujarati   0 of 6

**This is also where the corpus is ahead of its own exam.** JLPT scores no
interaction, and CEFR A1 interaction is largely the ability to keep an exchange
alive when it breaks. The track is strongest at exactly the thing its anchor
cannot see.

### And the joining column is still 0 of 8

`demo`, `kara`, `node`, the quotative `to`, `to` as a noun-joiner, `mo`,
`soretomo` and the te-form-as-link: none taught, and `demo`, `kara` and `node`
return zero occurrences of any kind, in kana and in romaji.

**Japanese is one of only two tracks in the whole corpus with zero findings in
every gentle-ramp queue** — no missed reinforcement window, no script-closure
debt, no forward reference, no atom or glyph spike, no measurement-blind lesson —
and its joining column is as empty as Gujarati's. Eight tracks in a row now, six
Indic, one Slavic, one Sinitic, one Japonic. **That is no longer a fact about a
language family or about neglect. It is a fact about how this corpus is
authored:** a chapter is built around a word family, and a conjunction belongs to
no family, so nothing ever schedules one.

### What Japanese does differently, measured rather than admired

Since it is the best-constructed track, it is worth saying what the construction
actually is:

    lessons                          117
    lessons introducing NOTHING       17  (14%, pure retrieval)
    declared reviews_of edges        392
      at distance 1-3   (R1)         123
      at distance 5-15  (R2)         128
      at distance 20-60 (R3)         110
      at distance 80-250 (R4)         31

Compare the same walk on two tracks measured beside it:

    chinese   50 zero-new of 175;  232 / 105 /  45 / 24   (front-loaded)
    russian    5 zero-new of  88;  120 /  44 /  28 /  2   (collapses)
    marwadi   45 zero-new of 257;  385 / 264 / 208 / 223  (the other flat one)

**The distribution is nearly flat across R1, R2 and R3.** Reviews are scheduled
at each expanding distance rather than taken wherever they happen to be
convenient, and one lesson in seven exists only to reach back. That is the
mechanism behind "zero missed windows", and it is copyable: it needs no new
content, only lessons whose `introduces` list is empty and whose `reviews_of`
list is chosen by distance.

### Two script-data findings, both cheap

* `data/scripts/japanese.d` carries 9 katakana and 6 logographs with **cited**
  stroke order. **Seven of the nine katakana and three of the six kanji are
  untaught** — and the three kanji are `kon` (inside `konnichiwa`) and `yuu` and
  `nan` (inside `arigatou`), both already explained in the corpus's own
  etymology. Evidence checked in, motivation already written, lessons missing.
  This is the cheapest script tranche available anywhere in the corpus.
* Three signs the writing lessons **do** teach — `ra`, `da`, `do` — are **not in
  the ductus at all**, while `ga`, `go` and `za` are in it as separate entries.
  So `ra`'s pen path rests on prose plus a Unicode code chart, which shows a
  shape and does not order strokes. HL11's "no citation, no pen path" is met in
  the letter and not in the substance. Three ledger entries with sources fixes it.

### What to author next, off this inventory

1. `desu` and `watashi` — closes four Spanish points and makes "I am X"
   sayable, which is the first sentence of any course and is currently
   impossible.
2. `wa` as the topic particle (the spelling is already taught) and `ka` — gives
   the reader a sentence template and a question at once.
3. The `kosoado` series — `kore`, `sore`, `are` — 8 of its 9 cells are missing
   and Spanish's `este`/`ese`/`aquel` maps onto it exactly.
4. `no` and `wo` — two hiragana that carry possession and the object.
5. The remaining fifteen hiragana, `wo` first.
6. The seven already-sourced katakana.
7. `-san`, which is two hiragana and is what makes a name usable.
