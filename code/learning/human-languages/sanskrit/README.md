# Sanskrit

A track of the [Human Languages](../README.md) curriculum, built the same way
as: one word per lesson, taken apart and traced to its root; the pieces taught
before the whole; and a book you can read straight through.

## What's different about the Sanskrit track

- **A taproot track — like Latin, but pointing east *and* west.** Sanskrit is
  the classical ancestor of the Hindi, Marathi, Punjabi, and Bengali tracks
  (every *namaste* and *dhanyavād* is a worn form of a Sanskrit word met here
  in full), **and** a sister of Latin, Greek, and English. Its roots reach west
  too — *te* ↔ *thee*, *su-* ↔ Greek *eu-*, √gam ↔ *come*, *na* ↔ Latin *nōn* ↔
  *no*. Learning it ties the two halves of the curriculum together.
- **Devanagari, taught inline** (vendored Noto Sans Devanagari — same font as
  the Hindi/Marathi tracks), with attention to what Sanskrit needs even from a
  Devanagari reader: the *visarga* (ḥ), the vocalic ṛ, conjuncts, and *sandhi*.
- **IAST transliteration** alongside the script, roots traced back toward
  Proto-Indo-European where the trail is clear.

## Progress

- **Chapter 1 — Greetings** ([`lessons/SA-C01-*`](./lessons/)): namaste,
  namaskāraḥ, dhanyavādaḥ, svāgatam, ām/na, practice. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/SA-C02-*`](./lessons/)):
  nāma, mama, asti, **mama nāma … asti** (Sanskrit keeps the copula),
  bhavān/tvam, kim, **tava nāma kim?**, ānandaḥ, practice. Each atom a *source*
  (→ *name/my/is/what*). In the book.
- **Chapter 3 — How Are You** ([`lessons/SA-C03-*`](./lessons/)): katham,
  **bhavān katham asti?**, aham (← *ego* → *I*), kuśalam, na cintā, practice.
  The copula trio asmi/asi/asti. In the book.
- **Chapter 4 — Farewells** ([`lessons/SA-C04-*`](./lessons/)): gacchāmi (←
  *gam* → *come*), punaḥ, **punar-darśanāya**, śvaḥ (kept distinct from
  *hyaḥ*), practice. The dative case. In the book.
- **Chapter 5 — First Verbs** ([`lessons/SA-C05-*`](./lessons/)): vadāmi (the
  **dual** *vadāvaḥ*), **ahaṁ saṁskṛtaṁ vadāmi**, vasāmi (← *vas* → *was*),
  karomi (← √kṛ), practice. In the book.
- **Chapter 6 — Numbers 1–5** ([`lessons/SA-C06-*`](./lessons/)): the gendered
  forms first, then an east-west sound-law map, then *pañca* in *Punjab*,
  *pentagon*, and the qualified history of *punch*—three prerequisite-ordered
  micro-lessons.
- **Chapter 7 — The Core Verbs** ([`lessons/SA-C07-*`](./lessons/)): asti /
  bhavati, gacchati, āgacchati, khādati, paśyati, jānāti. The **dhātu** (root)
  and its **gaṇa** (present-stem class) taught as the system they are, three of
  the ten classes walked in order, the *upasarga* prefix shown by turning "goes"
  into "comes," and every root followed west — \**es-*/\**bheu-* into English's
  patchwork *am/is/are/be/been*, \**gwem-* into *come* and *advent*, \**spek-*
  into *inspect* and *telescope*, \**gno-* into *know* and *diagnosis*.
  **Fully drivable — all six lessons are voice.**
- **Chapter 8 — The Mind and the Palm Leaf**
  ([`lessons/SA-C08-*`](./lessons/)): cintayati, avagacchati/budhyate, paṭhati,
  likhati. Class 10 plants **-अय-**; "understands" is built out of **अव** and a
  verb already owned; **लिख्** means *scratch* before it means *write*. The
  chapter names **तत्सम** against **तद्भव** so that every claim about a
  descendant says which kind it is. **Fully drivable.**
- **Chapter 9 — Taking, Asking, Helping, Loving**
  ([`lessons/SA-C09-*`](./lessons/)): gṛhṇāti, pṛcchati, sāhāyyaṁ karoti,
  snihyati/priyam. \**ghrebh-* → *grab*; \**prek-* → *pray* and *fragen*;
  सहाय "one who goes with" beside Latin *comes*; \**priHos* → *friend*, *free*,
  *Friday* and, eastward, Hindi *piyā*. **Fully drivable.**
- **Chapter 10 — What You'd Ask For** ([`lessons/SA-C10-*`](./lessons/)):
  पानीयम्, क्षीरम्, अन्नम् — chosen over the more textbook जल/उदक because
  पानीयम् is the attested ancestor of the "water" words already taught in
  Hindi, Gujarati and Bengali. Names the three grammatical genders every
  Sanskrit noun carries.
- **Chapter 11 — The People You'd Introduce** ([`lessons/SA-C11-*`](./lessons/)):
  मित्रम्, कुटुम्बम्, भ्राता, भगिनी. भगिनी, not the more "correct"-looking
  स्वसृ, is the verified ancestor of the daughter-language sister-words.
  कुटुम्बम् is a Dravidian loan **into** Sanskrit, later re-lent back to
  Kannada, Telugu and Tamil — the taproot running backwards.
- **Chapter 12 — The Face, Part One** ([`lessons/SA-C12-*`](./lessons/)):
  अक्षि, कर्णः, मुखम् — says why eyes take the dual number.
- **Chapter 13 — The Face, Part Two** ([`lessons/SA-C13-*`](./lessons/)):
  नासिका, हृदयम् — names the PIE root हृदयम् shares with *heart*, *cor* and
  *kardia*.

Chapters 1–13 are in the book. Core verb coverage: **14 of the canonical 40**,
including all eight verbs the other fifteen verb-bearing tracks share. All
seven pre-A1 spine nodes are now realized.

- **The script ladder** ([`lessons/SA-S*`](./lessons/)): 48 one-character
  lessons, starting in **chapter 1** and spaced at least two content lessons
  apart, so every word containing a character is met **glossed** before the
  character itself is taught. Each lesson introduces exactly one Devanagari
  character and nothing else — one in the whole lesson, not merely one in the
  title — and that is verified by walking the corpus in reading order, which is
  how four segments were caught smuggling a second character in through a
  component description, a stroke note or a quoted gloss. Pen path, pen-lift
  count and citation are read from
  [`data/scripts/devanagari.json`](../data/scripts/devanagari.json) rather than
  written by hand; where that file records no source (the vowel signs), the
  lesson asks the reader to copy the printed shape and says plainly that the
  stroke direction is not written down here yet. **48 of the 53 characters the
  track shows are now taught**; the remaining five are accounted for in backlog
  `HL-C241`.
- **Chapters 62–63 — the ordinals** ([`lessons/SA-C62-*`](./lessons/),
  [`lessons/SA-C63-*`](./lessons/)): first to tenth, one word a lesson, and NOT
  in numerical order. **62** opens at *fifth*, because **-म** is the ending that
  generalizes — it makes five, seven, eight, nine and ten with nothing else
  changing — and it skips *sixth* on purpose. **63** works outward from that
  hole: **षष्ठः** takes **-थ**, and stops being an oddity as soon as
  **चतुर्थः** shows the same ending; **तृतीयः** and **द्वितीयः** take a third
  ending, **-तीय**; and **प्रथमः** arrives last because it is not a numeral at
  all — Macdonell records it as *pra-tama*, a superlative of **प्र**
  "forward", exactly as English *first* is the superlative of *fore*. The
  grouping is read off the dictionary rather than asserted: Macdonell prints the
  join inside each of the ten headwords, and the chapter-63 payoff reproduces
  that table with his page numbers. A1 exam coverage 141/164 → **142/164**, and
  the Saṅkhyā column closes outright.
- **Chapters 275–285 — the B1 core** ([`lessons/`](./lessons/), `SA-C275-*`
  to `SA-R285-depends-and-would`): the track's first running sentences, each
  line short, romanized and translated. 275 bridges the A2 future — every
  person and number on करिष्य-, and each verb's own stem (पठिष्यति,
  द्रक्ष्यति, दास्यति). Then two chapters for each of the five B1 spine nodes:
  narrating (the -tavat past that follows the doer, सः गतवान् / सा गतवती; the
  present with स्म in a यदा … तदा scene; -त्वा and -य; प्रथमम् … अन्ते; an
  eighty-word reading), giving reasons (मम मतेन, मन्ये, यतः, -कारणात्, the
  optative of advice भवान् गच्छेत्, अतः, तस्मात्), travel (asking the way with
  the polite -तु, a ticket, a lost bag, a missed train), experience (feelings
  agreeing with the speaker, भवितुम् इच्छामि, -अपेक्षया and -तर, आरभ्य …
  -पर्यन्तम्) and conditions (यदि … तर्हि, अन्यथा, -अधीन, the conditional
  अभविष्यत्), closing on a six-line writing task. Pāṇini, Lanman's reader
  and Monier-Williams are the sources cited. Four letter
  lessons, `SA-S226-letter-gha` to `SA-S229-letter-ii` (घ ढ औ ई), sit right
  after their anchor words घटते, दृढः, औषधम् and ईदृशः and read their stroke
  order from the inventory; four filmstrips are added, and ऊ ऐ झ ळ stay
  unread. Still untaught: the passive, the perfect and aorist, vowel sandhi
  across words and the avagraha.
- **Chapters 286–555 — the B1 vocabulary** ([`lessons/`](./lessons/),
  `SA-C286-*` to `SA-R555-*`): 1,350 headwords, 394 of them verbs (each shown
  with its -tavat past agreeing with the doer), one word per lesson, with
  eight reviews in each of the five tranches:
  - 286–339: narrating (theft and the watch, accidents and fire, disasters
    and rescue, news and rumour, kings, war and history, festivals, the epics
    and the Pañcatantra);
  - 340–393: reasons and conditions (causes and purposes, debate and proof,
    law and dharma, government, values, study and research, then likelihood,
    conditions, degrees and goals);
  - 394–447: travel (pilgrimage and temples, roads, wheels and halts, rivers
    and boats, towns and gates, lodging, the bazaar and money, the weather,
    the sky and the fields);
  - 448–501: experience (feelings and character, the body and health,
    Ayurveda and care, birth, upbringing, the wedding and the family);
  - 502–555: crafts and work, the kitchen, free time, the arts and sport, then
    plans, luck, omens and choices.

  Examples use only the words and grammar taught before them: no passive,
  perfect or aorist, no case form the track has not shown (no genitive or
  dative plural, oblique dual or s- and n-stem oblique yet), and every
  condition from chapter 367 on in यदि … तर्हि or the conditional. Every word
  cites Monier-Williams; descendants cite CDIAL, and Western cousins appear
  only where Monier-Williams compares them. The first words of chapters
  286–293 practise the core's last grammar points again, and घ ढ औ ई each
  come back twice in chapters 294–316. With the core, Sanskrit teaches 2,601
  headwords and 563 verbs at or below B1 and attains B1 on the level gate.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## What each chapter lets you do

[`chapters.json`](./chapters.json) is the HL05 capability ledger: per chapter, one
first-person can-do sentence and the lesson that pays it off.

- **Chapter 6** — *"I can say the Sanskrit numerals एक to पञ्च with their dual and
  gendered forms, and follow पञ्च outward into Punjab, pentagon, and the disputed
  history of punch."* Payoff:
  [`SA-C06-pancha-travels`](./lessons/SA-C06-pancha-travels.md), a task.

  Its representativeness is 7/15 introduced atoms (0.47), just under the 0.5
  policy floor. This chapter is the widest of the Indic six, and its terminal
  lesson follows the *pañca* thread rather than the dual, the gendered paradigm,
  or the Grimm's-law material. The shortfall is recorded rather than padded away.

- **Chapter 7** — *"I can say that someone is, becomes, goes, comes, eats, sees
  and knows in Sanskrit; I can name each verb by its dhātu rather than by its
  present form, say which of the three gaṇa patterns builds its stem, and point
  at the English word that descends from the same root."* Payoff:
  [`SA-C07-janati`](./lessons/SA-C07-janati.md), a production.

  Representativeness is 12/12 (1.00): the payoff lesson gathers the whole
  chapter, because the chapter is one system taught six times rather than six
  unrelated words.

- **Chapter 8** — *"I can say that someone thinks, understands, reads and writes
  in Sanskrit; … and tell a word worn down by speech (तद्भव) from one carried
  over whole (तत्सम)."* Payoff:
  [`SA-C08-likhati`](./lessons/SA-C08-likhati.md), a production.
  Representativeness is 8/8 (1.00).

- **Chapter 9** — *"I can say that someone takes, asks, helps and loves in
  Sanskrit; … and point at the English word — grab, pray, friend — that each of
  these four roots also produced."* Payoff:
  [`SA-C09-snihyati`](./lessons/SA-C09-snihyati.md), a production.
  Representativeness is 8/8 (1.00); it also reaches back three chapters to
  retrieve Grimm's law, the *pañca* travels and the analogical *f* of *four*,
  which nothing had revisited since Chapter 6.

- **Chapter 10** — *"I can name water, milk and food in Sanskrit, say that food
  is grammatically the eaten thing, and name the three genders every Sanskrit
  noun carries."*
- **Chapter 11** — *"I can name a friend, a family, a brother and a sister in
  Sanskrit, and say which of those words travelled with English and which
  quietly did not."*
- **Chapter 12** — *"I can name an eye, an ear and a mouth in Sanskrit, say why
  eyes take a dual number, and say which of the three words has a settled
  Western cousin and which do not."*
- **Chapter 13** — *"I can name a nose and a heart in Sanskrit, say which
  daughter-language nose-word नासिका is a relative of rather than a parent of,
  and close the chapter's twelve nouns against the cousin words this book has
  already shown."*

Chapters 1–5 are **not in the ledger yet**, and that gap is deliberate. They are
still schema v1, so their lessons declare no knowledge atoms and no payoff there
could honestly claim to assess anything. A placeholder would hide debt the HL05
gap report is meant to surface; the entries land as those chapters migrate.

## Book / fonts

Compiles with XeLaTeX using the **vendored** Noto Sans Devanagari font
(`../../_fonts/NotoSansDevanagari-Static.ttf`). Build the isolated entrypoint with
`code/scripts/check-book-compile.sh --strict sanskrit` from the repository root.
Generated Devanagari runs use that font while section bookmarks use the
lessons' Latin romanization.

The forced nine-chapter build is warning-free — no overfull or underfull boxes,
no missing characters, no hyperref complaints: chapter-qualified recap anchors,
bookmark-safe Devanagari, natural page bottoms, explicit static-font shapes,
and concise running titles keep the downloadable PDF and its outline clean.

## Files

- [`lessons/`](./lessons/) · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `SA-C01-namaste`); order lives in the book and
`session-map.md`.
