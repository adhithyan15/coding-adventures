## HL-C201 — Bengali closure is now a glyph-inventory problem, not an ordering one

HL-C194 is closed. Interleaving the forty-five script lessons through the content
chapters, plus declaring the twenty-five missing `romanization` fields, moved
Bengali's script-closure violations **65 → 41** and
`headwordsWithoutRomanization` **25 → 0** without adding a lesson.

What is left is a different problem, and the ceiling on it was measured rather
than guessed. Replaying `measureScriptClosure` over the current corpus with every
script lesson hypothetically pre-taught — the best any reordering could ever
do — still reports **36** violations. The track sits at 41, so **reordering is
worth at most five more and the remaining thirty-six are shapes nobody teaches at
all.**

Those five, named, because three of them are not really reachable:

| lesson | chapter | wants |
|---|---|---|
| `BN-C02-amar` | 3 | ি |
| `BN-C02-tumi-apni` | 3 | ই |
| `BN-C02-tomar-naam-ki` | 3 | প |
| `BN-C09-neowa` | 13 | ী |
| `BN-C14-shobuj` | 20 | ী |

The three chapter-3 lessons want letters taught in chapter 4, and moving those
letters into chapter 2 would teach them before নাম and তুমি — the words they are
spent on — which is exactly the gloss-first rule this tranche was told to keep.
The two wanting **ী** are reachable and are the same finding as the last note
below.

The twenty-one never-taught shapes still printed load-bearing, by how many
Bengali lessons show each one:

**য** 15 · **়** 14 · **ও** 9 · **গ** 6 · **ৃ** 6 · **ছ** 5 · **ঞ** 4 ·
**ড** 4 · **ষ** 4 · **ট** 3 · **ূ** 3 · **ঙ** 2 · **ঝ** 2 · **থ** 2 ·
**ফ** 2 · **শ** 2 · **ঁ** 1 · **ঃ** 1 · **অ** 1 · **উ** 1 · **ঠ** 1

(`neverTaughtGlyphs` is 22; **ং** is the twenty-second and now appears only where
the exposure rule exempts it.)

Each of the heaviest already has a word glossed earlier in the book waiting to
pay it off, so none of them needs new vocabulary invented:

- **য** and **ঁ** — হ্যাঁ, chapter 1
- **ও** — হওয়া, chapter 11
- **ছ** — আছি, chapter 5
- **ড**, **়** and **ৃ** — কাপড়, chapter 22, and হৃদয়, chapter 18
- **গ** — লাগা, chapter 13

Teaching those eight would take `neverTaughtGlyphs` 22 → 14 and, because the
strand is now interleaved, each one can land *before* the lessons that show it
rather than after — which is the difference this tranche bought and the reason
the next one is finally worth doing.

Two structural notes for whoever picks this up:

- **The strand's placement is pinned by gloss-first, not by choice.** A `*-read`
  lesson may only read a word already met romanized, so its chapter is decided by
  its content chapter's position. New letter lessons are free to land anywhere;
  new reading lessons are not, and a new letter still needs a word to be spent on
  before the chapter after it can rely on the letter.
- **`ী` is the one taught glyph still arriving late** (chapter 21), because the
  only word in the book carrying it is নীল, a chapter 20 colour. A pre-A1 word
  with a long *i* met before chapter 13 would let the sign move up beside its
  short twin in chapter 4, where it belongs, and would close the two remaining
  reachable violations above.
