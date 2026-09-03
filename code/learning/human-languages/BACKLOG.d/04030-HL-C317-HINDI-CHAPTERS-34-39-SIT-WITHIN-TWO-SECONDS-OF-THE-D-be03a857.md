## HL-C317 — Hindi chapters 34-39 sit within two seconds of the duration ceiling and cannot accept any new content

Found while adding one retrieval line per lesson for the R2 fix (HL-C313). The
line costs about four computed seconds. On Telugu and Sanskrit that was
invisible — their word lessons compute at 110-160s against a 300s ceiling. On
Hindi it turned three lessons red immediately:

    HI-C36-kursi   299 -> 302
    HI-C38-pet     298 -> 302
    HI-C38-daant   298 -> 301

The three were already at 298-299. **They did not become fragile; they were
already fragile, and a four-second addition was enough to find out.**

### The measurement

Effective seconds per word lesson, Hindi, `estimateLessonDuration` on the merged
tree, before this change:

    chapter   lessons   min   max   with 25s of headroom
       34        4      268   279          1
       35        3      258   275          3
       36        4      282   299          0
       37        3      263   296          1
       38        4      275   298          1
       39        4      276   298          0
       40+       …      210   210          all

Twenty-two word lessons in chapters 34-39; **six of them have room for a
sentence.** Every chapter from 40 on declares 210s and computes well under it,
which is why the same edit was free everywhere else in the track.

These are the hand-authored chapters. `HI-C36-kursi` is representative: 464
words, three introduced atoms (feminine gender, the -ā/-ī lean, and the
four-thousand-year Sumerian-to-Arabic etymology), two grammar-lens blocks, a
letters block, five practice bullets and a three-question wrap-up. It is three
lessons wearing one lesson's frontmatter, and the chapter policy's
`maxNewAtomsPerLesson` of 3 lets it through at exactly the cap.

### What this blocks, and what was done instead

Chapters 34 and 36 could not be given a retrieval at all: every candidate seat
inside the R2 window is one of these full lessons. That is 13 atoms the R2 fix
left open in an otherwise-covered range, and they are named in the Hindi
changelog rather than quietly absorbed.

The retrieval was NOT shortened to fit. Placement moved instead — within a
chapter, the line goes to a lesson that has budget and still lands in the
window — and where no such seat exists the chapter is reported uncovered. A
line trimmed until it fits is a line that no longer asks for anything, which is
the failure mode the ceiling exists to prevent, not the one it is asking for.

### The fix, when someone takes it

Split. `HI-C36-kursi` splits cleanly along its own block boundaries: the gender
lesson (KURSI-01 and the -ī lean) and the etymology lesson (KURSI-02) are
already separate teaching points with separate blocks. `HI-C38-pet`,
`HI-C38-daant`, `HI-C39-aurat`, `HI-C37-kitaab` and `HI-C35-pasand` are the same
shape. The corpus has done this before — see the Urdu *shukriya* split recorded
in `ramp.test.ts`'s `unmeasurableLessons` history, which moved that counter
one-for-one — so the procedure and the counters it touches are known.

Doing it here would have meant writing six new lessons inside a PR about
retrieval spacing, and mixing a content-density fix into a scheduling fix makes
both harder to review. It is a real piece of work with a real number attached,
which is why it is an entry and not a footnote.

**Until it is done, chapters 34-39 are closed to new content of any kind** — not
just retrieval lines. Any sentence added to `HI-C36-kursi` fails
`ramp.test.ts`'s duration gate, and the failure will name the duration rather
than the sentence, so the next agent to touch these will lose time to it unless
they read this first.
