# Human Languages — backlog

Note on provenance: this file was EMPTY in git for its whole history until now.
Findings from the pre-A1 tranche work were recorded in commit messages and PR
bodies, which are durable and searchable, but a reader opening this file found
nothing. The entries below are the ones that change how the work is done.

## HL-C09FK — Gujarati ઉ verifies the bowl-and-return pattern; ઊ is next

The t30apps.com animation gives **ઉ** as exactly one populated SVG path. It
circles the small upper bowl through the middle cusp, sweeps around the broad
lower bowl, then climbs the tall outer-left curve to finish at the upper right.
The fitted Noto Sans Gujarati median preserves that whole zero-lift sequence.

Gujarati is now **5/33 verified, 28 remaining**. Continue to **ઊ** next: its
adjacent animation repeats the complete **ઉ** path and then extends the same
unbroken run into a long right-side tail. That direct reuse makes it the
lowest-risk next provenance and fitting tranche. A correctness defect still
outranks coverage if one appears during fitting or validation.

## HL-C09FJ — Gujarati ઈ confirms the adjacent source pattern; ઉ is next

The next t30apps.com animation also has exactly one populated SVG path. It
repeats **ઇ** through the upper loop, middle crossing, and lower loop, then
extends the rising motion into **ઈ**'s high clockwise curl. The fitted Noto Sans
Gujarati median stays on the taller printed outline without inventing a lift.

Gujarati is now **4/33 verified, 29 remaining**. Continue to **ઉ** next: it is
the next independent vowel in source order and keeps this tranche inside one
already-audited source before moving to ઊ. A correctness defect still outranks
coverage if one appears during fitting or validation.

## HL-C09FI — Gujarati ઇ is the next verified ductus; ઈ now outranks the rest

The t30apps.com version-1.0 animation gives **ઇ** as exactly one populated SVG
path: small upper-left loop, narrow middle crossing, broad lower loop, then the
rising upper-right hook. Its other path slots are empty, so this is positive
zero-lift evidence rather than a guess based on the finished outline. The fitted
Noto Sans Gujarati median stays on the bundled glyph's ink and preserves that
upper-loop → lower-loop → hook order.

Gujarati is now **3/33 verified, 30 remaining**. The next item is **ઈ**: it is
adjacent in the same source, reuses the newly checked loop-and-hook anatomy, and
therefore has lower provenance and fitting risk than jumping to a different
script or a later Gujarati letter. After ઈ, continue in source order through
ઉ and ઊ unless a correctness defect outranks coverage.

## HL-C213 — build all 22 books LOCALLY before pushing; it takes ~100 seconds

Measured on a 14-core laptop, clean rebuild, 8-way parallel:

```
all 22 books        ~100s wall clock
sequential sum      ~367s
spanish alone         74s  (1337pp — sets the parallel floor)
```

The same work in CI takes **5 to 58 minutes**, and once hung **6 hours** in `apt`
fetching the TeX toolchain, having compiled nothing. So the LaTeX work is not the
cost — provisioning and queueing are.

`data/scripts/build_all_books.sh` runs the lot and checks the four things that
matter. **The exit code alone is not one of them:**

| check | why the exit code misses it |
|---|---|
| missing characters | a font gap prints NOTHING and still exits 0 — telugu once shipped 89 |
| overfull boxes | spanish crossed 1000pp, contents numbers gained a digit, 14 lines overflowed, exit 0 |
| underfull boxes | the fix for overfull can trade one for the other |
| exit code | catches only a hard LaTeX error |

`sh build_all_books.sh --self-test` proves each detector fires on a known-dirty log
and stays silent on a clean one. It is not decoration: **it caught two real bugs in
this script before either ever ran on the corpus** —

1. `grep -c` EXITS 1 when the count is zero, so `|| echo 0` appended a second zero
   and the integer test errored instead of comparing. That silently disabled the
   missing-character check — the very thing the script exists for.
2. The overfull/underfull probes never fired, because `printf '%s'` does not
   interpret backslashes in its ARGUMENT, so `\\hbox` stayed two backslashes.

A rebuild overwrites `book.log`, so a defect planted in a real log cannot survive
long enough to test the detector. That is why classification is factored into
`classify_log` and tested against synthetic logs instead.

## Three rules this work keeps re-deriving

**A measurement that did not happen looks exactly like one that passed.** Check the
magnitude, not just the verdict: a vitest run with a bad `--reporter` flag failed at
startup and EXITED 0; a scan with a wrong cwd read zero files and reported clean;
`grep $'\x00'` is an empty pattern that matches every file. Read the exit code, the
`Test Files N passed (N)` line, AND the count against a known baseline.

**Verify a checker against a known-dirty case before believing a clean result.**
Eleven detectors in this work reported clean while blind — from `\w` excluding the
combining marks under inspection, to an unassigned codepoint splitting a
mixed-script word into two clean halves, to a heredoc mangling a detector's own
fixtures.

**Fix the thing measured, never the threshold.** A gate on the LARGEST eager chunk
was once satisfied by splitting one 502 kB chunk into four — the page still
downloaded the same bytes. The real fix made the data lazy: 502 kB → 287 kB, and no
corpus growth reaches the eager graph at all.


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
