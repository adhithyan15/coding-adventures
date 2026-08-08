# Gujarati

A track of the [Human Languages](../README.md) curriculum, built the same way
as: one word per lesson, taken apart and traced to its root; the pieces taught
before the whole; the script taught inside the words that need it; and a book
you can read straight through.

## What's different about the Gujarati track

- **The "headless" script, taught inline.** Gujarati (*gujarātī*) is
  **Indo-Aryan** — a granddaughter of Sanskrit, sister to Hindi and Marathi —
  but its script dropped the **top line** (*shirorekhā*) that Devanagari,
  Bengali, and Gurmukhi hang their letters from. A vendored Noto Sans Gujarati
  font renders it; each word lesson introduces the letters it needs, and a
  reader who already reads Gujarati simply skims. No gated reading course.
- **Three genders.** Where Hindi kept two, Gujarati (like Marathi) keeps
  **three** — masculine / feminine / **neuter** — visible right away in the *-o
  / -ī / -ũ* adjective endings (*sāro / sārī / sārũ*, "good").
- **A copula all its own.** Gujarati's "is" is **chhe** — not Hindi's *hai* nor
  Sanskrit's *asti* — one of the quickest tells that a sentence is Gujarati.
- **The trade-language layer.** Gujaratis, a great seafaring merchant people,
  wove **Perso-Arabic** (and later Portuguese) words into everyday speech —
  even "how are you" is answered with a Persian loan (*majā* ← *maza*).
- The language of **Gandhi**, grounded throughout against English + Sanskrit +
  the other Indo-Aryan tracks.

## Progress

- **Chapter 1 — Greetings** ([`lessons/GU-C01-*`](./lessons/)): namaste, ābhār,
  hā/nā, sārũ, āvjo ("come again"), practice.
- **Chapter 2 — Introducing Yourself** ([`lessons/GU-C02-*`](./lessons/)): nām,
  mārũ, chhe, "my name is…", tũ / tame, shũ, "what's your name?", ānand.
- **Chapter 3 — How Are You** ([`lessons/GU-C03-*`](./lessons/)): kem, "tame
  kem chho?", hũ, majā, vāndho nahī.
- **Chapter 4 — Farewells** ([`lessons/GU-C04-*`](./lessons/)): pāchhā,
  maḷīshũ, "pāchhā maḷīshũ", kāle.
- **Chapter 5 — The First Verbs** ([`lessons/GU-C05-*`](./lessons/)): bolvũ,
  "hũ gujarātī bolũ chhũ", rahevũ, kām karvũ.
- **Chapter 6 — Numbers 1–5** ([`lessons/GU-C06-*`](./lessons/)): a short
  counting lesson followed by a prerequisite-ordered history of why *be*
  continues Sanskrit *dvé* and why *traṇ* regained an *r* after Prakrit lost
  it.
- **Chapter 7 — Six Verbs at the Core** ([`lessons/GU-C07-*`](./lessons/)):
  hovũ, javũ, āvvũ, khāvũ, jovũ, jāṇvũ — the first Gujarati chapter on the
  shared spine's verb node, held together by the fact that the infinitive
  ending **-વું** is Gujarati's **neuter**, so every verb is named in the third
  gender. Every lesson is listenable end to end.
- **Chapter 8 — The Mind and the Page** ([`lessons/GU-C08-*`](./lessons/)):
  vichārvũ, samajvũ, vā̃chvũ, lakhvũ — think, understand, read, write. Held
  together by what each root was doing *before* it named a mental act: ranging
  about, waking up, speaking, scratching. Adds the medial *anusvāra* (the nasal
  dot inside **વાંચવું**, not at the end of it) and the Prakrit
  assimilate-then-simplify path *dhy* → *jjh* → *j*.
- **Chapter 9 — Taking, Asking, Helping, Liking** ([`lessons/GU-C09-*`](./lessons/)):
  levũ, pūchhvũ, madad karvī, gamvũ — take, ask, help, like. Two Gujarati facts
  get their own blocks here: **મદદ કરવી** ends in the *feminine* **-ી** because
  *madad* is a feminine noun and the infinitive agrees with it (against neuter
  *kām karvũ*), and **મને ગુજરાતી ગમે છે** has no room for *hũ* at all — *gamvũ*
  descends from *gamyate*, the **passive** of *gam-* "to go", so it was born
  without a doer.

Chapters 1–9 are in the book.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## What each chapter lets you do

[`chapters.json`](./chapters.json) is the HL05 capability ledger: per chapter, one
first-person can-do sentence and the lesson that pays it off.

- **Chapter 6** — *"I can count from one to five in headless Gujarati script and
  explain why બે starts with b where its neighbours start with d, and why ત્રણ has
  an r that Hindi tīn does not."* Payoff:
  [`GU-C06-number-histories`](./lessons/GU-C06-number-histories.md), a task —
  take **બે** back to feminine/neuter *dvé* via *dv → bb → b*, and **ત્રણ**'s *r*
  back to a learned restoration from Sanskrit.
- **Chapter 7** — *"I can name six everyday Gujarati verbs by the neuter -વું
  ending they all share, strip that ending to reach the stem, and say what I do
  with stem, person-ending, and the copula છે."* Payoff:
  [`GU-C07-jaanvun`](./lessons/GU-C07-jaanvun.md), a production task — run all
  six stems, hold the retroflex **ણ**, and build *hũ jāṇũ chhũ*.
- **Chapter 8** — *"I can say that I think, understand, read and write in
  Gujarati, hear and write the nasal that sits in the middle of વાંચવું, and name
  what each of the four roots was doing before it meant a mental act."* Payoff:
  [`GU-C08-lakhvun`](./lessons/GU-C08-lakhvun.md), a production task — say *hũ
  mārũ nām lakhũ chhũ* and run the four roots back. Assesses **10 of the
  chapter's 10** atoms.
- **Chapter 9** — *"I can take, ask, help and say what I like in Gujarati,
  explain why મદદ કરવી ends in the feminine -ી while કામ કરવું ends in the neuter,
  and say why મને ગુજરાતી ગમે છે leaves no room for me as its subject."* Payoff:
  [`GU-C09-gamvun`](./lessons/GU-C09-gamvun.md), a production task — say *mane
  gujarātī game chhe* against *hũ gujarātī bolũ chhũ*. Assesses **10 of the
  chapter's 10** atoms.

Chapters 8 and 9 are each four lessons and ten atoms, against the
`maxNewAtomsPerChapter` budget of twelve. Splitting the eight verbs across two
chapters rather than one was deliberate: a single eight-lesson chapter would
have run to roughly twenty atoms, and chapter 7 is already over budget at
sixteen.

Both payoffs also reach back past their own chapter. Between them the new
lessons rescue every one of the twelve Gujarati atoms that had been introduced
and never practised again, at the cost of three new ones in the final lesson:
the track moves from **12 orphans of 24 atoms** to **3 of 44**.

Chapters 1–5 are **not in the ledger yet**, and that gap is deliberate. They are
still schema v1, so their lessons declare no knowledge atoms and no payoff there
could honestly claim to assess anything. A placeholder would hide debt the HL05
gap report is meant to surface; the entries land as those chapters migrate.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Gujarati font
(`../../_fonts/NotoSansGujarati-Static.ttf`). `latexmk -xelatex book.tex`.
Generated Gujarati runs use that font while section bookmarks use the lessons'
Latin romanization.
The nine-chapter build is warning-clean — 54 pages, zero `Missing character`,
zero undefined references — and its PDF outline preserves readable Gujarati in
the handwritten chapters alongside generated bookmark-safe romanization.

Two font rules the Gujarati track has to obey, both verified by compiling rather
than assumed. **Punctuation must stay outside the `\gu{}` span**: Noto Sans
Gujarati carries no basic-Latin comma, hyphen or question mark, so a span
containing one emits `Missing character`. The book generator groups only
characters whose Unicode script is Gujarati, so it gets this right by
construction — but a *space* is fine, which is why `\gu{મદદ કરવી}` is safe.
**Non-Gujarati scripts in lesson prose are not covered by `\gu{}` at all** and
fall through to Latin Modern Roman, which has no Arabic or Devanagari: source
links to Arabic *madad* and Sanskrit *gamyate* are titled in romanization for
exactly this reason. Latin Modern Roman also lacks `ʰ`, `ʷ`, `ḱ` and the
subscript digits, so PIE roots are written in the plain Pokorny-style ASCII the
rest of the corpus uses (`*kwel-`, `*bheudh-`, `*wekw-`, `*prek-`, `*gwem-`).
The nasalised vowels Gujarati romanization needs — `ũ`, `ĩ`, `ã` — are declared
in [`book/preamble.tex`](./book/preamble.tex) via `newunicodechar`, and `ā̃`
(as in *pā̃ch*, *vā̃chvũ*) renders without one.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `GU-C01-namaste`); order lives in the book and
`session-map.md`.
