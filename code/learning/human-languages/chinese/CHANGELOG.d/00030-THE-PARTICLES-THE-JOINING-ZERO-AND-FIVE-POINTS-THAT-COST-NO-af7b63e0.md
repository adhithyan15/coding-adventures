## The particles, the joining zero, and five points that cost no character

Thirty-five lessons in seven chapters (22-28), chosen by **points-per-character**
against the uncovered list in `core/exam-inventory-chinese-a1.json` rather than by
topic. Chinese was measured at 38% (73/191), among the lowest in the corpus. This
closes **twenty-five points for TEN CHARACTERS** -- 98/191, 51%.

**A character is the expensive unit in this track**, and the inventory had already
priced it: a writing lesson, a reading lesson, a source-verified stroke record in
`data/scripts/chinese.json`, and a regeneration of the vendored subset font. So the
tranche was chosen the way that price implies -- by what each character buys.

### FIVE POINTS COST NO CHARACTER AT ALL

Chapter 22 adds **not one character** and closes five points.

  * **`ZH-A1-V-02`** -- a Mandarin verb has one form, for every person, every
    number and every time. The old note called this the cheapest sentence in the
    whole Mandarin queue and it was not written. It is now.
  * **`ZH-A1-NP-03`** and **`ZH-A1-VP-03`** -- nothing agrees, in either
    direction: not the verb with its subject, not what follows 是 with how many
    there are.
  * **`ZH-A1-ART-01`** -- 书 is *a* book, *the* book, and books. All four Spanish
    article points collapse onto that one fact, and no lesson had said it.
  * **`ZH-A1-ADJ-02`** -- 中国人. 中国 was taught in the country chapter and 人 was
    the first character this book ever wrote, and nothing had put them together.

Three of those five are things the reader must **stop** doing, which is why no
lesson had ever said them: an absence leaves no word to teach. `ZH-A1-S-04`, the
topic-comment order, is a sixth free point and rides in the 的 chapter because
that is where the first slot becomes worth filling.

### THE TEN CHARACTERS, AND WHAT EACH BOUGHT

| character | strokes | points |
|---|---|---|
| 的 | 8 | POSS-01, PRON-05 |
| 有 | 6 | V-04, NG1-01 |
| 个 | 3 | LIANG-01, NG2-01 |
| 了 | 2 | V-06, PART-03, ASP-01 |
| 呢 | 8 | PART-02, F2-01 |
| 对 | 5 | NG6-07, F6-02 |
| 岁 | 6 | NG5-04, F1-03 |
| 和 | 8 | J-01 |
| 喜 + 欢 | 12 + 6 | PRON-03, F3-01, NG6-04 |

**了 is two strokes** -- fewer than any character in the book but 一 and 二 -- and
before it every sentence in this track happened in an eternal present. **个 is
three strokes and two of them are 人.**

### THE PARTICLE GAP AND THE JOINING ZERO

The file's headline finding was that Mandarin carries almost all of its grammar in
a handful of toneless particles and **not one was taught**. That had already moved
once (吗, in the asking chapter). It is now **4 of 7**: 的, 了 and 呢 arrive here,
and the two that carry the most grammar between them cost eight strokes and two.
`ZH-A1-PART-04` stays open because a class point is not answered by four of seven,
and its note now names 吧, 过 and 着 as what is left.

**The joining column comes off zero** -- flat in seven tracks running. `ZH-A1-J-08`
had already done the work of saying why and which word was cheapest, and this
tranche spent exactly that budget: 和 is eight strokes, three of which are 口, and
it is a phono-semantic compound like 请 whose sounding half **is** *hé*. The lesson's
own content is the restriction: 和 joins two **nouns** and never two clauses.

### ONE NOTE HAD DECAYED, AND IT WAS THE NUMERALS

`ZH-A1-NG5-04` read "blocked on the numerals". The numeral tranche of chapters
20-21 had already lifted that block: only 岁 was missing. Six strokes closed age
AND `ZH-A1-F1-03` -- name, nationality, age -- because 名字 was already taught and
nationality had just been closed for no character at all. Four other uncovered
notes carrying the same stale blocker (`NUM-05`, `NG6-02`, `NG2-03`, `NG4-02`) are
narrowed rather than left alone, and `NG4-02` now says the calendar is the largest
single return left in the file.

### THE FONT WAS REGENERATED FROM ITS OWN SCRIPT

Ten characters were added to `data/scripts/chinese.json` with **source-verified PRC
stroke order**, each citing the same pinned Hanzi Writer Data commit the existing
fifty use, and `_fonts/subset-cjk.sh` was re-run. The upstream SHA was verified
against the pin in the script before anything was trusted, and the script was first
run unchanged to confirm it reproduces the committed subset **byte for byte**. A
character in a lesson and absent from that inventory is tofu that every other gate
passes.

**And the track's own punctuation habit was followed rather than broken.** The
inventory reports `Biaodian` at 0 of 7 -- not one mark taught in the corpus -- and
the subset font carries no CJK punctuation. So these lessons write no 。, no ，and
no ？either, and where Chinese would put a comma at a topic seam the page prints a
dash and says why.

### REINFORCEMENT, DECOMPOSED

Measured against the merged tree, not asserted:

```
reinforcementWindowMisses           238 -> 192
reinforcementMissesByWindow-R1       21 ->  21
reinforcementMissesByWindow-R2       57 ->  57
reinforcementMissesByWindow-R3       98 ->  87
reinforcementMissesByWindow-R4       62 ->  27
atomsNeverRevisited                  11 ->   9
atomsTaught                         166 -> 212
```

Adding thirty-five lessons makes **47** pre-existing atoms window-judged for the
first time (R2 6, R3 23, R4 18) -- debt the added LENGTH exposes rather than debt
the tranche creates. **All 47 are paid**, the tranche's own 46 atoms create **zero**
new debt in any window, and pre-existing debt falls by 46 on top of that (R3 11,
R4 35).

Every lesson opens with six recalls: R1 at distance 1, R2 at distance 5, two at R3
and two at R4. The second slot in each wide window is what pays the pre-existing
debt; one slot per lesson would only have covered the newly exposed. R1 and R2 do
not move and cannot -- their misses sit at positions the new lessons cannot reach
back into within a fifteen-lesson window.

### THE INVENTORY IS UPDATED IN THE SAME COMMIT

Landing 46 atoms and leaving the probes at `null` would have kept `98/191` reading
as `73/191` with a completely green suite. Both halves of the gate were falsified:
a fabricated atom id fails "probes only atoms that EXIST" AND the count, and nulling
a probe fails the count.

**The pinyin decision at `ZH-A1-PY-06` is untouched.** This file treats pinyin as a
pronunciation claim and not a script claim, a test forbids a pinyin point being
probed with a `ZH-SCRIPT` atom, and no point here re-litigates it.

### WHAT IT DELIBERATELY DOES NOT CLOSE

`ZH-A1-DEM-01` -- 这 and 那 -- is still the most expensive single gap in the file
and is the obvious next tranche after the calendar. 们 is one character from
`PRON-02` and half of `N-05`, whose first half this tranche closed. Food and drink
is one Spanish point and not one Mandarin word. Punctuation is seven points and not
one mark. And translation stays 0 of 2: no lesson in the track declares `mediation`.

