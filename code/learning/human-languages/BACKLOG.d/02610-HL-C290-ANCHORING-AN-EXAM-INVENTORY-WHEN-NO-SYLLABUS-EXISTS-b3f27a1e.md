## HL-C290 — Anchoring an exam inventory when no external syllabus exists

Twenty of the twenty-three tracks are `basis: editorial` in `core/exam-levels.json`
and have no awarding body to restate. **And the search for one is settled: stop
looking.** The parallel Hindi tranche ran the sourcing question against the
best-placed South Asian candidate and came back empty — DBHPS publishes the
*names* of its examinations and the prescribed readers and no content syllabus,
no grammar inventory, no word list, no can-do descriptors, no A1/A2 split;
Kendriya Hindi Sansthan and the Central Hindi Training Institute publish none;
and the Council of Europe has issued no Reference Level Description for Hindi.
Hindi was the only one of the twenty with a *named* external ladder, so that
negative result generalises. **There is no South Asian PCIC.** Do not spend
agent-hours on a Marathi, Tamil or Bengali syllabus hunt.

What follows from that is *not* "write points from CEFR descriptors" — that was
the first conclusion and the next section supersedes it. The file must, however,
still disclaim every body it might be mistaken for, in its first sentence: Hindi's
says "NOTHING IN THIS FILE MAY BE ATTRIBUTED TO DBHPS", Marathi's names the
Maharashtra Directorate of Languages the same way, and — because it derives from
the Spanish set — DELE and the Instituto Cervantes as well.

`measureExamCoverage` has always been generic; what blocked these tracks was that
writing a target list for them means writing down a judgement, and a judgement
dressed as a standard is worse than no list at all. This is the method the first
one used. It is meant to be copied.

### Borrow a LEVEL from a sourced track; do not go straight to bare descriptors

**This supersedes the obvious move, and it is the single most important thing in
this entry.** Faced with "nothing is sourceable", the natural response is to
write points from the CEFR Companion Volume's A1 descriptors. Hindi did that and
got 172 points. Marathi did it too and got 131. Both were wrong in the same
direction, and the size of the error is measurable: rebuilding Marathi's list
from `core/exam-inventory-es-a1.json` produced **301**.

`exam-inventory-es-a1.json` is DELE/PCIC-sourced — Spanish is the only track here
whose A1 set restates a real awarding body's *published* inventory — so its 273
points are an **attributable** statement of what an A1 learner must handle. Walk
them and ask, of each, what it DEMANDS of a learner rather than what Spanish
grammar it names:

- *"give personal information: name, age, origin"*, *"negate a statement"*,
  *"ask a price"*, *"proper nouns: personal names, forms of address, place
  names"* — language-neutral demands, transfer directly.
- *"definite article el/la/los/las"* — does not transfer; Marathi has no article
  system. But the demand behind *"absence of the article: bare nouns"* does.
- Devanagari orthography, the postposition system, the ergative, the
  gender-marked present — no Spanish counterpart at all. **A proxy is a scaffold,
  not a template to translate.**

What the descriptor route missed, and the proxy caught, was almost entirely
LEXICAL DOMAINS: the Spanish source publishes twenty thematic areas, and an
editorially-chosen list had simply never thought of education, work, leisure,
media, housing, services, shopping, health, travel, money, government, the arts,
religion or the natural world. It also caught the demonstratives, the relative
clause, coordination, subordination, object marking, the exclamative, and eight
punctuation and symbol points.

Record the mapping per point in `derivedFrom`, and **assert the derivation is
total in both directions**: every source point either derives into some target
point or is listed in `proxy.notTransferred` with a reason, and every
`derivedFrom` id must exist. That test is what caught `A1-O1-06` going missing
from Marathi's walk. A point that is silently absent is indistinguishable from
one nobody thought of, which is the failure the whole exercise exists to prevent.

Marathi's result: **301 points, 273 derived from Spanish, 28 Marathi-specific;
266 of Spanish's 273 accounted for, 7 explicitly dropped** (three article points,
two capitalisation points — Devanagari is unicase — and two written-accent
points).

### The four-kind anchor rule

Every point carries `anchorIds` into a top-level `anchors` array, and every
anchor declares a `kind`. A reader is expected to read the kind before the point:

- **`sourced-proxy`** — another track's inventory that IS attributable, used for
  the level and never for the language. The disclaimer has to be explicit and in
  the `about`: *"NOTHING IN THIS FILE MAY BE ATTRIBUTED TO DELE, THE INSTITUTO
  CERVANTES OR THE PLAN CURRICULAR ABOUT MARATHI"*, alongside the one naming the
  local body — Marathi's is the Maharashtra Directorate of Languages, Hindi's is
  DBHPS. "Derived from the DELE A1 inventory" is one careless edit away from
  reading as "DELE says this about Marathi", so a test asserts both lines.
- **`external-framework`** — public, third-party, and *language-neutral*. For A1
  that is the Council of Europe's CEFR Companion Volume. It can anchor a
  FUNCTION ("can fill in a form with personal details") and it can never anchor a
  Marathi form, because it names none. Anything claiming otherwise is a
  smuggled editorial decision.
- **`project-owned`** — a file checked into this repository: `core/spine.d`,
  `core/assessment-policy.json`, the track's `assessment-spec.md`, its
  `task-shapes/<level>.json`. Finite and auditable, so far better than nothing —
  but it is our own work, and agreeing with it is not corroboration. Saying
  "project-owned" instead of "sourced" is the whole point of the distinction.
- **`editorial`** — the project's unaided judgement. Named as such, loudly, and
  described as a working default to be corrected.

The shape mirrors the `sources` / `sourceIds` convention `task-shapes/a1.json`
already uses, so it is not a new idea, only a stricter one: `kind` is the field
that convention lacked.

### Mine the mocks first; if there are none, say what you used instead

Hindi's single most useful artifact was its checked-in timed **mocks**. Every
mock stimulus is a construction a candidate must handle, written down before the
inventory and without it in view, so it yields gap notes anyone can verify in one
step: not *"probably needs X"* but *"mock 1 items 8 and 10 and mock 2 item 13 all
turn on X."* Mine them before anything else.

Marathi has no mocks — `assessment.json` names `mocks/a1/*` as required future
artifacts and none exist — so it used the next-best written-down statement of
what a candidate will be handed, and **said so in `source` under a NOTE ON
METHOD heading**. An unstated substitution is an unauditable one.

### The strongest anchor available to a track without mocks is its own task shapes

`marathi/task-shapes/a1.json` enumerates eleven paper
parts with genres, item counts, stimulus lengths and response modes. That is a
*closed, finite* list of things a candidate will literally be handed, and it
answers "does A1 need this?" far more sharply than any intuition about levels:
the speaking paper's third part is a transactional role-play, therefore A1 needs
wanting, prices, a productive request form and a polar question — four points,
each independently justified, none of them a guess about a syllabus.

So the method is: **derive the point list from the task envelope backwards, not
from a grammar forwards.** A grammar walked forwards produces a plausible list
nobody can defend the boundary of. The envelope walked backwards produces a list
where every entry can name the paper part that demands it.

Reference grammars still get cited — for Marathi, Berntsen and Nimbkar's *A
Marathi Reference Grammar* and Pandharipande's *Marathi* — but only for what they
actually supply: a description of the structure. They assign no levels, and the
file says so.

### Probes name only atoms that EXIST. No exceptions, and this is not obvious

Every uncovered point is `probe: null` plus a note, never a probe pointing at an
id somebody expects a future lesson to introduce. The tempting alternative — "I
know the author will call it `MR-LEX-AAI`, so probe it and let it flip when
written" — fails silently and permanently: a missed suffix resolves to "not
introduced" forever, and sits in the report indistinguishable from the honest
gaps around it. All four pre-existing inventories already obey this (measured:
`partial` is 0 for Spanish, French, German A1 and German A2), and
`exam-inventory.test.ts` now asserts it per track.

The cost is real and worth stating: a point half-covered by the corpus reads as
zero. That is why every null carries a note naming precisely what is present and
what is missing — `A1-OR-05` says four of the five retroflex letters are taught
and DDHA is not. The note carries the partial credit the score refuses to.

### A null is TWO different findings, and conflating them costs an author a chapter

The one that nearly went out wrong. A probe reads *declared* atoms. 26 of
Marathi's 205 lessons — the whole of chapters 9 to 12 — are schema-v1 and declare
none, while teaching मी, तू/तुम्ही, माझं, काय, कसा/कशी/कसं, *tumchaṁ nāv kāy
āhe?*, *tumhī kase āhāt?*, काम करणे and राहणे. Every one of those scores zero.

The first draft of this file recorded "the interrogatives are untaught" and "not
one personal pronoun is taught". Both are false, and both would have queued an
author to write chapter 9 a second time. Grepping the *headwords* rather than the
declared atoms is what caught it, and that check should be routine:

```
grep -h '^headword:' marathi/lessons/*.md | grep '<the thing you are about to call missing>'
```

So every null now says which of two things it is:

- **CONTENT gap** — nobody has taught it. Real authoring work.
- **SCHEMA-V1 MEASUREMENT GAP** — it is taught and cannot be measured. The work
  is to give an existing lesson an atom, which is an order of magnitude cheaper
  and closes exam points immediately.

Eight of Marathi's seventy-six nulls are the second kind, and they cluster
exactly where an exam hurts most: the pronouns, two of the wh-words, and both
halves of the two A1 exchanges (*what is your name*, *how are you*). Any track
with a schema-v1 band will have the same shape, so **check the schema version
distribution before believing the score**:

```
grep -L 'schema_version: 2' <track>/lessons/*.md | wc -l
```

The corollary for the reported number: **coverage on a track with schema-v1
lessons is a floor, not an estimate**, and the inventory should say so where a
reader will see it. Marathi's `about` and `probeSemantics` both do.

### The second kind of measurement gap hides inside schema-v2

The schema-v1 band above is easy to find once you know to look. The dangerous one
is not. **A schema-v2 lesson can declare `introduces: []` and still teach
something new**, and then it looks fully annotated to every tool.

Most empty-introduces lessons are correct — a retrieval lesson *should* introduce
nothing, and `ramp.ts` explicitly blesses the explicit-empty contract. Marathi has
66 of them and 62 are honest. The four that are not:

- `MR-R22-request-verbs` drills **द्या, प्या, आणा, ठेवा** — four polite
  imperatives — while practising only the *infinitive* atoms it reviews.
- `MR-R23-wellbeing-verbs` adds **बसा** and the future **चालेल** the same way.
- `MR-A1M17-guided-message` and `MR-A1M18-independent-message` teach the guided
  32-word and the independent named-reader message — **the A1 writing paper's
  entire second task** — and declare nothing at all.

Hindi found the identical class: eight writing lessons, one of them teaching the
30–40 word message worth **60 of the writing paper's 100 points**. Two tracks,
one shape, discovered independently. The test to run before writing any note that
says "untaught":

```
# v2 lessons that declare no introductions — then read the ones whose `type`
# is writing, or whose headword is a FORM rather than a lemma
grep -l 'schema_version: 2' <track>/lessons/*.md \
  | xargs grep -l -A1 'introduces:' | ...   # see marathi-exam-gen.py in the PR
```

The rule: **when a probe reports an atom missing, check whether the lesson exists
and declares nothing before concluding the content is absent.** Getting this
wrong queues an author to rewrite a chapter that is already written.

### `complete` is not reachable this way, and should not be faked

All four dimensions are `partial`, and each note says what would close it. Only
one of the four is cheap: `communicative-functions` closes by walking the CEFR
Companion Volume's scales and recording, per scale, whether an A1 descriptor
exists. `grammar` and `lexicon` cannot close without either an external Marathi
syllabus that does not exist or an explicitly reviewed project decision recorded
as one. Saying so in the file is more useful than a `complete: true` nobody
believes.

### Result, for calibration

Marathi A1: **301 points, 88 covered (29%), 213 unmapped, 0 partial.**
Hindi A1, for calibration: **172 points, 116 covered (67%)** — but Hindi's list
is descriptor-derived and Marathi's is proxy-derived, so the two denominators are
not comparable and **Hindi's should be rebuilt against the Spanish set before
anyone reads the two percentages side by side.** Hindi also holds ~155 headwords
to Marathi's ~48, which is most of the rest of the difference.

Marathi's own rebuild moved 55/131 → 88/301, and both halves are the result: the
numerator rose because the Spanish walk found taught material the editorial list
never asked about, and the denominator nearly trebled because it found the twenty
thematic domains above. The shape
is the finding and it differs from the two European editorial-adjacent tracks:
French and German are grammar-shaped with vocabulary strong, whereas Marathi's
strong columns are its script (12/20, after two tranches took closure 44 → 0) and
its script (**15/24**), while whole categories that carry an exam paper are at
zero: demonstratives 0/3, coordination 0/5, temporal notions 0/6, housing 0/4,
shopping 0/3, work 0/6, travel 0/3.

"Vocabulary is the blocker" survives contact with a target list, but it stops
being a headword count and becomes a shopping list — and the list splits three
ways, which a bare count could never have shown:

1. **Taught, unmeasurable** (11 points): the pronouns मी and तू/तुम्ही, the
   interrogatives काय and कसा, both A1 exchanges, five polite imperatives, and
   the 30-to-40-word message that is half the writing paper. Cost: annotate
   existing lessons.
2. **Genuinely absent vocabulary**: कोण, कुठे, कधी, किती, the third-person
   pronouns, every number above five, days, months, both parents, every plural,
   the ten Devanagari digits.
3. **Not vocabulary at all**: the oblique stem, which stands between every taught
   noun and every postposition, and the ergative -ने, without which a past
   transitive clause is ungrammatical. No quantity of headwords moves either.

### Ranked, for whoever writes the next editorial inventory

0. **Derive the point set from `core/exam-inventory-es-a1.json`, the only
   DELE-sourced list here, and record `derivedFrom` per point.** Assert the
   derivation is total both ways. Do not start from bare CEFR descriptors: on
   Marathi that route produced 131 points against the proxy's 301, and the
   difference was almost entirely lexical domains nobody remembered to include.
1. Read the track's `task-shapes/<level>.json` and use it to settle whether a
   demand the Spanish set raises really applies to this book's own exam.
2. Enumerate the shared spine's nodes at and below the level; the track's own
   `curriculum.d/spine/*.json` `omits` arrays name the concepts it has not
   realized — useful corroboration, but **verify against the introduced-atom set
   before quoting one**, because several Marathi `omits` entries are stale in
   both directions.
3. Write probes only for atoms you have grepped out of the corpus.
4. Give every null a note that names what IS present, or the score lies by
   omission even while being technically correct.
5. Before writing a single "untaught" note, count the track's schema-v1 lessons,
   list its schema-v2 lessons with `introduces: []`, and grep its headwords. On
   Marathi that check moved thirteen points out of the authoring queue and into a
   much cheaper annotation queue.
6. **Hindi's own inventory should be rebuilt against the Spanish proxy** before
   its 67% is compared with anybody's. It is descriptor-derived; the denominators
   are not the same object.
