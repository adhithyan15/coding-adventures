## HL-C316 — the non-attributable R2 debt is four different defects, and only one of them is unfixable by spacing

HL-C313's second entry measured the corpus's R2 misses, attributed a third of
them to the one-new-word-per-lesson chapter shape, and left the rest
uncharacterised. It also listed seven tracks — Arabic, German, Italian, Marathi,
Persian, Portuguese, Urdu — as "do not apply this fix", because none of their
R2-missing atoms sit in a chapter matching that shape's signature.

**That do-not-apply list was derived from the wrong thing, and it is wrong.** It
came from a chapter's *bookkeeping* — does every lesson introduce exactly one
atom? — rather than from the *mechanism*, which is where the retrievals actually
land. The two disagree, and the mechanism is what the fix acts on.

### The decomposition

Measured on the merged tree with Telugu's fix in and Sanskrit's applied, so
these are the numbers a next agent inherits rather than the ones two PRs ago.
For every R2-missing atom outside the tranche signature, take the distances at
which the corpus does practise it and bucket by where they fall relative to the
5-15 window:

    corpus R2 misses                                   4509
      still carrying the one-per-lesson signature      1337
      everything else                                  3172
        onlyEarly  every retrieval at distance 1-4     1632   51%
        never      no retrieval at any distance         639   20%
        straddles  some before 5, some after 15,
                   none inside                          603   19%
        onlyLate   first retrieval later than 15        298    9%

    track          never  onlyEarly  onlyLate  straddles   total
    spanish           86        324        40        168     618
    german            73        107        15         11     206
    french            35        132        10          7     184
    hindi             71         58        45          9     183
    kannada           14         99        15         43     171
    tamil             49         78        23         20     170
    arabic           123         30        12          0     165
    portuguese        11        107        13         27     158
    malayalam         50         74        28          4     156
    italian           17         97         9         25     148
    latin             22         94        10         21     147
    telugu             9         55        11         63     138
    sanskrit          38         60        10          7     115
    urdu               7         53        19         29     108
    persian           10         58        13         18      99
    russian           17         65         5         11      98
    marathi            1         60         5         27      93
    punjabi            0         37         4         38      79
    gujarati           0         12         5         60      77
    chinese            6         15         3         10      34
    bengali            0         17         3          5      25

### What each bucket is, and what it wants

**onlyEarly (1632)** is the *same defect* as the tranche shape. The retrieval
exists and every instance of it is packed into distances 1-4. The chapter simply
does not carry the signature — one grammar lesson introducing two atoms, or one
interleaved script lesson, is enough to break it. **The chapter-boundary reach
fixes these.** Italian (97), Portuguese (107), German (107), Marathi (60) and
Persian (58) are in here — five of the seven tracks the earlier entry told the
next agent to skip.

**straddles (603)** is the genuine spacing gap: the course retrieves inside R1
and again past R3's edge and jumps the window in between. Gujarati (60 of 77),
Telugu (63), Kannada (43) and Punjabi (38) lead. The chapter-boundary reach
lands squarely in the hole, so it works here too, though the diagnosis differs —
the retrieval budget exists and is mis-spaced rather than absent.

**never (639)** is a different defect and no spacing rule touches it: the atom
is taught and never practised again at any distance, so it misses every window
rather than only R2. It is already tracked as `atomsNeverRevisited`. Arabic
(123) and German (73) dominate.

**onlyLate (298)** is the reverse defect — nothing until after lesson 15. These
atoms miss R1 as well and need a *nearer* reach, not a farther one. Hindi (45)
and Malayalam (28) lead. Pointing the chapter-boundary rule at these would add a
second late retrieval and leave the consolidation gap open.

### The corrected list

**A mid-window reach addresses 3572 of the remaining 4509 R2 misses** — 1337
still carrying the tranche signature, plus 1632 `onlyEarly`, plus 603
`straddles`. That is 79% of the debt, not the third the earlier entry claimed.
The 937 it cannot touch are 639 `never` and 298 `onlyLate`.

**Arabic is the only track that is genuinely mostly out of scope**: 123 of its
165 misses are `never`, so those atoms need a retrieval at all before spacing
them is a meaningful question. The other six tracks on the old do-not-apply list
are in scope and should be taken after the Indic tranches, since their debt is
`onlyEarly` and `straddles` — exactly what the reach fixes.

One caution for whoever takes Hindi. Its 183 non-attributable misses are 71
`never` and 45 `onlyLate` — the two buckets the reach does NOT help — against
only 58 `onlyEarly`. Hindi's attributable atoms are still worth the PR, but do
not expect its total to fall the way Telugu's and Sanskrit's did.
