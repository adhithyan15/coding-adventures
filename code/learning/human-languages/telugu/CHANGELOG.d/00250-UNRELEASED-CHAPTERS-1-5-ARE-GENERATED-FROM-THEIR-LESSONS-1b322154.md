## Unreleased — chapters 1-5 are generated from their lessons

Telugu has **no hand-written chapters left**. `ch01`–`ch05` were the last five;
they are now built from the same lessons everything else is built from, and the
corpus's handwritten holding falls **69 → 64**.

### What had to happen first, and what it cost

`handwritten_parity.py` put Telugu at **7 blocks** of prose that lived only in
the LaTeX and would have been deleted by a naive flip. All seven are carried,
and `--check telugu` exited 0 **with the five chapters still hand-written** —
which is the only ordering in which that gate proves anything:

* **`sounds` ×1.** `TE-C01-namaskaram` was the only chapter-1 word lesson with
  no "Sounds you'll need" section. Its four siblings all had one; now it does.
* **`cognates` ×5.** The renderer has no `cognates` environment and no reason to
  grow one. An unclassified `##` heading already renders as a `\subsection*`,
  and two of the five tables reached the page that way already, so the other
  three were carried the same way — into `TE-C01-namaskaram`, `TE-C02-peru` and
  `TE-C04-velli-vastaanu`. `TE-C02-peru` held its comparison as a run-on
  sentence and `TE-C04-velli-vastaanu` inside a paragraph; both are now the
  five-row table the `.tex` had. The tables stay romanization-only, exactly as
  the `.tex` wrote them, so no lesson gained a cousin-script glyph.
* **`culture` ×1.** The chapter-1 recap's "go and come back" farewell. Not lost
  — **re-homed**, and deliberately: teaching a sixth phrase in the recap of a
  five-word chapter is the ramp violation this migration exists to fix. It lives
  in chapter 4 on `TE-C04-velli-vastaanu`, where the reader has met వెళ్ళు
  first, and chapter 1 keeps the forward pointer `TE-C01-practice` already had.

### The real blocker was schema v1

Moving the ledger entries produced one error: *"generated books require schema
version 2."* All **thirty** word, phrase and recap lessons in chapters 1-5 were
schema v1 — the same thirty the gentle-ramp report had been calling
*measurement-blind*. So the flip is a migration:

* every `##` heading now carries a stable block type (sixteen did not: five
  "Across the family" tables folded into the etymology block they belong in,
  "The phrase, assembled" became "The phrase, taken apart", and the recap
  sections took the "The exchange" / "What you've built" / "Guided Practice"
  spellings the parser knows);
* every block declares `hl-knowledge`, every lesson declares atoms, duration,
  skills, modes, strands, register and variety;
* `TE-C02-practice` gained the warm-up a v2 lesson must open with;
* the four chapter recaps that were absent from the local path were placed in
  it, each with a consolidation extension node, matching `TE-C05-practice`.

**No lesson had to be split.** The computed five-minute ceiling is derived from
content, and the heaviest lesson in chapters 1-5 computes at **252 s** against
the 300 s bar — chapters 1-5 were already one headword per lesson, with the
letter ladder interleaved, before this tranche started.

### Reading the chapters side by side found three things the script cannot see

The parity script counts prose BLOCKS. `center`/`tabular` are layout to it, so
it is blind to a table. Reading the old and new chapters against each other:

* **Three end-of-chapter summary tables** (the how-are-you exchange, the three
  farewells, the three sentences about yourself) existed only in the `.tex`.
  They are now in `TE-C03-practice`, `TE-C04-practice` and `TE-C05-practice`,
  with a romanization column the originals did not have.
* **Five chapter-opening paragraphs** had no home in a generated chapter, which
  builds its opening from `canDo` and `payoff.summary`. The writing moved into
  `chapters.d`, so the reader still opens chapter 4 on *"Telugu … does not say a
  plain 'goodbye.' It promises to return."*
* **`cousinweb` drops its block heading**, so "Roots you now carry" would have
  arrived as an unlabelled box of bullets. The label moved inside the box.

### Numbers

| | before | after |
|---|---|---|
| hand-written chapters | 5 | **0** |
| `handwritten_parity.py --check telugu` | 7 blocks at risk | **0** |
| schema-v2 lessons in chapters 1-5 | 24 of 54 | **54 of 54** |
| atom-measurement-blind lessons | 30 | **0** |
| chapter payoff debt | 5 findings | **0** — telugu joins the clean list |
| lessons over the 300 s ceiling | 0 | 0 |
| headwords without romanization | 0 | 0 |
| script-closure violations | 6 | 9 |

The closure movement is the recap tables described above: they show the
chapter's sentences in Telugu, as the hand-written book always did, and five
characters in them — **గ · బ · ళ · ె · చ** — have not had a letter lesson yet.
Each table now says so on the page, and the characters are named in
`BACKLOG.d` HL-C282 as the next ladder rungs. `TE-C05-undu` also lost a
zero-width non-joiner, which the emitter was placing OUTSIDE the script run
where it could neither join nor render.


