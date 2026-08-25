### Fixed — four wrong etymologies and one missing book target

- *"`*kʷ-` **hardened** to `hw-`"* — Grimm's Law turns a stop into a fricative. That
  is softening, and it was the one sentence the lesson was built on.
- *"`que` ← Latin `quid` / **`quod`**"* — `que` inherits `quid` and *usurps* `quod`'s
  roles without taking its form. The lesson's own recall answer already said `quid`.
- *"Knock the final consonant off each"* — false for `-ābās` → `-abas`, in the table
  three lines below. Both this and the contested intervocalic-`b`-loss claim were cut
  with the rest of the forms material.
- *"Two languages, **independently**"* — English *recount* and *account* are the same
  word Spanish inherited, via Old French. Only *tell* is a genuine parallel, and the
  paragraph now says so.

**The chapter reached the app but not the book.** `core/book-generation.json` is a
hand-maintained target list; narration and modality are corpus-driven and picked
chapter 38 up automatically, so `check:books` passed while the book had no chapter.
Target added and `book.tex` wired — `bookChapters` 442 → 443.

