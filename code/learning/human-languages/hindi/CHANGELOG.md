# Changelog

## Chapter 1 — Greetings (Devanagari taught inline)

- New Hindi track on the HL00 framework: one word per lesson, slug ids,
  atom-first, derivations shown, LaTeX book. Uses the **vendored** Noto Sans
  Devanagari font (static instance, loaded by relative `Path=` so local and CI
  builds match).
- **No reading course.** Per `HL00`'s inline-letters rule, Devanagari is taught
  *inside* each word lesson: a *"The letters in this word"* section introduces
  exactly the letters that word needs, so you learn to read the word and learn
  its meaning together. A Devanagari reference page is included in the book as a
  lookup, explicitly not a gated pre-course.
- Chapter 1 (`lessons/HI-C01-*`), built around greetings and Hindi's double
  inheritance:
  - **नमस्ते** namaste (Sanskrit root *nam*, "to bow"; *namaḥ* + *te* = "I bow
    to you") — teaches inherent *a*, the *e*-mātrā े, halant, and the स्त
    conjunct.
  - **नमस्कार** namaskār (*namaḥ* + *kāra*, "the making of a bow"; root *kṛ*) —
    adds क र and the long-*ā* mātrā ा.
  - **धन्यवाद** dhanyavād (*dhanya* "worthy" + *vāda* "a saying"; root *vad*) —
    the formal, Sanskritic "thank you"; adds ध य व द and the न्य conjunct.
  - **शुक्रिया** shukriyā (Persian ← **Arabic** *shukr*, root **sh-k-r** — the
    same word as Arabic *shukran*) — the everyday "thanks"; introduces Hindi's
    two vocabularies, and the *i*-mātrā ि + क्र conjunct.
  - **अलविदा** alvidā (Persian ← **Arabic** *al-widāʿ*, "the farewell,"
    carrying the article **al-**) — the independent vowel अ and ल.
  - **practice** — recap; the two heritages held side by side.
- Grounds each word against English and Arabic; foregrounds the Sanskrit vs.
  Perso-Arabic split as the key to Hindi. Book compiles clean with XeLaTeX.
