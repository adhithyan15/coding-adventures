## Chapters 36–39 — the pre-A1 noun tranche, and what it measured

Sixteen everyday nouns, four chapters of four lessons. This wave was authored as
a **measurement probe** as much as content: `src/level-gate.ts` reported that no
track had attained even pre-A1, and that Hindi's binding blockers were
`vocabulary` and `reinforcement`. The question was whether track-local lessons
attached to pre-A1 spine nodes through the `HI-EXT-*` mechanism actually move
those numbers.

**They do.** Measured with `buildCurriculumGapReport` before and after:

| `levelGate.tracks[hindi]` | before | after |
|---|---|---|
| `vocabulary` (whole track, any level) | 70 | 86 |
| headwords at or below pre-A1 | 34 | 50 |
| `vocabulary` shortfall (target 300) | 266 | 250 |
| `reinforcement` shortfall | 39 | **4** |
| `attained` / `inProgressAt` | null / pre-A1 | null / pre-A1 |

The vocabulary criterion counts **distinct headwords on `word`/`phrase` lessons
whose spine node sits at or below the level**. Sixteen new `word` lessons in
path segments pointing at pre-A1 nodes therefore moved it by exactly sixteen —
one per lesson, no more. That is the honest exchange rate: **closing pre-A1 on
vocabulary alone needs another 250 lessons of this shape**, and nothing about
the mechanism makes it cheaper. Lesson count, not authoring cleverness, is the
whole cost.

The reinforcement number is where the leverage was. All **39** pre-A1 atoms the
continuity ledger reported as revisited fewer than twice are now practised at
least twice — the yes/no words, कृपया and माफ़ कीजिए, the family and body
lessons, पानी/रोटी, शुभ रात्रि, and the entire W01–W05 writing series, rescued
inside "The letters in this word" sections that also carry their own word. The
remaining 4 are atoms introduced by these chapters' own final lessons, which no
later lesson exists to revisit.

**Where the four chapters hang, and what would not hang honestly.** The seven
pre-A1 spine nodes are all **speech acts** — greet, thank, respond, exchange
names, check wellbeing, request, take leave. Four of them take nouns without
strain: MEET-GREET holds what you are received into and offered (घर, दरवाज़ा,
कमरा, कुर्सी); POLITE-REQUEST-REPAIR holds what you ask for (चाय, दूध, खाना,
किताब); CHECK-WELLBEING already held सिर and हाथ, so body words extend it
(आँख, दाँत, पैर, पेट); EXCHANGE-NAMES holds the people you introduce (दोस्त,
बच्चा, आदमी, औरत). **There is no pre-A1 node for naming a thing in front of
you.** मेज़ (table), खिड़की (window), सड़क (road), गाड़ी (vehicle) and कागज़
(paper) were on the shortlist and were dropped rather than forced: no honest
reading of a greeting-or-request node covers them. If pre-A1 is to reach 300
headwords, the spine needs a node for plain naming. That is a finding about the
spine, not about the words.

Not duplicated: पानी and रोटी (Ch. 15), नाम (Ch. 2), पिता/माता and भाई/बहन
(Ch. 12) were already taught, so the brief's suggested list shrank by six before
authoring began. माँ was dropped for the same reason — Ch. 12 covers "mother".

**Gender is the atom, not a footnote.** Every lesson teaches its noun's gender
where the noun is taught, because Hindi gender is not recoverable from meaning
and only partly from shape. The chapters build and then break the rule of thumb
deliberately: Ch. 36 establishes long **-ā** masculine / long **-ī** feminine
and shows घर outside it; Ch. 37 breaks it with **चाय** and **किताब**, feminine
with no **-ī** to show for it; Ch. 38 sets **आँख** (f.) against **दाँत** (m.),
identical in shape; Ch. 39 breaks it the other way with **आदमी**, masculine
*with* an **-ī**, and shows **दोस्त** taking its possessive from the person
rather than the word. The thread is carried by **मेरा / मेरी**, and it finally
keeps the promise Ch. 2 made in writing: *"You'll meet merī at your first
feminine noun."*

**Corrections made during authoring, all against sources.** Several attractive
etymologies did not survive checking and are recorded here so they are not
reintroduced: गृह is **not** from √*grah* "to seize" (that is a grammarians'
root-assignment; the real root is \**gʰerdʰ-* "enclosure"); पैर is **not** from
*pāda* (पाँव is); पेट is **not** demonstrably from *peṭaka* "basket" (Turner
notes a resemblance and derives nothing; the mainstream account is a Dravidian
loan); Persian *bacca* is a **sister** of Sanskrit *vatsa*, not a descendant;
the Hebrew *ʾādām* / *ʾadāmāh* "ground" link is Genesis making a deliberate pun,
not the etymology; the Greek cognate of अक्षि is *ósse*, **not** *ophthalmós*;
English *dough* is unrelated to दूध (*doughty* is the real cousin); and the
tea *chá*/*tê* split is about which Chinese **port** a trader used, not overland
versus sea, since Portuguese *chá* came by sea. One usage correction too:
**कृपया** is written register — signage and announcements — and **एक चाय
दीजिए** is what is actually said, so the chapter says so.

**Fonts.** Latin Modern has no Greek, Hebrew or Han. The first draft quoted
Greek *odoús* and *ósse*, Hebrew *ʾādām* and *ʾadāmāh*, and the Chinese
character for tea in their own scripts; a XeLaTeX run reported **28** dropped
glyphs against a track baseline of **zero**. All are now romanization only. The
39-chapter book compiles to 164 pages with **zero** missing characters.

**Gates.** Zero chapter-gate findings for 36–39; each payoff assesses every atom
its own chapter introduces plus its reach-back list. Zero duration violations
(every lesson computes under 300s). Zero new atom-budget violations — the one
Hindi ramp violation, `HI-C22-gyarah-bees`, pre-dates this branch. Zero
script-ramp violations. `attained` is still `null` and honestly so: pre-A1 is
250 headwords away.

