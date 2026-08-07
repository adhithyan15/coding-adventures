# Bengali

A track of the [Human Languages](../README.md) curriculum, built the same way
as: one word per lesson, taken apart and traced to its root; the pieces taught
before the whole; and a book you can read straight through.

## What's different about the Bengali track

- **Bengali script, taught inline.** Bengali (*Bāṅlā*) is **Indo-Aryan** (like
  Hindi, Marathi, Punjabi), written in its own script — sister to Devanagari,
  sharing the hanging top line but with rounder shapes. A vendored Noto Sans
  Bengali font renders it; each word lesson has a *"The letters in this word"*
  section, and a reader who already reads Bengali simply skims it. No reading
  course.
- **One sound-shift as the spine.** The recurring thread is Bengali's
  **fingerprint**: the inherent vowel is **ô** (not "a"), *s* tilts to **sh**,
  and *v* collapses to **b** — so Sanskrit *namaskāra* is spoken **nômoshkar**
  and *dhanyavāda* becomes **dhônyobad**. Learn the shift and every
  Sanskrit-derived word becomes legible.
- **Grounded against English + Sanskrit**, with the wider Indo-European family
  drawn in where it reaches (*nā* ← PIE *ne*, English *no*).

## Progress

- **Chapter 1 — Greetings** ([`lessons/BN-C01-*`](./lessons/)): nômoshkar,
  dhônyobad, hyã/nā, āchchhā, āshi (the "I'll come again" goodbye), practice.
  In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/BN-C02-*`](./lessons/)): nām,
  āmār, **āmār nām …** ("my name is," zero copula), tumi/āpni (+ tui), ki,
  **tomār nām ki?**, ālāp kore bhālo lāglo, practice. The zero copula; no
  gender. In the book.
- **Chapter 3 — How Are You** ([`lessons/BN-C03-*`](./lessons/)): kemon, **tumi
  kemon āchho?**, āmi (← *asmi* → *am*), bhālo, kono bæpār nā, practice. The
  verb *āchhā* returns for state. In the book.
- **Chapter 4 — Farewells** ([`lessons/BN-C04-*`](./lessons/)): ābār, dækhā
  hôbe, **ābār dækhā hôbe**, kāl dækhā hôbe, practice. The impersonal future.
  In the book.
- **Chapter 5 — First Verbs** ([`lessons/BN-C05-*`](./lessons/)): bôlā, **āmi
  bānglā bôli**, thākā (← *sthā* → *stand/stay*), kāj kôrā, practice. The verb
  never changes for gender. In the book.
- **Chapter 6 — Numbers 1–5** ([`lessons/BN-C06-*`](./lessons/)): *ek, dui,
  tin, chār, pā̃ch*, the chandrabindu, and the conservative vowel in *dui*. In
  the book.
- **Chapter 7 — The Core Verbs** ([`lessons/BN-C07-*`](./lessons/)): hôwā (two
  be-verbs, and আছ- has no future), jāwā (respect rides on the ending), āsā (no
  grammatical gender, anywhere), khāwā (Bengali eats its drinks), dækhā (vowel
  harmony under a fixed spelling), jānā (জানা for facts, চেনা for people). The
  first six **canonical** verb concepts the track has ever realized. In the
  book, and fully drivable.
- **Chapter 8 — The Mind and the Page** ([`lessons/BN-C08-*`](./lessons/)):
  bhābā (thinking is হওয়া's root with the causative gear engaged), bojhā
  (√budh → the **Buddha**, and vowel harmony that this time moves the spelling),
  pôṛā (the flapped **ড়**, and the two Sanskrit verbs that both landed on পড়া),
  lekhā (writing is scratching in three unrelated families — and every **-া**
  form is a noun, which is what দেখা হবে was doing all along). In the book.
- **Chapter 9 — Taking, Asking, Helping, Liking**
  ([`lessons/BN-C09-*`](./lessons/)): neowā (the verb that ends a compound —
  নিয়ে আসা "bring," লিখে নেওয়া "write it down"), jijñāsā kôrā (the desiderative
  of জানা's root, and **noun + করা**, the pattern that turns any noun into a
  verb), sāhājjo kôrā (*saha* "together" + √i "to go," and the word-final
  inherent **o** Chapter 6 could not demonstrate), bhālo lāgā (**the liker is
  not the subject** — *āmār bhālo lāge*, "good sticks to me" — set against
  ভালোবাসা, where you are). In the book.

With Chapters 8 and 9 the track realizes **14 of the 40 core verb concepts**, up
from six, and joins Spanish, Latin and Portuguese on all eight of
`VERB-THINK`, `VERB-UNDERSTAND`, `VERB-READ`, `VERB-WRITE`, `VERB-TAKE`,
`VERB-ASK`, `VERB-HELP` and `VERB-LIKE-LOVE` — each of which was a three-track
concept before and is now a four-track one.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## What each chapter lets you do

[`chapters.json`](./chapters.json) is the HL05 capability ledger: per chapter, one
first-person can-do sentence and the lesson that pays it off.

- **Chapter 6** — *"I can count from one to five in Bengali script and say what
  দুই kept that Hindi and Marathi flattened away."* Payoff:
  [`BN-C06-numbers-1-5`](./lessons/BN-C06-numbers-1-5.md), a spoken production —
  count the five, name the chandrabindu on **পাঁচ**, place *dui* against *do* and
  *don*.
- **Chapter 7** — *"I can say what I do with six everyday Bengali verbs, pick
  the ending that matches তুই, তুমি or আপনি, and say why a Bengali verb never
  changes for gender and why আছে has to hand the future to হবে."* Payoff:
  [`BN-C07-jana`](./lessons/BN-C07-jana.md), a spoken production covering 8 of
  the chapter's 12 introduced atoms.
- **Chapter 8** — *"I can say that I think, understand, read and write in
  Bengali, hear the stem vowel rise whenever the -i ending arrives, and read the
  flapped ড় that Sanskrit never needed."* Payoff:
  [`BN-C08-lekha`](./lessons/BN-C08-lekha.md), a spoken production covering
  **8 of the chapter's 8** introduced atoms.
- **Chapter 9** — *"I can take, ask, help and say what I like and whom I love in
  Bengali, build a fresh verb out of any noun by putting করা behind it, and say
  why liking happens to me while loving is something I do."* Payoff:
  [`BN-C09-bhalo-laga`](./lessons/BN-C09-bhalo-laga.md), a spoken production
  covering 6 of the chapter's 9 introduced atoms (0.67, above the 0.5 floor).
  Two of the remaining three are retrieved by the very next lesson after the one
  that taught them; the third — the doubled **য্য** of সাহায্য — genuinely is
  not, and `chapters.json` records that rather than claiming otherwise.

Chapters 1–5 are **not in the ledger yet**, and that gap is deliberate. They are
still schema v1, so their lessons declare no knowledge atoms and no payoff there
could honestly claim to assess anything. A placeholder would hide debt the HL05
gap report is meant to surface; the entries land as those chapters migrate.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Bengali font
(`../../_fonts/NotoSansBengali-Static.ttf`). `latexmk -xelatex book.tex`.
Chapters 6 through 9 are generated from the canonical lessons; their
Bengali-script runs use that font while their section bookmarks use authored
romanization.

The forced nine-chapter build is warning-free — 54 pages, zero
`Missing character`, zero over/underfull boxes, and the conjuncts Chapters 8 and
9 add (**ড়**, **জ্ঞ**, **য্য**, **দ্বার**) all render from the vendored font
with no preamble change: main-font punctuation,
chapter-qualified recap anchors, bookmark-safe Bengali, natural page bottoms,
explicit static-font shapes, and a breakable long title keep the downloadable
PDF and its outline clean. Two details serve the generated chapters: the
`grammarlens` box takes an optional title, because the generator passes each
lesson's own "Grammar Lens: …" heading through, and `ǵ`/`ḱ` are composed with
`\newunicodechar` since the main font has no precomposed glyph for the
reconstructed PIE palatals.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `BN-C01-nomoshkar`); order lives in the book and
`session-map.md`.
