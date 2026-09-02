## HL-C201 — script closure and forward references pull against each other

Resequencing the Kannada letter ladder to fix script closure made the forward-
reference count jump from 13 to 42 in a single move, and the two numbers turned
out to be measuring opposite ends of the same decision.

`measureScriptClosure` wants a letter taught **before** the first word that
needs it. `measureContinuity`'s forward-reference detector flags a lesson whose
body prints a headword some **later** lesson teaches. A letter lesson's whole
pedagogical content is a list of words containing that letter — so a ladder
ordered by first consumer is, by construction, a ladder of forward references.
For most Kannada letters the first word containing the letter *is* the first
word that needs it, and then no placement satisfies both rules.

Three consequences worth carrying forward:

1. **A letter's example list is the constraint, not its position.** Kannada's
   ladder moved from chapter 6 to chapters 1–8 only because ten lessons' "you
   already say these" lists were rewritten to cite words the reader had
   actually met. That claim had been false at the new positions; fixing it is
   what held forward references at 13.
2. **Two letters have no legal placement.** ಬ appears in no Kannada word before
   ಬಾ, which is inside the lesson that first needs it; ೊ likewise with ಗೊತ್ತು.
   `KA-S110-letter-ba` and `KA-S114-vowel-sign-o` were moved to sit after their
   word rather than before their consumer, which costs one closure violation
   each and is the honest trade.
3. **Grid lessons are the remaining blocker.** Five of Kannada's ten remaining
   closure violations wait on ಅ or ಭ, which exist only inside
   `KA-S122-letter-ca` and `KA-S124-letter-pa` — lessons that put 38 glyphs on
   the page at once. Moving a grid earlier is an alphabet block. Splitting the
   two grids into single-letter segments is the next tranche's work, and it
   would also drop the track's four remaining glyph spikes.

Three smaller items from the same tranche, all recorded rather than fixed:

- **The Kannada digit lessons carry three letters they do not teach.**
  `KA-S140` (೧), `KA-S142` (೩) and `KA-S144` (೫) are the first teaching lessons
  to print ಒ, ೂ and ಐ, because those vowels appear in no word before chapter 7
  and the ladder has no lesson for them. `neverTaughtGlyphs` is 0 partly on
  their account. Three one-glyph vowel lessons placed after chapter 7 would
  make the credit honest.
- **Chapter payoff representativeness fell** from 3 payoff surprises to 9.
  Kannada chapters 1–5 are schema-v1 with empty `assesses`, and they now hold
  script lessons whose atoms no payoff claims. Migrating those five chapters to
  schema v2 is the fix.
- **The counting chapter is staged A1 in five tracks, and that is where the
  pre-A1 vocabulary target keeps stalling.** Kannada chapter 7's ten new
  headwords landed at A1, leaving pre-A1 vocabulary at 152/300 — unchanged.
  Tamil, Telugu, Malayalam and Hindi all stage their counting node `A1` too;
  Sanskrit alone stages it `pre-A1`. Either the four are mislabelled or
  Sanskrit is, and the pre-A1 push needs the answer before it schedules more
  vocabulary: counting to ten is pre-A1 material in every published inventory,
  and 200-odd headwords are currently parked one level above the target they
  are meant to be filling. This is a cross-track decision, so it is asked here
  rather than answered inside one track.
