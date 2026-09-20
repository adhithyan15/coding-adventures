## Unreleased — the chapter boundary now reaches back far enough to enter R2

Sanskrit is the second track through the HL-C313 fix, after Telugu, and it was
measured on its own rather than assumed to behave like Telugu — which turned out
to matter twice.

A one-new-word-per-lesson chapter retrieves each of its words only from the
lessons that follow it inside the chapter, and the next chapter's opening lesson
reaches back for the last two. The largest distance the shape can produce is 4.
`continuity.ts` calls 5-15 lessons **R2, "first real retrieval"**, so the window
was not merely missed here, it was unreachable.

Chapters 20-51 now each carry, in every word lesson, one `[YOU RECALL: ...]`
task naming the word at the **matching position two chapters back** (the
load-bearing arm, landing at distance 8-12) and, where the chapter sizes put it
strictly inside the window, the word one chapter back as well. Spoken and read
alternate by position in the chapter:

    - [YOU RECALL: say *kṛpayā*, then read **द्वारम्**]
    - [YOU RECALL: read **क्षम्यताम्**, then say *āsanam*]

### Two things Sanskrit does not share with Telugu

**Chapter sizes vary here** — 4 words in chapters 21-23, 5 from 24 on — so a
fixed "same index" pairing does not cover a longer source chapter from a shorter
target. The mapping distributes source index *i* over the target chapter's
lessons instead, so every source word is retrieved by exactly one later lesson
whatever the two lengths are. Where two four-lesson chapters sit next to each
other the adjacent-chapter arm lands at distance 4 — R1, not R2 — and the
generator declines to write a line that would not count, rather than writing it
and reporting a number it did not earn. That happened for chapters 21 and 22;
both chapters' words are still covered, by the two-chapter arm from 23 and 24.

**`practises` is validated against the transitive prerequisite closure, not
against reading order** (HL-C316). Telugu's lesson chain is linear, so its
closure already reached back and the retrieval validated without further
change. Sanskrit's is not, and the same edit produced 28
`practice-before-introduction` errors for atoms that are taught eighty lessons
earlier. Each retrieving lesson now names the lesson it retrieves among its
`prerequisites`, which is the correct claim anyway and the way the corpus
already spells a hand-authored reach-back.

### Every number re-measured against the merged tree, not derived

    sanskrit R2 misses (5-15, "first real retrieval")   302 -> 145   (-157)
    sanskrit R1 misses (1-3)                             81 ->  81   (held)
    sanskrit R3 misses (20-60)                          374 -> 374   (held)
    sanskrit R4 misses (80-250)                         313 -> 313   (held)
    sanskrit reinforcement window misses               1070 -> 913
    sanskrit atoms taught                               406 -> 406   (held)
    sanskrit atoms never revisited                       46 ->  44   (improved)
    sanskrit lessons                                    335 -> 335   (held)
    forward prerequisites                                 0 ->   0   (held)
    forward references                                    3 ->   3   (held)
    corpus R2 misses                                   4666 -> 4509
    lessons at or over the 300s ceiling (ch18-51)         0 ->   0
    computed seconds, median of ch18-51                 115 -> 120

Chapters 18-50 were the whole tranche and 157 of their judgeable atoms now enter
R2. Every one of those figures was re-measured after `git merge origin/main`
brought in chapters 52-61 (#14170) and Telugu's own fix (#14173) — the first
draft of this section read `279 -> 122` against a 304-lesson track, and both
ends of that moved when the track grew to 335. The delta did not, because the
edited range is untouched by the new chapters.

What is left in Sanskrit is not this shape: 115 misses split between chapters
1-17, which mix word, writing, phrase, etymology and practice lessons and need
their own reading, and the new chapters 52-61, whose grammar and review lessons
are a different shape again. Chapter 51's five atoms are in that remainder for a
reason worth naming: they were correctly NOT counted while chapter 51 ended the
track — `continuity.ts` skips a window a track is too short to contain — and
became judgeable the moment chapter 52 landed. Chapter 52 has three word
lessons against chapter 51's five, which puts the reach at distances 3 to 5, and
the generator refuses to write a line that would not count. They close when a
uniform chapter follows them.

The derivation was falsified before shipping, not merely asserted: reverting the
single lesson `SA-C20-man` and re-measuring put R2 back one, with the chapter 18
atom that lesson retrieves reappearing as the extra miss.

### One unrelated red test, fixed rather than filed

Merging `origin/main` brought the banned-word ceiling (HL10 §7.4) to 1081
against a pinned 1077 — four new occurrences of `just` and `simply` arrived with
chapters 52-61 and were red on `main` before this branch touched anything. The
ceiling's own comment says the fix is the sentence and never the number, so five
sentences in `SA-C52-ca`, `SA-C52-va`, `SA-C59-bharatiyah`, `SA-C60-danda` and
`SA-R58-likes` were rewritten: 1081 -> 1076 occurrences across 872 lessons, both
back under the pin. The pin itself was left at 1077 rather than ratcheted to
1076; it is a contended counter and lowering it from one branch is how two
branches end up disagreeing about it.

