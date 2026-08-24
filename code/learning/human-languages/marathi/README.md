# Marathi

The twelfth track of the [Human Languages](../README.md) curriculum, built the
same way as: one word per lesson, taken apart and traced to its root; the
pieces taught before the whole; and a book you can read straight through.

## What's different about the Marathi track

- **Devanagari, taught inline — but Marathi, not Hindi.** Marathi is written in
  the same script as Hindi/Sanskrit and reuses the vendored Devanagari font,
  but it is its own **Indo-Aryan** language (of Maharashtra). The recurring
  thread is what makes it distinct: it prefers **namaskār** as the greeting,
  keeps **three** genders (Hindi has two), marks **gender on the verb** even in
  the present (*yeto* m. / *yete* f.), and has an **extra letter ळ** (retroflex
  *ḷ*) shared with the Dravidian south. Every word lesson teaches the letters
  its own word needs, inline, under *"The letters in this word"* — the canonical
  heading, which the tooling treats as a **detachable** section, so a hands-free
  renderer may skip it and the lesson still counts as drivable. (Chapter 8 uses
  *"Sounds you'll need"* instead; see the CHANGELOG for why that is a workaround
  rather than a convention.) A reader who knows Devanagari skims either.
- **Grounded against English + Sanskrit**, with the wider Indo-European family
  drawn in where it reaches (*nāhī* ← PIE *\*ne*, English *no*).

## Assessment destination

The complete book targets the project-defined [Coding Adventures Marathi
Assessment](assessment-spec.md) from pre-A1 through C2. Its machine-readable
[contract](assessment.json) requires separate passes in reading, listening,
writing, and speaking, the full gentle writing ladder, and two timed mocks at
every rung. This names the destination; it does not claim that the current book
is exam-ready or that the future certificate is externally accredited. Task
inventories, mocks, rubrics, answer keys, calibration, and book-only human
validation remain explicit backlog.

## Progress

- **Chapter 1 — Namaskar, piece by piece**
  ([`lessons/MR-C01-*`](./lessons/), [`lessons/MR-W01-*`](./lessons/)):
  hear **namaskār**, then trace, copy, recall, and write the pieces needed for
  **हो** before reading **नमस्कार** independently. The chapter reaches the
  12-atom ceiling exactly; no sign is bundled and no whole form comes first.
- **Chapter 2 — Courtesy, responses, and leave-taking**
  ([`lessons/MR-C01-*`](./lessons/)): add *dhanyavād*, *ho*, *baraṁ*, *nāhī*,
  and *yeto/yete* only after the opening script runway, then close with a mixed
  listening, speaking, and independent **हो** writing checkpoint.
- **Chapter 3 — Introducing Yourself** ([`lessons/MR-C02-*`](./lessons/)): nāv,
  mājhaṁ, āhe, "my name is…", tū/tumhī, kāy, "what's your name?", ānand.
- **Chapter 4 — How Are You** ([`lessons/MR-C03-*`](./lessons/)): kasā, "tumhī
  kase āhāt?", mī, "mī barā āhe", kāhī harkat nāhī.
- **Chapter 5 — Farewells** ([`lessons/MR-C04-*`](./lessons/)): punhā, bheṭū,
  "punhā bheṭū", "udyā bheṭū", kāḷjī ghyā.
- **Chapter 6 — The First Verbs** ([`lessons/MR-C05-*`](./lessons/)): bolṇe,
  "mī marāṭhī bolto", rāhṇe, kām karṇe.
- **Chapter 7 — Numbers 1–5** ([`lessons/MR-C06-*`](./lessons/)): a short
  counting lesson followed by a prerequisite-ordered etymology lesson on why
  *don* copied *tīn*, why Hindi retains *pāṁch*'s nasal, and why written *chār*
  sounds nearer *tsār* in Marathi.
- **Chapter 8 — The Core Verbs** ([`lessons/MR-C07-*`](./lessons/)): asṇe,
  jāṇe, yeṇe, khāṇe, pāhṇe, māhīt asṇe — one idea each. The copula **आहे** and
  the verb-last rule; the present ending that declares your gender; *going* and
  *coming* as one Sanskrit root; the *-णे* infinitive as a **neuter** noun (the
  third gender Hindi lost); *paś-* as Indo-European *\*spek-*, cousin of
  *spectacle* and *spy*; and knowledge in the dative — *malā māhīt āhe*, "to me
  known is."
- **Chapter 9 — The Mind and the Page** ([`lessons/MR-C08-*`](./lessons/)):
  vichār karṇe, samajṇe, vāchṇe, lihiṇe. Four roots that meant something
  physical first — *car-* "to turn" (Indo-European *\*kʷelh₁-*, behind **wheel**
  and **cycle**), *budh-* "to wake" (which named the **Buddha**), *vac-* "to
  speak" (Latin *vōx*, English **voice**), and *likh-* "to scratch," beside
  Latin *scrībere* and English *write*, which named writing the same way.
  **समजणे** takes the dative, so *malā marāṭhī samajte* stands beside the core
  verb chapter's *malā marāṭhī yete*.
- **Chapter 10 — Taking, Asking, Helping, Liking**
  ([`lessons/MR-C09-*`](./lessons/)): gheṇe, vichārṇe, madat karṇe, āvaḍṇe.
  **घेणे** is the verb hiding inside Chapter 5's *kāḷjī ghyā*, on Sanskrit
  **ग्रह्** — Vedic *grabh*, which is English **grab**. **विचारणे** is
  Chapter 9's **विचार** with an infinitive ending, so in Marathi thinking and
  asking are one word. **मदत** is Arabic *madad* through Persian, the same
  Deccan layer as **माहीत**. And **आवडणे** is native, with no securely
  established Sanskrit ancestor, where Hindi and Urdu use the Persian loan
  *pasand* — it makes *malā marāṭhī āvaḍte* the third sentence on one frame.

- **Chapter 11 — Water, Tea, Milk, and Bhakri** ([`lessons/MR-C10-*`](./lessons/)):
  pāṇī, chahā, dūdh, bhākarī — the track's first pre-A1 nouns, and the first
  "please" pattern it has ever built, **[word], कृपया**. **कृपया** is Sanskrit's
  own instrumental of **कृपा**, "kindness." **पाणी** and **दूध** are secure
  Sanskrit/PIE inheritances (*peh₃(i)-* "to drink," behind potion, poison,
  symposium; *dʰewgʰ-*, behind English **doughty**); **चहा** is Marathi's first
  loanword of the tranche, Mandarin *cha* by the overland route; **भाकरी**
  never touched Sanskrit at all and is not Hindi's word for bread — Hindi says
  **रोटी**. The four words carry all three Marathi genders between them.
- **Chapter 12 — Friend and Family** ([`lessons/MR-C11-*`](./lessons/)): mitra,
  kuṭumb, bhāū, bahīṇ. **मित्र** and **कुटुंब** are tatsamas, taken up whole
  from Sanskrit; **भाऊ** and **बहीण** are tadbhavas, worn down by Prakrit sound
  change. **भाऊ** is a secure cousin of English **brother**; **बहीण**,
  deliberately, is not a cousin of **sister** at all — a different Sanskrit
  root, traditionally tied to "sharing." **कुटुंब** is neuter in Marathi and
  masculine in Hindi, from the identical borrowed word.
- **Chapter 13 — Eye, Ear, Mouth, and Nose** ([`lessons/MR-C12-*`](./lessons/)):
  ḍoḷā, kān, tõḍ, nāk. **डोळा** formally teaches the retroflex **ळ** first
  heard, unlabeled, in Chapter 5's *kāḷjī* — and its own root is Sanskrit
  **दोल**, "a swing," a replacement for the old Indo-Aryan eye-word Hindi's
  **आँख** still carries. **कान**'s root is honestly disputed among scholars;
  its *rṇ* → *ṇṇ* → *ṇ* path is the same cluster-simplifying habit Chapter 7
  traced in **दोन**. **तोंड** keeps a formal doublet, **मुख**. **नाक** alone
  reaches English **nose** without qualification.
- **Chapter 14 — Heart** ([`lessons/MR-C13-*`](./lessons/)): hṛdaya, one lesson,
  the surest cognate this book has taught — English **heart**, Latin
  **cor**/**cordis** (**cordial**), Greek **kardia** (**cardiac**), all one PIE
  root, *ḱērd-*, with no metaphorical drift at all, set against **वाचणे**'s and
  **समजणे**'s (Chapter 9) and **डोळा**'s (Chapter 13) drifted meanings.
- **Chapter 15 — Family Memory at Real Distance**
  ([`lessons/MR-R14-*`](./lessons/)): twelve two-to-three-minute retrieval
  sessions revisit family, body, food, verbs, numbers, and the opening core.
  In particular, **मित्र** and **कुटुंब** now receive genuine R2 and R3 recalls
  after their immediate Chapter 12 reviews rather than being credited through
  metadata or a learner-invisible order.

Chapters 1–15 are in the book. Chapters 1–2 and 7–15 are schema v2. Chapter 8 is `voice`
at full modality; Chapters 9–14 derive as `sight` because each lesson carries
a *"The letters in this word"* section — but that section is **detachable**, so
every lesson is `coreVoice` and every chapter is fully drivable by ear.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## What each chapter lets you do

[`chapters.json`](./chapters.json) is the HL05 capability ledger: per chapter, one
first-person can-do sentence and the lesson that pays it off.

- **Chapter 7** — *"I can count from one to five in Marathi, say चार as tsār
  rather than chār, and tell which of Marathi's differences from Hindi is an
  innovation and which is Hindi holding on to something older."* Payoff:
  [`MR-C06-number-differences`](./lessons/MR-C06-number-differences.md), a task —
  **दोन**'s borrowed *-n*, **पाच**'s missing nasal, and the *ts* hiding behind an
  unchanged spelling.
- **Chapter 8** — *"I can use six everyday Marathi verbs, pick the present-tense
  ending that agrees with my subject's gender, end every sentence on the verb,
  and say that I know something the way Marathi says it."* Payoff:
  [`MR-C07-mahit-asne`](./lessons/MR-C07-mahit-asne.md), a task — *malā māhīt
  āhe* and *malā marāṭhī yete*, which put the knower in the dative, agree the
  verb with **मराठी** rather than the speaker, and close on **आहे**. It assesses
  7 of the chapter's 12 introduced atoms (0.58, above the 0.5 floor).
- **Chapter 9** — *"I can say that I think, understand, read and write in
  Marathi, put the understander in the dative where Marathi puts it, and name
  what each of the four roots meant before it meant a mental act."* Payoff:
  [`MR-C08-lihine`](./lessons/MR-C08-lihine.md), a production task — write your
  own name, run the four verbs and their four root meanings back, and pick out
  the one that refuses **मी**. It assesses **8 of the chapter's 8** introduced
  atoms (1.00).
- **Chapter 10** — *"I can take, ask, help and say what I like and what I love in
  Marathi, build a fresh verb by putting करणे behind a noun, and say why मला
  मराठी आवडते has no room for me as its subject."* Payoff:
  [`MR-C09-avadne`](./lessons/MR-C09-avadne.md), a production task — *malā
  marāṭhī āvaḍte* beside *mī prem karto*, and both beside *malā marāṭhī yete*
  and *malā marāṭhī samajte*, so one frame carries three meanings. It assesses
  **9 of the chapter's 9** introduced atoms (1.00).
- **Chapter 11** — *"I can politely ask for water, tea, milk, or bhakri in
  Marathi, name which of the four is a Chinese loan and which never touched
  Sanskrit at all, and say why all three of Marathi's genders already show up
  in one short chapter."* Payoff: [`MR-C10-bhakari`](./lessons/MR-C10-bhakari.md),
  a production task — all four requests with **कृपया**, sorted by origin and by
  gender. It assesses **10 of the chapter's 10** introduced atoms (1.00).
- **Chapter 12** — *"I can name a friend, a family, a brother, and a sister in
  Marathi, sort each word into a tatsama or a tadbhava, and say which sibling
  word is genuinely English's cousin and which is not."* Payoff:
  [`MR-C11-bahin`](./lessons/MR-C11-bahin.md), a production task — all four
  words sorted tatsama/tadbhava, **भाऊ** named as brother's secure cousin
  against **बहीण**'s unrelated root. It assesses **10 of the chapter's 10**
  introduced atoms (1.00).
- **Chapter 13** — *"I can name the eye, ear, mouth, and nose in Marathi, say
  the retroflex ळ formally for the first time, and tell which of the four
  carries the most secure English cognate and which replaced its own
  inherited word entirely."* Payoff: [`MR-C12-naak`](./lessons/MR-C12-naak.md),
  a production task — all four face words with gender, sorted by kind of
  history: replaced, disputed, twinned, secure. It assesses **10 of the
  chapter's 10** introduced atoms (1.00).
- **Chapter 14** — *"I can name the heart in Marathi, name three English words
  that share its root, and say what makes it different from every other word
  this book has taken apart: it never drifted at all."* Payoff:
  [`MR-C13-hrudaya`](./lessons/MR-C13-hrudaya.md), a production task — **heart,
  cordial, cardiac** named as cousins, set against **वाचणे**, **समजणे** and
  **डोळा**'s drifted meanings. It assesses **2 of the chapter's 2** introduced
  atoms (1.00).

Chapters 3–6 are **not in the ledger yet**, and that gap is deliberate. They are
still schema v1, so their lessons declare no knowledge atoms and no payoff there
could honestly claim to assess anything. A placeholder would hide debt the HL05
gap report is meant to surface; the entries land as those chapters migrate.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Devanagari font
(`../../_fonts/`) — the same font as the Hindi track. `latexmk -xelatex book.tex`.
The fifteen-chapter build is warning-clean — zero `Missing character`,
zero overfull or underfull boxes, zero package warnings — and its PDF outline
preserves readable Devanagari while generated non-Latin sections use
bookmark-safe romanization. (An early draft of Chapters 10 and 11 quoted
Avestan, Old Persian, Bengali, Kannada, and Gujarati text directly, plus a
Proto-Indo-Iranian reconstruction using the combining ring-below U+0325 in
Chapter 14 — none of it covered by the book's fonts. All were flattened to
plain-Latin romanizations before commit.)

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `MR-C01-namaskar`); order lives in the book and
`session-map.md`.
