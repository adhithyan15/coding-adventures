## Unreleased — an A1 exam inventory, and the ऋ refusal is half lifted (HL-C310)

`core/exam-inventory-sanskrit-a1.json` is the ninth exam inventory in the repo
and the first for Sanskrit. **164 points, 126 covered, 38 uncovered.** It is a
project-defined editorial equivalent derived structurally from the DELE-sourced
Spanish A1 set as a proxy for LEVEL; nothing in it is attributable to Samskrita
Bharati, to any pariksha, or to the Instituto Cervantes.

- **The `exam-levels.json` caveat was read first and it changed the file's
  shape.** Sanskrit is recorded `basis: editorial` with the note that this
  project teaches it as a living spoken language — so CEFR descriptors apply —
  while "a traditional syllabus is ordered by grammar and text rather than by
  communicative function, so the two ladders are not parallel". That forced a
  **Register category** the proxy cannot supply, and a deliberately UNCOVERED
  point, `SA-A1-RG-02`, saying in the file that this inventory cannot measure a
  traditional pariksha and that anyone who wants it to must build a second
  column rather than extend this one.
- **Every probe was checked mechanically against the 381 atoms the track
  introduces; zero invented ids.** From chapter 14 the lexical ids are opaque
  (`SA-LEX-C31-SKY-03`), so each was first resolved to the word it carries by
  reading the introducing lesson. Probing them by name would have been guessing.
- **The proxy is fully accounted for**: 272 of its 273 points are cited by some
  point's `derivedFrom`; one — Spanish superscript abbreviation letters — is in
  `proxy.notTransferred` with a reason.
- **Twelve points are `sanskritSpecific`**, answering no Spanish demand at all:
  the eight-case system, sandhi, the ten verb classes, the upasarga, the dual,
  the noun families, hospitality as a field, the comparative-etymology layer the
  whole track is built on, and the three register points.

**The work queue, in the order it would pay.** The uncovered notes name what is
missing rather than gesturing: `ca` — one enclitic word, taught nowhere, and a
learner cannot join two nouns; the past tense, on which **seven** proxy points
ride; the `gustar` frame, on which **four** ride and for which no Sanskrit
liking-sentence exists at all; `iti`, without which no thought is reportable;
the `yad … tad` correlative; and a word for a country, since `kutaḥ` ("from
where") is taught with nothing to answer it.

**The ऋ refusal is half lifted, and `roadmap.md` was stale.** The roadmap said ऋ
and ङ "have no cited stroke order in the shared script file and so have no
lesson at all". That is now half true, and the two are blocked on OPPOSITE
halves:

| glyph | sourced ductus? | Sanskrit headword? | verdict |
|---|---|---|---|
| **ऋ** | **yes** — cited four-panel Saurmandal diagram in `data/scripts/devanagari.json` | **no** | sourcing refusal LIFTED; blocked on vocabulary |
| **ङ** | **no** — absent from `devanagari.json` entirely | yes, `सङ्ख्याशब्दाः` ch. 19 | refusal STANDS |

For ऋ the fix is vocabulary first, per HL-C217's own rule that a recognition
segment with an empty "you already say these" list teaches a shape for nothing:
schedule `ऋतु` ("season") — which would also close the calendar half of
`SA-A1-T-02` — or `ऋषि`, then teach the shape. For ङ the refusal must stand: a
stroke order is sourced and cited, never invented. The roadmap now says both.

Script census, re-measured: **53 Devanagari characters shown, 48 taught, 5
neither** (ऋ ङ ई ँ घ).

Counters, re-measured by running the CLI **on the merged tree** rather than by
arithmetic: `npm run plan` moves **8 → 9** written inventories and **748 → 786**
uncovered points; the unmeasurable remainder falls **16 → 15** tracks. Sanskrit
contributes exactly its 38. The base moved under this branch while it was open —
a Telugu tranche took the total 792 → 748 from the other direction — so the
number was measured after the merge, not before it. This line has now moved
529 → 686 → 793 → 792 → 748 → 786.

Verified: human-language-data 124 test files / 1738 passing; all eleven
`check:*` gates; language-ladder 39 files / 442 passing.

