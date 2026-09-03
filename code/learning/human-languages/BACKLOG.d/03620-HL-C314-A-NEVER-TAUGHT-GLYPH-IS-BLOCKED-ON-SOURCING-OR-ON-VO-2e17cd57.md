## HL-C314 — a never-taught glyph is blocked on sourcing or on vocabulary, and only one of those is a refusal

Found while teaching `ऋ` in the Sanskrit joining tranche. The track's script
report says five characters are shown and never taught — `ऋ ङ ई घ ँ` — and both
the Sanskrit roadmap and `exam-inventory-sanskrit-a1.json` described that as two
kinds of blocked, using `ऋ` and `ङ` as the two examples. Measured against
`data/scripts/devanagari.json` rather than assumed, it is **three and two**:

    glyph  ductus in devanagari.json   in a headword   blocked on
    ऋ      yes (Saurmandal, 4 panels)  no  -> now yes  VOCABULARY  (now taught)
    ई      yes (Saurmandal, 3 panels)  no               VOCABULARY
    घ      yes (Opiaterein, GIF)       no               VOCABULARY
    ङ      NO                          yes              SOURCING
    ँ      NO                          no               SOURCING

The distinction matters because the two halves have completely different costs
and completely different verdicts.

**Blocked on vocabulary is a schedule, not a refusal.** The ductus is already
sourced and cited; what is missing is a headword for the recognition segment's
"you already say these" list, which HL-C217 requires to be non-empty. The fix is
one vocabulary lesson followed by one script lesson, in that order. `ऋ` cost
exactly that: `ऋतुः` ("a season"), then the shape. `ई` and `घ` are one word away
each.

**Blocked on sourcing is a refusal and must stay one.** There is no pen path in
the shared script file, and HL11 §5 forbids inventing one. `ङ` shows why having a
headword does not help: it has ridden `सङ्ख्याशब्दाः` since the numbers chapter
and is still untouchable, because the missing half is the half that cannot be
manufactured.

TWO THINGS TO CARRY

1. **Read the script file before writing the refusal.** Both of the documents
   that described this split had the right shape and the wrong membership, and
   both were written by someone reasoning from the five-character list rather
   than from `devanagari.json`. The list says which glyphs are untaught; only the
   script file says why.
2. **A never-taught-glyph count is two queues, not one.** Reporting "five
   untaught" hides that three of them are a week of vocabulary and two of them
   are waiting on an upstream source. The same is likely true in every Indic
   track that carries this metric, and the census is cheap: one pass over
   `devanagari.json` plus one grep of the headwords.
