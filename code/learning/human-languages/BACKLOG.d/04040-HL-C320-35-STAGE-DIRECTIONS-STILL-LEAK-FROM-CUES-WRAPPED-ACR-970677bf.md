## HL-C320 — 35 stage directions still leak from cues WRAPPED across source lines

`bookVoice` converted `[YOU SAY: …]` cues only inside a bullet list. On any other
line the cue fell through to `stripDeliveryCues`, which handles only `[PAUSE …]`
and `[REPEAT …]`, so the cue was emitted verbatim and LaTeX-escaped into
`{[}YOU SAY: …{]}` **on the printed page**.

No gate could see it: `check:books` compares the generator against itself, and
the escaping produces valid LaTeX that compiles cleanly.

**93 leaked markers across 30 chapter files in 10 tracks** — spanish 14 files,
hindi 5, gujarati 4, tamil 3, punjabi 2, marathi 2, malayalam 2, russian 1,
marwadi 1, french 1. By kind: 55 `YOU SAY`, 37 `YOU HEAR`, 1 `YOU ANSWER`.

**58 are fixed**; two shapes now render in book voice like their bullet
equivalents:

* the cue filling a whole non-bullet line;
* the cue **inline** — as a prefix followed by prose, or several in one sentence
  (`[YOU SAY: *a*], [YOU SAY: *b*]`).

A lazy match to the first `]` is used for the inline form only, and that is safe
by measurement rather than by assumption: across every lesson in the corpus, 26
lines carry trailing prose after the close bracket and **not one** has a `[`
inside the cue content. The whole-line form keeps its end-anchored greedy match,
because there the copy legitimately may contain brackets.

### What remains — 35 leaks, one shape

The cue is **wrapped across source lines**: `[YOU SAY: *…*` on one line with its
closing `]` on the next. `bookVoice` walks line by line and cannot see it.

Example: `gujarati/lessons/GU-R17-introduction-r4.md:46`, printing into
`gujarati/book/chapters/ch21-r4-respectful-questions.tex:83`.

A first attempt joined continuation lines before matching and fixed only **one**
of the 35, which means the remaining sites do not reach `bookVoice` by the path
assumed — the join was reverted rather than shipped, since an ineffective change
to a shared renderer is worse than none. **The next attempt should start by
finding which render path those lines actually take**, not by widening the
match. `renderBlock` is the only caller of `bookVoice`; `renderReferenceMarkdown`
bypasses it.

Do not fix this by editing the 35 lesson sites. A rendering rule is a candidate
list, not a patch, and the generator should own the treatment so the next author
who wraps a cue is not punished for it.
