# Russian

A track of the [Human Languages](../README.md) curriculum — the first on the
**Cyrillic** script —, built the same way as:
one word per lesson, taken apart and traced to its root; the script taught
inside the words that need it; grammar introduced only when a word needs it.

## What's different about the Russian track

- **Cyrillic taught inside the words — no reading course.** Each lesson has a
  *"The letters in this word"* section introducing exactly the letters that
  word needs. The track's spine is the **four false friends** — в=v, р=r, с=s,
  н=n — the letters that look Latin and lie; by the end of Chapter 1 they're
  fixed. Full per-letter decomposition lives in
  [`data/scripts/cyrillic.json`](../data/scripts/cyrillic.json); the track
  points at it via [`track.json`](./track.json).
- **English cousins through deep Indo-European roots.** Russian is Slavic —
  Indo-European, like English — so its oldest words rhyme with words you own:
  *нет* is the ancient negation **\*ne** (English *no, not, never*); *привет*
  shares its "speak" root with **Soviet** (*совет*, a council); *есть* "is" is
  the same verb as English *is*.
- **Courtesy words as fossilised prayers.** *спасибо* = *спаси Бог* ("God save
  you"), a sibling of Spanish *adiós* and English *goodbye*; *пожалуйста* asks
  for a favour and doubles as "you're welcome."
- **Grammar inline**: the formal/informal split and *politeness = plural* rule
  arrive at *здравствуйте* (its polite *-те*).
- **Drivable by ear.** 34 of the track's 50 lessons need only your ears; the
  rest are the five handwriting lessons, one cover-the-column retrieval drill,
  and four lessons whose new Cyrillic letter (ш, ф, х, ц) needs a look at the
  page. Russian used to be the least drivable track here, entirely because its
  cross-language comparisons were set as tables rather than said as sentences.
  They are sentences now, and Chapters 3, 4 and 5 were written that way from the
  start — three chapters drivable end to end.
- **Nouns arrive with a gender and no article at all.** Chapter 6 opens the
  noun half of this book on *вода*: masculine ends in a consonant, feminine in
  *-а/-я*, neuter in *-о/-е*, and Russian has no word for *the* or *a* to
  learn on top of it. *кофе* immediately breaks the rule on purpose — masculine
  despite its *-е* ending, a fossil of the vanished form *кофий* — because a
  rule taught only through its regular cases isn't earned yet.
- **Stress lives in the romanization, not on the Cyrillic.** Russian print does
  not mark stress and the vendored Cyrillic font carries no combining acute, so
  this track puts the accent where it can always be rendered and always be read:
  *chitát'*, *pishú*, *lyublyú*. See
  [`_fonts/README.md`](../_fonts/README.md) for why.
- **Aspect is named, not finished.** Chapters 4 and 5 are eight imperfective
  verbs, and every lesson gives its perfective partner by name — *читать ·
  прочитать*, *брать · взять*, *спрашивать · спросить*. What a pair does to a
  whole sentence is deferred to a chapter of its own, because it is the hardest
  thing about Russian verbs and cannot be done in a footnote.

## Assessment destination

The complete book targets a project-defined [pre-A1 bridge followed by the
external TORFL/ТРКИ ladder](assessment-spec.md) from A1 through C2. Its
machine-readable [contract](assessment.json) requires independent reading,
listening, writing, and speaking passes, the complete gentle Cyrillic writing
ramp, and two full timed mocks at every rung. External-format mocks must also
include TORFL's compulsory Lexis/Grammar subtest. This names the destination; it
does not claim that the current book is exam-ready or that pre-A1 is an official
TORFL certificate.

## Progress

- **Chapter 1 — Greetings & courtesy** ([`lessons/RU-C01-*`](./lessons/)):
  привет (hi) → здравствуйте (hello, formal) → спасибо (thank you) → да (yes) →
  нет (no) → пожалуйста (please / you're welcome), plus a practice recap. Six
  words, and enough Cyrillic to read them all cold.
- **Chapter 2 — Introducing yourself** ([`lessons/RU-C02-*`](./lessons/)):
  я → ты / вы → why вы is polite → меня зовут… → как вас зовут? → why Russian
  asks “how” → очень приятно, followed by three focused practices for the
  exchange, person shapes, and zero copula. Case appears only through the forms
  the exchange needs. Every lesson is prerequisite-ordered and below five
  minutes, while the cross-language and etymological depth remains intact.
- **Chapter 3 — Six verbs, and the one you never say**
  ([`lessons/RU-C03-*`](./lessons/)): быть (to be) → жить (to live) → знать (to
  know) → говорить (to speak) → видеть (to see) → идти (to go). One verb per
  lesson and one grammatical idea per verb — the **zero copula**, the **-у** that
  by itself means "I", **не** as the whole of English *don't*, the
  **-ешь / -ишь** families, the **д → ж** swap in *вижу*, and verbs of motion
  (*иду* now, against *хожу* habitually). The etymology carries the chapter:
  *быть* is **be**, *знать* is **know**, *видеть* is **wit** and **wise**, *жить*
  is **quick** in its older sense of *alive*, and *идти → шёл* is *go → went* in
  a second language. One new letter (**г**), and English *govern* flagged as the
  false friend it is.

- **Chapter 4 — Verbs of the mind and the page**
  ([`lessons/RU-C04-*`](./lessons/)): думать (to think) → понимать (to
  understand) → читать (to read) → писать (to write). The chapter opens the
  **aspect pair** — every Russian verb travels with a finished partner — and
  explains why Russian has no separate *I am reading*: **быть** has no present
  tense to build one with. *думать* is the **Дума**, and the standard account
  ties it to English *doom* by Gothic borrowing rather than descent; *понимать*
  is a root meaning **take**, exactly as *comprehend* and *grasp* are; *читать*
  sits with *число* and *честь* and has **no secure English cousin**, which the
  lesson says rather than inventing one; *писать* is \**peyḱ-* "to scratch",
  behind **paint** and **picture**. One new letter, **ш** — the one the track
  has been reading since *живёшь* without naming it — and the **stress trap**
  that turns *pisát'* into *písat'* flagged plainly.
- **Chapter 5 — Taking, asking, helping, loving**
  ([`lessons/RU-C05-*`](./lessons/)): брать (to take) → спрашивать (to ask) →
  помогать (to help) → любить (to love, and to like). *брать* is English
  **bear**, and its partner **взять** is **suppletive** — a different word
  entirely, the same trick that gives *идти* its past *шёл*; *спрашивать* is
  \**preḱ-*, which became Latin *precārī* and so English **pray**; *помогать*
  is built on *мочь* "to be able", English **may** and **might**; *любить* is
  English **love** itself, and teaches the **л** that appears in *люблю* and
  nowhere else. The chapter closes by putting every verb the track owns after
  *люблю* as a bare infinitive.

- **Chapter 6 — Water, coffee, tea, and bread**
  ([`lessons/RU-C06-*`](./lessons/)): вода (water) → кофе (coffee) → чай (tea)
  → хлеб (bread) — the track's first nouns, and the lesson that finally gives
  *пожалуйста* something to attach to. *вода* is PIE \**wódr*, the same root
  as English *water* and *hydro-*, and the source of *vódka* ("little
  water"); *кофе* is masculine by fossil, not by rule, having crossed Arabic →
  Turkish → Dutch into Peter the Great's Russia; *чай* took the opposite,
  overland road — Mandarin *chá* by way of Persian and Turkic — the same split
  that gives English *tea* and Hindi *chai* two different words for one plant;
  *хлеб* looks completely native and isn't, a prehistoric borrowing from
  Proto-Germanic \*hlaibaz, the same root as English *loaf*.
- **Chapter 7 — Friend and siblings** ([`lessons/RU-C07-*`](./lessons/)):
  друг (friend) → подруга (friend, feminine) → брат (brother) → сестра
  (sister) — the four people Chapter 2's ты was always for. *подруга* is built
  live from *по- + друг + -а*, Russian's ordinary feminine-agent pattern;
  *брат* and *сестра* are two of the surest cognates in the book, PIE
  \**bʰréh₂tēr* and \**swésōr*, and the noun *брат* is one soft sign from
  Chapter 5's verb *брать* — the exact minimal pair that lesson warned about.
- **Chapter 8 — Family** ([`lessons/RU-C08-semya.md`](./lessons/RU-C08-semya.md)):
  семья gathers Chapter 7's four people, and its root — PIE \**ḱey-*, "to lie
  down, settle" — is a distant cousin of English *home*, *hamlet*, and every
  English place name ending in *-ham*.
- **Chapter 9 — Eyes, ears, mouth, and nose** ([`lessons/RU-C09-*`](./lessons/)):
  ухо (ear) → нос (nose) → рот (mouth) → глаз (eye) — the track's first
  realization of `SPINE-CHECK-WELLBEING`. *ухо* and *нос* are straight PIE
  inheritances (English *ear*, *nose*); *рот* has no secure English cousin at
  all; and *глаз* is the strangest word in the chapter — slang for "round
  stone" that evicted Russian's true inherited eye-word, *око* (the real
  cousin of English *eye*), which survives today only in *очки*, "glasses" —
  literally "little eyes."
- **Chapter 10 — Heart** ([`lessons/RU-C10-serdtse.md`](./lessons/RU-C10-serdtse.md)):
  сердце closes the tranche on the surest cognate in the book — PIE \**ḱerd-*,
  the same root as English *heart*, Latin *cor/cordis* (→ *cordial*,
  *courage*, *record*), and Greek *kardía* (→ *cardiac*) — with a silent **д**
  and the new letter **ц**.
- **Chapter 11 — Parting words** ([`lessons/RU-C11-*`](./lessons/)): до
  свидания (goodbye) → пока (bye) → до скорого (see you soon) → до завтра
  (see you tomorrow) → до встречи (see you later) → спокойной ночи (good
  night) — the track's first realization of `SPINE-TAKE-LEAVE`, and the first
  in the corpus to close all six of the node's concepts. *до свидания* is
  built on *видеть* (Chapter 3's "to see"), the same move as French *au
  revoir* and German *auf Wiedersehen*; *пока* shares its root with *как* and
  the whole English *wh-* family, though its use as a farewell is only
  20th-century slang; *ночь* and *покой* are two of the surest cognates in
  the book, matching English *night* and *quiet*/*while*.
- **Chapter 12 — Mother and Father** ([`lessons/RU-C12-*`](./lessons/)): мама
  (mom) → папа (dad) → мать (mother, formal) → отец (father, formal) —
  completing the family Chapter 8's *семья* gathered. *мама* and *папа* are
  universal infant-babbling words, not true cousins of anything; *мать* is a
  secure cousin of English *mother*; *отец* is genuinely ancient but is
  **not** related to English *father* at all — Slavic lost the inherited
  word and replaced it with a different old nursery root. *папа* is also the
  book's first live example of natural gender overriding the *-а/-я* ending
  rule.
- **Chapter 13 — Milk, Cheese, Juice, and Soup** ([`lessons/RU-C13-*`](./lessons/)):
  молоко (milk) → сыр (cheese) → сок (juice) → суп (soup), extending Chapter
  6's drink shelf. *молоко* is a straight cousin of English *milk*, down to
  the hand motion; *сыр* shares its root with *сырой* ("raw") and English
  *sour*; *сок* is ancient and reconstructable but has no surviving English
  cousin; *суп* mirrors *кофе*'s loanword story — an 18th-century borrowing
  from French that English also independently borrowed.

See [`roadmap.md`](./roadmap.md) for the plan toward B1 and
[`session-map.md`](./session-map.md) for how the lessons compose into commute
sessions.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## What each chapter lets you do

[`chapters.json`](./chapters.json) is the HL05 capability ledger: per chapter, one
first-person can-do sentence and the lesson that pays it off.

- **Chapter 2** — *"I can give my name in Russian, ask for someone else's, and
  pick ты or вы to match how well I know them."* Payoff:
  [`RU-C02-practice`](./lessons/RU-C02-practice.md), a dialogue — run the whole
  introduction with a stranger, then switch it for one friend by changing only
  the greeting and *вас → тебя*. It assesses ten of the chapter's fifteen
  introduced atoms, for a representativeness of **0.67** against the 0.5 floor.

  One honest caveat remains: Russian has no `core/book-generation.json` targets,
  so the chapter title and label come from
  [`book/chapters/ch02-introducing-yourself.tex`](./book/chapters/ch02-introducing-yourself.tex).

- **Chapter 11** — *"I can close a Russian conversation with the parting word
  that fits how long until I see the person again, and say good night."*
  Payoff: [`RU-C11-spokoynoy-nochi`](./lessons/RU-C11-spokoynoy-nochi.md),
  which produces all six parting words in order and reaches back to Chapter 6
  to rescue **ф**, **х**, *чай*, *хлеб* and the request pattern.
- **Chapter 12** — *"I can name my mother and father in Russian, in both the
  affectionate and the formal register, and say which one is a real cousin
  of the English word and which is not."* Payoff:
  [`RU-C12-otets`](./lessons/RU-C12-otets.md), which produces all four family
  words, states the natural-gender exception *папа* tests, and reaches back
  to Chapter 8's *семья* to complete the family it gathers.
- **Chapter 13** — *"I can name milk, cheese, juice and soup in Russian with
  their genders, ask for any of them politely, and say which are inherited
  and which are borrowed."* Payoff:
  [`RU-C13-sup`](./lessons/RU-C13-sup.md), which produces all four
  drink-shelf words and reaches all the way back to Chapter 2 to rescue the
  naming-question comparison and *я*'s script warning.

**Chapter 1 is not in the ledger**, and that gap is deliberate: all twelve of its
lessons are schema v1, so it has no assessable payoff to point at. A placeholder
would hide debt the HL05 gap report is meant to surface.

**Chapter 3 is not in the ledger either**, for a different and equally recorded
reason: the ledger is checked against the chapters the *book* has, and Russian's
book chapters are handwritten `.tex` files. No `ch03` was authored, so there is
no book chapter for a capability entry to describe. The six lessons are
canonical schema-v2 content and the app can serve them today; the book is what
is behind.

## Read and practise

- [`book/book.tex`](./book/book.tex) builds the free two-chapter starter edition
  with XeLaTeX and the vendored Cyrillic font.
- Merged editions appear in the public human-languages book catalog.

## Status

Chapters 1 and 2 are authored as lessons and as a downloadable LaTeX starter
book typeset with the vendored `NotoSansCyrillic-Static.ttf`.
The six-lesson naming chain is also schema-v2 canonical content shared by the
app, with both mapped non-lexical Russian frontiers using objective activities.
Chapter 3 is authored as six schema-v2 lessons only — no book chapter yet. It is
also the corpus's first realization of the shared `VERB-*` concepts (six of the
core forty), and the first content anywhere to sit on an A2 spine node.
