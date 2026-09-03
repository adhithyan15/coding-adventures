## HL-C319 — retrieval-only lessons are a mechanism, not the fix: Japanese is not distinguished by having them

Japanese is the only track with zero findings in every gentle-ramp queue, and
the reading offered for that is its **17 of 117 lessons that introduce nothing
and exist only to reach back** — dedicated retrieval lessons rather than
retrieval bolted onto teaching lessons. The suggestion was that this shape might
answer both the `never` bucket (HL-C316) and the duration cliff (HL-C317).

It is the right mechanism for both. **It is not what makes Japanese Japanese**,
and a plan built on "add retrieval-only lessons" without the rest will not
reproduce the result.

### Counted across the corpus: lessons that introduce nothing and practise something

    marathi     230 lessons   77   33.5%      japanese  117 lessons  17   14.5%
    chinese     175 lessons   50   28.6%      spanish  1155 lessons 166   14.4%
    gujarati    263 lessons   55   20.9%      french    232 lessons  30   12.9%
    marwadi     257 lessons   45   17.5%      german    211 lessons  16    7.6%
    punjabi     226 lessons   37   16.4%      hindi     343 lessons  15    4.4%
                                              tamil     390 lessons  13    3.3%
                                              kannada   303 lessons   5    1.7%
                                              telugu    336 lessons   5    1.5%

**Five tracks carry a larger share of retrieval-only lessons than Japanese, and
four of them have substantial R2 debt** — Marathi 93 misses at 33.5%, Chinese 55
at 28.6%, Gujarati 124 at 20.9%, Punjabi 113 at 16.4%. Marwadi is the exception
that proves the point: 17.5% and zero misses in every window. Having the lessons
is not the property.

The property is where they reach. The distance histograms say it directly —
practice events by distance from introduction:

    japanese  d1:102 d2:31  d3:18  d4:17 | d5:74  ... d20:30
    marwadi   d1:215 d2:143 d3:113 d4:92 | d5:140 ... d20:131
    hindi     d1:244 d2:202 d3:95  d4:61 | d5:20  d6:9  d7:11
    sanskrit  d1:203 d2:178 d3:114 d4:44 | d5:20  d6:43 d7:22

Japanese and Marwadi — the two tracks with **zero misses in every window** —
have deliberate humps at distance 5 and again at 20. The others fall off a cliff
after 4. A retrieval-only lesson that sits next to what it reviews contributes
to the R1 pile like any other; the schedule is what makes it count.

### It does answer the `never` bucket, and my earlier claim was too strong

HL-C316 said of the 639 `never` atoms that "no spacing rule touches" them. That
is true of a spacing rule and false as a statement about the corpus. Measured:

    atoms that miss R2 with no retrieval at ANY distance      649
    of which a position exists inside their own 5-15 window   649

**Every one of them has somewhere for a retrieval to go.** They are not
unreachable; nothing sources them. The reason the seating rule (HL-C318) does
not is narrower and fixable: it sources retrieval only from WORD lessons,
because a headword plus a romanization is what makes `say *X*` / `read **X**` a
real task. The remainders show it — of Kannada's surviving 68, four came from a
word lesson and the rest from phrase (32), writing (29) and grammar (3); of
Bengali's 51, thirty-two came from a writing lesson.

So the work `never` needs is **a recall phrasing for non-word atoms** — a
letter, a phrase, a grammar cell — and once that exists a retrieval-only lesson
can carry any of them. Arabic, whose 123 `never` atoms are 75% of its R2 debt,
is the track that would move most.

### It also answers the duration cliff, and that is its strongest case

A retrieval-only lesson is a NEW seat that costs no existing lesson any budget.
The atoms currently uncovered purely because every seat in their window is at
the 300-second ceiling:

    urdu       36        malayalam   23 + 1 with no seat at all
    tamil      33        kannada     20 + 2
    hindi      13        bengali     13 with no word lesson in window

Roughly 140 atoms, and at three recalled items per lesson on the order of fifty
new lessons across six tracks. That is cheaper than splitting the dense
hand-authored chapters (HL-C317) and reaches Bengali's thirteen, which splitting
does not — those have no word lesson in the window at all, so only a lesson that
does not yet exist can reach them.

### Do not do this inline

This is content, not scheduling: a retrieval-only lesson needs an id, a
sequence, a place in the chapter ledger, and a body somebody wrote. Every
distance in the R2 fix is computed from reading position, so inserting lessons
moves every measurement after the insertion point — the fix has to be
re-measured on the track afterwards, not adjusted.

Sequence it after HL-C317's splits or instead of them, decide per track which is
cheaper, and measure the seat capacity again once the lessons exist.
