# Changelog

## Reworked to inline-letters: one Chapter 1, script taught within the words

Replaced the standalone reading-course structure (Ch1 reading course + Ch2
greetings) with the inline-letters model the rest of the curriculum now uses —
per `HL00`'s updated rule and direct user feedback ("introduce the letters as
you introduce words that use them… I do not want people to sit through a reading
course before they start").

- **Merged into a single `Chapter 1 — Greetings`** (`lessons/AR-C01-*`,
  `book/chapters/ch01-greetings.tex`): salām → marḥaban → al- → as-salāmu
  ʿalaykum → ṣabāḥ al-khayr → masāʾ al-khayr → shukran → practice. Each word
  lesson now carries a *"The letters in this word"* section teaching only the
  new letters that word needs (RTL, connecting letters, dots-on-a-skeleton, the
  emphatic ṣ, ʿayn, hamza), so reading + meaning + root arrive together. The
  root engine (k-t-b) and the three script facts moved into the chapter intro.
- **Removed** the old `book/chapters/ch01-reading.tex` + `ch02-greetings.tex`
  and the `AR-C01-read-*` / `AR-C02-*` lesson files (their content folded into
  the new inline lessons).
- **Beginner-audience fixes** (HL00 Audience rule): the preface and appendix no
  longer say "you already read Arabic (rusty)" or assume the reader knows
  Spanish; the Spanish-loanword thread is now self-contained enrichment.
- Retitled the book's green callout box from "Sounds & script you'll need" to
  "The letters in this word." Removed HL00's note that Arabic was still to be
  reworked (now done). Book compiles clean with XeLaTeX.

## Reading course: Chapter 1 = learn to read, greetings → Chapter 2 (superseded)

Reworked after feedback that the first draft "dropped a bunch of words but
never taught how to actually read any of it" — a vocabulary list, not
reading, and a break from the atom-first playbook.

- **Chapter 1 is now an incremental reading course** (`lessons/AR-C01-read-*`):
  a few letters per lesson, each cashing out in a real, decodable word —
  ل+ا → **لا** ("no"), then م/س → **سلام**, ب/ر/ح → **مرحبا**, ص/خ/ي → **صباح
  الخير**, ش/ك → **شكرا**, ع/ء → **السلام عليكم**/**مساء الخير**, then a reading
  recap. ~15 letters, half the alphabet, each welded to a word. RTL, letter
  connection, dots-on-shared-skeletons, one-way letters — all taught inline as
  the words need them.
- **The greeting lessons moved to Chapter 2** (`lessons/AR-C02-*`) — same
  content (root system, salām family, al-/sun-moon letters, the greetings and
  replies), now that the learner can read the words. The book's chapters were
  restructured to match (ch01-reading + ch02-greetings; LaTeX auto-renumbers).
- `HL00` updated to codify: **for any non-Latin script, Chapter 1 is a reading
  course** (letters → words, incremental), not a gated alphabet chart and not
  word-first.

## Chapter 1 — Greetings (initial draft, superseded by the reading course)

- New Arabic track on the HL00 framework: one word per lesson, slug ids,
  atom-first, derivations shown, LaTeX book. First **right-to-left** track and
  first to use a **vendored font** (Noto Naskh Arabic, static instance, loaded
  by relative `Path=` so local and CI builds match).
- Chapter 1 (`lessons/AR-C01-*`), built around Arabic's own structure:
  - **the root system** — the three-consonant root engine (k-t-b → kitāb/
    kātib/maktab), Arabic's version of the curriculum's root obsession, made
    literal; plus the RTL/connecting-script refresher inline
  - **مرحبا** marḥaban (root r-ḥ-b, "there's room for you") · **سلام** salām
    (root s-l-m — salām/islām/muslim; Hebrew *shalom* the same Semitic root)
  - **ال** al- ("the"; the Al-Andalus loanword web — algebra/alcohol/azúcar;
    sun/moon-letter assimilation) · **السلام عليكم** as-salāmu ʿalaykum
  - **صباح** ṣabāḥ · **خير** khayr · **صباح الخير** ṣabāḥ al-khayr (reply
    ṣabāḥ an-nūr) · **مساء** masāʾ · **مساء الخير** masāʾ al-khayr
  - **شكرا** shukran (root sh-k-r; shākir/mashkūr reuse the kātib/maktūb
    patterns) · **practice**
- Grounds each word against English and Spanish; foregrounds Arabic's shadow
  over Spanish (al-, azúcar). Book compiles clean with XeLaTeX (13 pages).
- Added a shared `_fonts/` dir (vendored static Noto fonts + OFL license) for
  this and the later Indic-script tracks.
