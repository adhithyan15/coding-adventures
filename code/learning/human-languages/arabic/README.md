# Arabic

The fourth track of the [Human Languages](../README.md) curriculum, built the
same way as: one word per lesson, taken apart and traced to its root; the
pieces taught before the whole; and a book you can read straight through.

## What's different about the Arabic track

Arabic doesn't *trace* to roots — its roots are on the **surface**. Nearly
every word is built from a **three-consonant root** carrying a core meaning,
poured into fixed patterns (s-l-m → *salām*/*islām*/*muslim*/*salaam*), which
is the whole curriculum's obsession made literal. So the Arabic track teaches
the **root system** itself as the organizing engine.

Two more things:

- **The script is taught inside the word lessons — no reading course.** Written
  for someone who may not read a single Arabic letter, each word lesson has a
  *"The letters in this word"* section introducing exactly the letters that
  word needs, right to left (سلام brings ا ل م س and the long-ā; مرحبا adds ب ر
  ح). A reader who already reads Arabic skims those notes.
- **Grounded against English + Spanish.** Arabic's long shadow over Spanish is
  a recurring thread: the article **al-** smuggled into English *algebra*/
  *alcohol*, and the sun-letter assimilation you can still hear in Spanish
  *azúcar* (← *as-sukkar*) — every form supplied so no prior Spanish is
  assumed. The Al-Andalus loanwords the Spanish track traces *backward* are met
  here from the source.

## Progress

- **Chapter 1 — Greetings** ([`lessons/AR-C01-*`](./lessons/)): salām →
  marḥaban → al- → as-salāmu ʿalaykum → ṣabāḥ al-khayr → masāʾ al-khayr →
  shukran → practice. The Arabic script is taught inline (RTL, connecting
  letters, dots-on-a-skeleton, the emphatic consonants, ʿayn/hamza), and the
  root engine + attached *al-* are shown as the words are built. In the book.
- **Chapter 2 — Introducing Yourself** ([`lessons/AR-C02-*`](./lessons/)): ism,
  -ī ("my"), **ismī** ("my name is," zero copula), anta/anti (gendered "you"),
  mā, **mā ismuka/ismuki?** ("what's your name?"), tasharrafnā, practice. The
  zero copula (shared with Dravidian) and "you" split by **gender**. In the
  book.
- **Chapters 3–7 — Wellbeing, farewells, and courtesy**
  ([`lessons/AR-C0{3,4,5,6,7}-*`](./lessons/)): ask and answer how someone is,
  assemble **maʿa s-salāma**, respond yes or no, and add please and sorry. Six
  dependency-ordered writing companions introduce only the letters and joins
  needed by the spoken lessons. In the book.
- **Chapters 8–18 — Calendar and everyday domains**
  ([`lessons/AR-C{08..18}-*`](./lessons/)): days, colours, family, body,
  seasons, food, months, dayparts, clock time, age, and weather, with Arabic's
  root and pattern system kept visible. In the book.
- **Chapters 19–27 — Counting, description, and leave-taking**
  ([`lessons/AR-C{19..27}-*`](./lessons/)): numbers one through twenty,
  animals, more colours, **ʿafwan**, and a gentle sequence from tomorrow and
  day/night vocabulary to **tuṣbiḥ ʿalā khayr**. In the book.

All forty-five lessons in Chapters 3–27 remain below five effective minutes.

## Can you learn this track in the car?

Partly. Under [`HL08`](../../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md)
each lesson is `voice` 🚗, `sight` 👁 or `pen` ✍, and Arabic measures 37 / 18 / 16
— **52% drivable**, with 31 lessons reachable in chapter-prefix order.

The instinct is to blame the script, and it is wrong. **All 18 of the `sight`
lessons are `sight` because of a Markdown table, none because of a script
block.** Chapters 3 and 4 do open with writing lessons, but moving them would
change nothing: a chapter's drivable prefix can only start with a lesson that
has no in-chapter prerequisite, and Chapter 3's only candidates are
`AR-W07-hook-family-ha-kha` (`pen`) and `AR-C03-kayfa` (`sight`, table), while
Chapter 4's only candidate is `AR-W10-ayn` — `AR-C04-maa-with` requires it,
because مع cannot be read without ʿayn. Both chapters are prefix 0 under *every*
legal ordering, and would still be at 0 with the writing lessons deleted. The
work that actually frees this track is table linearisation (HL-C17), not
resequencing.

Chapters 1 and 2 are, separately, **undercounted**: their lessons are still
schema v1 and carry no `sequence`, so the modality report falls back to
alphabetical order and reports prefixes of 4 and 6 where the authored path in
`curriculum.json` gives 7 and 7. That is a measurement artifact of the mixed
schema, not a curriculum defect.

## Book / fonts

Open-right blank versos are intentionally retained for print and contain no
running header or page number.

---

## For contributors

Everything below this line is about how the track is built and checked. It is
here for people working on the curriculum; nothing in it is needed to learn the
language.

## Chapter capabilities

[`chapters.json`](./chapters.json) is the track's
[`HL05`](../../../specs/HL05-chapter-capability-and-step-by-step-shape.md)
capability ledger. Each entry says, in the reader's own first-person words, what
finishing that chapter lets them do, and names the lesson that proves it:

```json
{
  "chapter": 17,
  "title": "Asking Someone's Age",
  "canDo": "I can ask how old someone is in Arabic and give my own age without using a verb.",
  "payoff": { "lesson": "AR-C17-kam-umruka", "kind": "dialogue", "assesses": ["…"] }
}
```

The file is **authored intent**, not a derived cache — no validator may rewrite
it. Chapters 3–27 are covered. Chapters 1 and 2 are deliberately left out: their
recap lessons are still schema v1 with no declared knowledge atoms, so a payoff
there could only be invented. That absence is honest, measurable debt and is
reported as such.

## Files

- [`lessons/`](./lessons/) · [`chapters.json`](./chapters.json)
  · [`pronunciation-reference.md`](./pronunciation-reference.md)
  · [`roadmap.md`](./roadmap.md) · [`session-map.md`](./session-map.md)
  · [`book/`](./book/)

Lessons are slug-named (e.g. `AR-C01-salam`); schema-v2 lessons carry their
prerequisite-safe sequence in canonical metadata consumed by both app and book.
