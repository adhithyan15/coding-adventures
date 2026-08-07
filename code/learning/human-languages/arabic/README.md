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
- **Chapters 31–32 — Eight more verbs, and the root system paying out**
  ([`lessons/AR-C{31,32}-*`](./lessons/)): **fahima** ("he understood") →
  **qaraʾa** ("he read") → **saʾala** ("he asked") → **kataba** ("he wrote"),
  then **ʾakhadha** ("he took") → **fakkara** ("he thought") → **sāʿada** ("he
  helped") → **ʾaḥabba** ("he loved"). Chapter 31 is where the root engine
  stops being demonstrated and starts being *used*: **ك-ت-ب** is poured through
  every pattern the track already taught and returns *kitāb* ("a book"),
  *kātib* ("a writer"), *maktūb* ("written"), *maktab* ("an office") and
  *maktaba* ("a library") with nothing new to memorise. Chapter 32 is a ladder
  of verb **shapes** — plain *ʾakhadha*, Form II *fakkara* (middle consonant
  doubled), Form III *sāʿada* (vowel stretched), Form IV *ʾaḥabba* (prefixed) —
  taught as a system rather than as irregularities. Cousins are claimed only
  where they exist: **ق-ر-أ** really did give English **Quran** and **س-ع-د**
  the **Saudi** in Saudi Arabia, while *fahima*, *fakkara*, *kataba*,
  *ʾakhadha*, *saʾala* and *ʾaḥabba* have **no English relative at all**, which
  each lesson says outright. In the book.

All fifty-nine lessons in Chapters 3–32 remain below five effective minutes.

## Can you learn this track in the car?

Mostly. Under [`HL08`](../../../specs/HL08-modality-gentle-ramp-and-the-drivable-course.md)
each lesson is `voice` 🚗, `sight` 👁 or `pen` ✍ at **full** modality, and
Arabic measures 44 / 25 / 16 — but 20 of those `sight` lessons carry their
visual load entirely inside a **detachable** block, so **56 of 85 lessons have
a `voice` core (75% drivable)**, with 56 reachable in chapter-prefix order.

The eight verb lessons of Chapters 31–32 sit in exactly that group. Each uses
the canonical `## The letters in this word` heading, which types as a `script`
block: honest at full modality (a letter shape is not a sound), detachable for
a hands-free renderer, so their `coreModality` is `voice`. The cost is
visible and deliberate — Arabic's `sight` count rises by eight and two more
chapters become "unstartable" at full modality — and the alternative was the
older habit of hiding the letters section behind a non-script heading, which
made the number look better without making the lesson more listenable.

Chapters 3, 4 and 8 are prefix 0 at full modality, and so are Chapters 31 and
32 — the latter two only because every lesson in them opens on its detachable
letters block, which is why they still count as fully drivable once that block
is set aside. Chapters 3 and 4 are the real blocks: each opens on a `pen`
writing lesson that later lessons
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
it. Chapters 3–32 are covered. Chapters 1 and 2 are deliberately left out: their
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
