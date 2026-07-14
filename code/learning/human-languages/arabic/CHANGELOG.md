# Changelog

## Reading course: Chapter 1 = learn to read, greetings → Chapter 2

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
