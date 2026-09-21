## Unreleased — the retrieval is seated per lesson, and Malayalam enters R2

Seventh track through the HL-C313 fix, using the per-lesson seating rule
(HL-C318). Of Malayalam's 278 R2 misses, 254 had a word lesson with duration
headroom inside their window; **219 of those closed.**

### One track-specific invariant this fix had to learn about

`tests/corpus/malayalam.test.ts` asserts that chapter 7's two spoken number
lessons contain **no Malayalam glyph at all** — a deliberate meaning-first step
taken before the script arrives. Glyph closure does not protect them: by chapter
7 the letters HAVE been taught, so a `read **X**` line passes the closure check
and still breaks the lesson. It did, on the first run.

The guard is therefore not "are these glyphs taught" but **"does this lesson
already use the script"**. A lesson showing no target script is showing none on
purpose, so it gets a spoken retrieval. That needs no list of which lessons are
special and preserves the same intent in any track that has made the same
choice.

### Every number re-measured against the merged tree, not derived

    malayalam R2 misses (5-15)                        278 ->  59   (-219)
    malayalam R1 misses (1-3)                         117 -> 117   (held)
    malayalam R3 misses (20-60)                       319 -> 319   (held)
    malayalam R4 misses (80-250)                      275 -> 275   (held)
    malayalam reinforcement window misses             789 -> 570
    malayalam atoms taught                            358 -> 358   (held)
    malayalam atoms never revisited                    54 ->  24   (improved)
    malayalam lessons                                 292 -> 292   (held)
    forward prerequisites                               0 ->   0   (held)
    forward references                                 12 ->  12   (held)
    script closure violations                         271 -> 271   (held)
    corpus R2 misses                                 4212 -> 3725
    lessons at or over the 300s ceiling                 0 ->   0
    computed seconds, median                          140 -> 142

164 lessons gained a line; the book carries 164 recall lines, 72 with a read.

Falsified before shipping: reverting `ML-C46-book` and re-measuring put R2 back
to 60.

Of the 59 that remain, only 6 were introduced by a word lesson; the rest are
phrase (24), writing (24), etymology (3) and grammar (2) atoms, which need a
different recall phrasing before they can be scheduled.

