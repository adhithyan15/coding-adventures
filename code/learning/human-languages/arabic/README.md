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
- **Chapters 28–30 — The first verbs**
  ([`lessons/AR-C{28,29,30}-*`](./lessons/)): **dhahaba** ("he went") →
  **jāʾa** ("he came") → **qāla** ("he said") → **raʾā** ("he saw") →
  **ʿarafa** ("he knew") → **akala** ("he ate"). Six lessons, one verb each,
  and the arc where the root-and-pattern engine stops being a remark and
  becomes the subject: *dh-h-b* gives "he went," *dhahab* ("gold") and
  *madhhab* ("a school of law"); the *ma-* place shape behind *madhhab* is the
  one English already owns in **mosque** and **Maghreb**. Along the way the
  three kinds of **weak root** (middle-weak *jāʾa*/*qāla*, final-weak *raʾā*)
  show that even Arabic's irregularities are patterned, and the letters **ذ**,
  **ق**, **ى** and **ف** are taught inline in the words that need them —
  *akala*, last of the six, needs no new letter at all. These are the
  curriculum's **first canonical `VERB-*` realizations in any track**, and the
  first lessons anywhere in the corpus to reach **A2**. In the book.

All fifty-one lessons in Chapters 3–30 remain below five effective minutes.

## Can you learn this track in the car?

Mostly. Under [`HL08`](../../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md)
each lesson is `voice` 🚗, `sight` 👁 or `pen` ✍, and Arabic measures 56 / 5 / 16
— **73% drivable**, with 48 lessons reachable in chapter-prefix order. (The six
verb lessons of Chapters 28–30 are all `voice`, and those three chapters are
drivable end to end: the letters they teach are described in speakable prose,
not shown as figures.)

The instinct is to blame the script, and it is wrong. **Not one `sight` lesson
in the track is `sight` because of a script block.** Four of the five are
`sight` for a sight cue in the prose and the fifth for a table the narration
lineariser will not read. Only Chapters 3, 4 and 8 are still prefix 0; every
other chapter is drivable from its first lesson to its last. Chapters 3 and 4
are blocked because each opens on a `pen` writing lesson that later lessons
depend on: a chapter's drivable prefix can only start with a lesson that has no
in-chapter prerequisite, and Chapter 4's only such lesson is `AR-W10-ayn`,
because `AR-C04-maa-with` requires it — مع cannot be read without ʿayn.
Chapter 8 is blocked by a single table. What frees the remaining three is
sight-cue rewording and table linearisation (HL-C17), not resequencing.

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
