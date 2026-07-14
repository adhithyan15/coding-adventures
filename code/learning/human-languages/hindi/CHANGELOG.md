# Changelog

## Chapter 2 — Introducing Yourself

- New chapter around the introduction dialogue (*merā nām … hai / āpkā nām kyā
  hai? / …khushī huī*), atom-first, one word per lesson (`lessons/HI-C02-*`,
  `book/chapters/ch02-introductions.tex`), Devanagari taught inline. Every atom
  traced with its cross-family cousins (no glossing):
  - **नाम** nām ("name") ← Sanskrit *nāman* → English **name**, Latin *nōmen* →
    **noun**.
  - **मेरा** merā ("my") ← root *ma-* → English **me/my/mine**; agrees with the
    noun.
  - **है** hai ("is") ← Sanskrit *asti* → English **is**, German *ist*, Latin
    *est*, Spanish *es*.
  - **मेरा नाम … है** — **"my name is…"**; subject–object–verb order.
  - **आप / तुम** āp/tum — the three-level "you" (āp/tum/tū); *tum* ← *tū* →
    archaic **thou**.
  - **क्या** kyā ("what") ← stem *ka-* → English **what/who**.
  - **आपका नाम क्या है?** — **"what's your name?"** (verb still last).
  - **ख़ुशी** khushī — "pleased to meet you"; *khushī* ← **Persian**, Hindi's
    second vocabulary.
  - **practice** — the whole dialogue.
- Book compiles clean with XeLaTeX.

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
