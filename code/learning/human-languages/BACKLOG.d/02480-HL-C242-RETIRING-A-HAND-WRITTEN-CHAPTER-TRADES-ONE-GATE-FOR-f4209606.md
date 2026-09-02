## HL-C242 — retiring a hand-written chapter trades one gate for another: German 1 and 2 are generated

German was the worst-blocked track in the corpus: 16 hand-written chapters and 78
prose blocks that a naive flip would have deleted. Chapters 1 and 2 are now
generated from schema-v2 lessons. **14 chapters and 77 blocks remain**; this
entry records what the first slice cost so the next fourteen are cheaper.

### The parity script measures the wrong half of the work

`handwritten_parity.py german` reported chapter 1 at a gap of **zero** — and
chapter 1 was still not flippable, because `book.ts` refuses any chapter whose
lessons are not schema v2:

```
Error: GE-C01-gut: generated books require schema version 2
```

Eleven of chapter 1's thirteen lessons were legacy. Across chapters 1–16 there
were **65** of them. So the real unit of work is not "carry N prose blocks", it
is "migrate N lessons to v2, and carry the prose while you are in there." The
parity number is a *lower* bound on the job, and a chapter reading `gap 0` says
nothing about whether it can be generated at all.

**Do this first for any remaining chapter**: count its legacy lessons, not its
prose blocks. Chapters 6–16 have two or three lessons each for a whole chapter of
material (numbers 1–10 in two lessons, the twelve months in one), so those are
lesson-*authoring* jobs, not migrations.

### Where the prose actually went

The four block headings the renderer maps are the easy part. What the block-count
gate cannot see is a sentence dropped inside a block that survived. A crude
vocabulary diff of the old `.tex` against the generated one found eight of them
in chapter 1 alone — the plurals *die Tage / die Morgen / die Abende / die
Nächte*, the rule that every German plural takes *die* whatever the gender, *Tag*
rhyming with English *tock*, the *Nacht* scrape being the same sound as Spanish
*j* and Arabic *kh*, the full *ich wünsche Ihnen einen guten Tag*. None of them
was a block; all of them were content.

The diff is four lines of Python (strip LaTeX macros, lowercase, set-difference
the words) and it is worth re-running per chapter. Most of its output is quoting
noise — `` day'' `` from `` ``day'' `` — but the residue is real.

### Splitting is how the atom budget is paid, and it does not always fit

`chapter-policy.json` allows **3 new atoms per lesson**. A word lesson that has a
sounds block, an etymology block and a grammar lens is already at four unless the
grammar lens only elaborates the word. The rule this slice settled on, and the
next slice should keep:

> One lexical atom, one sound atom, one etymology atom per word lesson. A grammar
> lens that explains *this word* assesses the lexical atom. A grammar lens that
> teaches a **transferable rule** — adjective endings, verb endings, verb-second,
> the plural article — gets its own atom, and if that makes four, it gets its own
> lesson.

Three lessons came out of that rule: `GE-C01-die-plural`, `GE-C02-heissen-endungen`
and `GE-C02-mich`. Each carries prose that was in the LaTeX and had no home.

### The wall the next slice will hit

The same policy allows **12 new atoms per chapter**. Chapter 1 has 31 across 14
lessons; chapter 2 has 23 across 10. Both are now counted, so the corpus line
moved from "27 chapters above 12" to 29 and German's `atom-step` queue from 8 to
10.

**This is a measurement, not a regression.** Those chapters were schema-v1 and
therefore invisible to the ramp gate; German's measurement-blind lessons fell 65
→ 46 in the same change. But it is a real finding: a chapter that teaches eleven
headwords cannot satisfy a twelve-atom chapter budget while also teaching one
headword per lesson. The already-generated German chapters (17, 24–31) run three
to four lessons each, which is what the budget implies.

So the honest end state for chapters 1–2 is a **chapter split**, not more atom
accounting — and a split renumbers every later German chapter, every
`chapters.d` shard, every `book-generation.d` owner and every `.tex` filename.
That is its own PR and it should not be smuggled into a retirement slice. Left
open deliberately.

### `morphologybox` and `etymology` are still unanswered

Chapters 1 and 2 contained neither, so this slice did not have to decide. The
remaining fourteen hold **17 `morphologybox`** and **7 `etymology`** environments
that the generator cannot emit at all. `grammarlens` and `cousinweb` are the
natural homes; whoever takes chapters 3–5 owns that call.

### Two pins moved, and one was already red

`tests/grouped-shards.test.ts` pins the handwritten owner count as a deliberate
tripwire; 69 → 67 is the tripwire firing correctly. `tests/corpus/german.test.ts`
pins measured lessons, 42 → 64. Both are the designed consequence of the flip.

`tests/script-closure.test.ts` expects the corpus above 500 closure violations and
measures 477. German is a Latin-script track and is excluded from that measure
entirely (`measureScriptClosure` never lists it), so this failure predates this
work and no German change can move it. It needs its own owner.
