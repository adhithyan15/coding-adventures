### Added — coverage assertions for the book manifest (HL-C50)

The Spanish book was printing chapter 38 twice and dropping chapter 40, and **every
check passed** while it did. That defect is fixed; these are the assertions that would
have caught it, plus an audit confirming no other track carries the same drift.

The gap was structural. `check:books` compares each **declared** target against what
the generator produces, so a manifest declaring the wrong chapter round-trips
perfectly. `titleDrift` stayed 0 because each file took its title from its own
(correct) target. Narration and modality read the corpus directly and never consult
the manifest. Nothing asked the coverage question: *do the declarations line up with
the corpus?*

Five assertions now do, in `book-cli.test.ts`'s existing manifest block:

1. **No chapter number declared twice in a track** — the drift's direct signature.
2. **Every filename agrees with its declared chapter** — `ch39-*.tex` must not be
   declared as chapter 38. The cheapest tripwire, and it would have fired the instant
   the drift was written.
3. **Every declaration stays inside its own track's directory** — a target writing
   into another track's folder passes every other check while silently adding a
   chapter to a book nobody edited.
4. **No two declarations write the same path** — the loser vanishes silently.
5. **Every ledgered chapter is `\input` into its book**, not merely present on disk.
   "Reaches a file" is the weaker claim, and the weaker claim is what let the original
   bug through in spirit.

**They run over `targets` and `handwritten` together**, which matters more than it
sounds: the manifest's `handwritten[]` array holds 105 of the 452 declarations, and the
identical drift there was invisible to every test in the package. Keeping the two
halves apart is what allowed that.

Each assertion was proven to fire before being trusted. Reintroducing the exact
Spanish drift trips three; the same drift in `handwritten[]` trips two; a target
escaping its track trips two; deleting a declaration trips two; and removing an
`\input` while leaving the file on disk trips the fifth alone.

**Audit result: no other track is affected.** Across all 22 — and across both arrays —
there are no duplicate declarations, no filename/chapter mismatches, no path
collisions, and no ledgered chapter missing from its book. Seven chapters have no
generation target (hindi 1–2, latin 1, persian 2, russian 2, tamil 1, urdu 2); all
seven are hand-authored, declared in `handwritten[]`, and `\input` into their books.

