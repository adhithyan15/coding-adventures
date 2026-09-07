## Chapters 34–35 — the second verb tranche, and thirteen rescued atoms

Hindi sat at **4 of the shared spine's 40 core verbs**, the thinnest coverage of
any large track, while Spanish (21), Latin (16) and Portuguese (15) had already
taught the eight verbs below. These two chapters bring Hindi in, so each of the
eight becomes a **four-way** cross-language join — and, unlike the three tracks
already there, one that reaches across language families.

| Lesson | Concept | Word |
|---|---|---|
| `HI-C34-sochna` | `VERB-THINK` | सोचना |
| `HI-C34-samajhna` | `VERB-UNDERSTAND` | समझना |
| `HI-C34-padhna` | `VERB-READ` | पढ़ना |
| `HI-C34-likhna` | `VERB-WRITE` | लिखना |
| `HI-C35-lena` | `VERB-TAKE` | लेना |
| `HI-C35-puchna` | `VERB-ASK` | पूछना |
| `HI-C35-madad` | `VERB-HELP` | मदद करना |
| `HI-C35-pasand` | `VERB-LIKE-LOVE` | पसंद |

**Two chapters, not one.** Eight one-verb lessons introduce seventeen atoms
against `maxNewAtomsPerChapter: 12`; the last wave of tracks all broke that
ceiling. Splitting is the resolution rather than raising the budget, so chapter
34 introduces **8** atoms and chapter 35 introduces **9**, each with its own
capability and its own payoff. Hindi reports **zero** ramp chapter violations.

**Every payoff reaches back.** The corpus was measured at 51% of taught atoms
never revisited, median zero. A tranche that only reviews itself adds to that
pile, so each of the eight lessons re-practises at least one atom from an
earlier chapter — thirteen in all, and **all thirteen had never been practised
again anywhere in the corpus before this branch**: नहीं (7), माफ़ कीजिए ×2 (9),
पानी and रोटी (15), उम्र and कितने साल के हो (19), एक–पाँच (6), छह–दस (21),
कुत्ता (23), शाम (29), the preposed ि (W03) and मेरा नाम (W04). The reach-back
is teaching, not name-checking: *maiṁ nahīṁ samajhtā* is built on chapter 7's
negator, *paṛhnā*'s drill is reciting one to ten because that is literally what
the verb once meant, and *madad karnā* finally explains the join the learner
used unknowingly in *māf kījiye*.

**Hindi's own signature, three times over.**

- **पसंद is not a verb of liking — it is not a verb at all.** It is a Persian
  noun/adjective, and *mujhe roṭī pasand hai* means "to me, bread is pleasing":
  the liker sits in the dative **मुझे** and the thing liked is the grammatical
  subject that **है** agrees with. Spanish arrived at the identical shape in *me
  gusta el café* and Italian in *mi piace*, in another family, with no borrowing
  in either direction. Spanish is supplied as self-contained contrast, never as
  assumed knowledge, so the chapter stands alone in a single-language PDF.
- **मदद करना opens the conjunct verb**, not just one word. Arabic *madad* (root
  *m-d-d*, "to stretch out" — help as a hand extended) plus native **करना**, and
  only *karnā* ever conjugates. That is the mechanism by which Hindi absorbed
  centuries of Persian and Arabic vocabulary without ever bending a foreign verb:
  the loan stays a frozen noun and native grammar does the work.
- **पढ़ना means read *and* study**, because Sanskrit **पठति** *paṭhati* meant
  "to **recite aloud**" in a tradition whose oldest texts are *śruti*, "that
  which is heard." The retroflex flap **ढ़** — ढ under the same nuqtā met in
  *māf* — gets its own pronunciation note.

**Etymology carries the rest, and names its own gaps.** *sochnā* ← *śocati* "to
burn, to grieve," which is why **सोच** still means thought *and* worry;
*samajhnā* ← *sam-* + *budh-* "to wake," the root of **Buddha**, **bodhi** and
English **bode/forebode/forbid**; *lenā* ← *labhate* with the *-bh-* eroded and
the book-borrowed **लाभ** preserving it; *pūchnā* ← *pṛcchati* on \**prek-*, the
verb English kept as **pray**, **prayer** and **precarious** after its own
inherited *frignan* died out; *likhnā* ← *likhati* "to scratch," beside Latin
*scrībere*, Greek *gráphein* and Old English *wrītan* — **four separate roots**,
so the shared thing is the metaphor, not the word. Three dead ends are stated as
dead ends rather than papered over: *śuc-* has no secure English cousin,
*paṭh-* has no secure Indo-European ancestry at all, and English inherited
nothing from Arabic *m-d-d*.

**Wiring.** `HI-PATH-029` is a third `SPINE-SAY-WHAT-I-DO` segment carrying the
eight, and the eight concepts drop out of that node's `omits` (38 → 30). One
pre-existing hole had to be closed first: **`HI-C05-bolta-hun` — the lesson that
teaches the present habitual every one of these verbs runs on — was on no
realization path at all**, so naming it as a prerequisite produced a
`curriculum-prerequisite-omitted` error. It now sits in `HI-PATH-028` beside
*bolnā*, carried by the new `HI-EXT-028-LANGUAGE-SPECIFIC` extension. No lesson
moved relative to another. `chapters.json`, `core/book-generation.json`,
`book.tex`, the generated chapter TeX and the ch34/ch35 narration all follow.

All eight lessons are schema v2, computed at **258–297 s** against the 300 s
ceiling, and both chapters are **fully drivable** (`drivablePrefix` 4 of 4):
the four inline-letter sections derive as `sight` for the book and detach
cleanly, so `coreModality` stays `voice`. The book compiles under XeLaTeX at
124 pages with **zero** missing characters and zero errors; the single overfull
and single underfull box both pre-date these chapters.

Corpus snapshot tests are deliberately left failing rather than re-pinned:
book chapters 399 → 401, declared chapters 301 → 303, lessons 1249 → 1257,
A2 lessons 153 → 162, atoms taught 1519 → 1536, `universallyMissing` holds at
15 (all eight were already taught elsewhere), `meanCoveredPercent` 17 → 18,
`payoffsNotClosed` 0 and `payoffsNotRepresentative` 24 both unmoved. Hindi goes
**4 → 12 of 40** core verbs.

