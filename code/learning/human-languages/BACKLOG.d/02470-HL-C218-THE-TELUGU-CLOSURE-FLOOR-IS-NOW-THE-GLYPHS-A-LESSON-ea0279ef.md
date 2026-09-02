## HL-C218 — the Telugu closure floor is now the glyphs a lesson meets for the first time

HL-C217 said the remaining work was "start the ladder earlier, not teach more
letters." That was right. Moving the first script lesson from **chapter 6 to
chapter 1** — and reordering the chain so the vowel signs come first — took
Telugu's script-closure violations from **22 to 6** and its never-taught glyphs
from **13 to 0**, without adding a single lesson to the end of the book.

What is left is a different shape of debt, and it is worth stating precisely
because it is now measurable rather than estimated. The six survivors are:

| lesson | untaught | where that glyph FIRST appears in the corpus |
|---|---|---|
| `TE-C01-sare` | ి | `TE-C01-sare` itself |
| `TE-C02-practice` | ణ | `TE-C02-practice` itself |
| `TE-C03-miiru-elaa-unnaaru` | డ | `TE-C03-miiru-elaa-unnaaru` itself |
| `TE-C04-vellu` | చ | `TE-C04-vellu` itself |
| `TE-C05-undu` | బ హ ై | హ and ై in `TE-C05-undu` itself |
| `TE-C09-kshaminchandi` | ఇ భ | both in `TE-C09-kshaminchandi` itself |

**In every case the failing lesson is the corpus's own first sighting of the
glyph.** A script lesson can only cite words the reader has already met, so
there is no honest lesson that could be placed before any of these: the word
that would teach the letter does not exist yet. The floor is therefore not
"roughly 15 until the practice-lesson exposure rule is settled" — that estimate
was wrong, and the practice lessons are no longer the problem. `TE-C01-practice`
and `TE-C02-practice` both went clean, because chapter 1's three letter lessons
put their example words on the page before the recap asks for them.

The remaining six can only be moved by **vocabulary sequencing**: bring a word
containing ి, ణ, డ, చ, బ, హ, ై, ఇ or భ earlier than the lesson that currently
introduces it. Four of the nine are single incidental mentions — a cousin-word
spelled in Telugu script (`సరి` in `TE-C01-sare`), a proper noun in a practice
table (`అరుణ్` in `TE-C02-practice`) — and those are the cheapest to reconsider.

Two smaller findings from the same tranche:

- **A script lesson teaches every glyph in its body, per `script-closure.ts`'s
  own stated approximation.** Moving a letter lesson earlier therefore moves
  credit for its example words too, and a large part of the 22 → 6 movement is
  that second-order effect rather than the letter itself. The module documents
  this as a deliberate lower bound; it is worth knowing that the bound is loose
  enough that the ladder's *position* changes the number more than its
  *content* does. A per-letter declaration in frontmatter would tighten it.
- **The `TE-SCRIPT-RECOG-*` atoms now return three times, not once.** Each
  script lesson recalls its predecessor (R1), the letter four back (R2) in its
  warm-up, and the letter twelve back (R3) in its wrap-up. Telugu's script-atom
  reinforcement misses fall **103 → 68** while the atom count rises 33 → 49
  (3.12 → 1.39 misses per atom), and mean lessons per script atom rises **1.94
  → 3.65**. R4 (80–250 lessons later) is still unserved for every script atom
  and is the obvious next step — it needs a periodic mixed-review lesson rather
  than another chain link, because a 30-step back-reference would reach past
  the end of the ladder for the letters taught late.

