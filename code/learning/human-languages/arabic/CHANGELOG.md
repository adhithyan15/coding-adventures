# Changelog

## Chapter 1 — Greetings (track bootstrapped)

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
