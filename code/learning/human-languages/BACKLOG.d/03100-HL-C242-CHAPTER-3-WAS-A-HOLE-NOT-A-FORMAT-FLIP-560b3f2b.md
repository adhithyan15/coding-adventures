## HL-C242 — chapter 3 was a hole, not a format flip: budget the authoring, not the conversion

German chapter 3 is now generated. **13 hand-written German chapters and 69
prose blocks remain.** The first slice (chapters 1–2) recorded what conversion
costs; this entry records the thing that slice did not have to pay, and the next
thirteen will.

### The parity gap understated the work by more than half

`handwritten_parity.py german` reported chapter 3 at a gap of **8** blocks. The
real deficit was not eight blocks of prose — it was **ten missing lessons**.

The chapter's LaTeX taught about twenty things across eight lessons. *es*,
*mir*, *dir*, *Ihnen*, *sehr*, *nicht*, *und*, *so lala*, *schön* and *vielen
Dank* were all taught on the page and owned by no lesson at all. The parity
script cannot see this, because it counts LaTeX *environments*, and a word
taught in a paragraph inside a surviving `cousinweb` costs zero blocks while
costing a whole lesson.

Worse, the lessons that did exist were over budget in a way the v1 schema hid:
`GE-C03-gehen` taught *gehen*, *es*, *mir* and the dative in one sitting. Under
`maxNewAtomsPerLesson: 3` that is one lesson doing four lessons' work, and it
only became visible once the lesson declared atoms.

**So the estimate for chapters 4–16 should be lessons, not blocks.** Chapter 3
went 8 lessons -> 18. Counting the words its `.tex` teaches against the lessons
that own them takes a few minutes and predicts the work; the block gap does not.

### Two defects only the rendered page showed

Both passed every text-level gate.

1. Both practice lessons wrote their four-line dialogue as consecutive `>`
   lines. Markdown joins those into one paragraph, so the exchange printed as a
   single run-on line. Chapter 2's practice lesson had already solved this with
   a two-column German/English table — **one exchange line per row.** Prefer that
   shape for any dialogue; a multi-line blockquote will not survive.
2. `GE-C03-gehen` declared the sound tag `h-pronounced` while its own prose said
   the *h* "merely lengthens the vowel." The registry has `h-silent-lengthening`.
   A tag from the closed vocabulary is still validated only for *membership*,
   never against what the lesson says — so a wrong-but-registered tag is
   invisible to `validate`.

This is the second slice in a row where compiling the book and reading the page
found something the gates could not. Budget for it.

### Atom ids are first-come, and chapter order is not id order

Chapter 3 introduces *gehen* first, but `GE-C27-gehen` already owned
`GE-LEX-GEHEN-02`, with three chapter-27 lessons requiring it —
`schema-v2-duplicate-knowledge-introduction` is a hard error, so one side had to
move. Re-pointing an already-generated chapter's atoms is the larger blast
radius, so chapter 3 minted `GE-LEX-GEHEN-01` and chapter 27 kept `-02`.

**Expect this for every remaining chapter**, because chapters 17–31 are already
generated and own atoms for words the earlier chapters teach first. Check
`grep -rl GE-LEX-<WORD>` before minting, and prefer a new ordinal on the *earlier*
chapter over re-pointing a later one.

### Still open

The chapter atom budget is still the unfixed thing. Chapter 3 introduces 36
atoms against `maxNewAtomsPerChapter: 12`, so German's over-budget chapter count
goes 2 -> 3. As with chapters 1 and 2, the honest fix is a chapter split, which
renumbers every later German chapter, and it is deliberately not attempted here.
