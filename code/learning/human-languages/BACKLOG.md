
## HL-C214 — class (d) is MECHANIZABLE: glossed-but-never-a-headword

HL-C207 named the collision class no string search reaches — a candidate that
collides with a GLOSS rather than a token — and said mechanising it would need a
schema change. **It does not.** The Hindi round-3 tranche derived it from data
already in the corpus:

1. extract every script token appearing anywhere in the track (Hindi: 587 distinct);
2. subtract the set of tokens that are some lesson's HEADWORD;
3. what remains — **67 tokens for Hindi** — is the set of words the reader has been
   handed a meaning for without ever being taught them.

Every class-(d) collision found by hand on five previous tracks lands in that set.
Hindi's discards from it include अलमारी and चाबी (both glossed inline in a ROOM
lesson listing Portuguese loans), ठंड (glossed as the stock phrase ठंड है), मीठा
(glossed inside another word's etymology), and तिल (glossed in an OIL lesson).
Four of those were on the shortlist and would have shipped.

**This is a report, not a gate.** The set is a candidate list to READ, not to reject
automatically: the same tranche cleared मिट्टी, बीज, अनाज and टोकरी after reading
the sentences, because the gloss was of an English word, a different word, or an
absence. A rule that only rejects is a filter.

**KNOWN LIMIT, found by running it first on sanskrit:** the check operates on
SCRIPT tokens, so a word glossed only in ROMANIZATION is invisible to it. Sanskrit's
कथा was caught by a person reading `SA-C03-katham`, which glosses *kathā* as "a
telling, a story" without ever printing the Devanagari. **Run the same three steps a
second time over romanized forms**, or accept that a human still has to read the
etymology notes. The mechanized pass narrows the field; it does not close it.

Sanskrit's numbers, running it FIRST rather than last: **267 glossed-but-never-taught
tokens out of 495 distinct**. It reshaped the word list instead of invalidating it —
3 discarded (मृगः glossed outright in a ROAD lesson; कथा; शीतम् glossed inside
"himam is snow, and cold generally"), 3 turned INTO teaching (सिंहः, handed over
twice inside the names *Siṁhapura* and *Narasiṁha*; शब्दः, which three earlier
headwords END in without any of them naming it; प्रकाशः, whose काश् root a SKY
lesson had already given away), and 2 examined and kept because the colliding
sentence was an English description rather than a gloss.

Worth building as `report --glossed-not-taught <track>`. Until then, the three
steps above are cheap to run inline and catch the class that nothing else does.

## HL-C215 — two pre-existing mixed-script typos in Hindi, reported not fixed

A per-word purity sweep of all 485 files in the Hindi tree found exactly two, both
shipping into the PDF and the narration:

```
HI-C33-shubh-dopahar.md:40   "the widened सense"   DEVANAGARI SA standing in for Latin s
HI-C32-shubh-sandhya.md:54,76 "रात्रि became राat"  should be रात
```

Both render as plausible wrong text — the same class as the Telugu and Malayalam
defects already fixed under HL-C202, and invisible to any check that looks at one
script at a time.

**Left unfixed deliberately.** They sit in chapters unrelated to the tranche that
found them, and touching them would pull those chapters' book and narration hashes
into an unrelated diff. They want their own small change.

Not a defect, do not "fix": `HI-C25-din.md:56` carries Proto-Slavic `*dьnь`, whose
Cyrillic yers inside a Latin transliteration are correct scholarly notation.
