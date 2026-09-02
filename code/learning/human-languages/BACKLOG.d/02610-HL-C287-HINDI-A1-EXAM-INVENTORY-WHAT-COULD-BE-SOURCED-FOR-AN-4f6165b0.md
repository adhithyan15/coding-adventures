## HL-C287 - Hindi A1 exam inventory: what could be sourced for an Indic track, and what could not

Hindi is the first track outside Spanish, French and German to get an
`exam-inventory`, and the first Indic track to have its exam gap MEASURED
rather than proxied. Recording the research so the other nineteen tracks
inherit the method instead of repeating it.

### 1. No Hindi awarding body publishes a content syllabus. Checked, not assumed

`core/exam-levels.json` maps Hindi A1 to the **Dakshina Bharat Hindi Prachar
Sabha** "Prathmic" rung, `basis: editorial`. That mapping is the strongest
external anchor any Indic track has, so it was the obvious place to look for a
sourced A1 list. It does not contain one.

- `dbhpscentral.org/Syllabus.html` and `dbhpscentral.org/examination.html`
  publish the **names** of the examinations (Parichaya, Prathamic, Madhyama,
  Rashtrabhasha, Praveshika, Rashtrabhasha Visharad, Rashtrabhasha Praveen)
  and the prescribed readers. No grammar inventory, no word list, no can-do
  descriptors, **no A1/A2 column**.
- **Kendriya Hindi Sansthan** (Agra) and the **Central Hindi Training
  Institute** publish prospectuses and course names, not level-split content
  inventories.
- There is **no Council of Europe Reference Level Description for Hindi**. RLDs
  are written by national teams, and no team has written one.

So the Spanish pattern - restate a published, finite, level-split inventory in
our own words - is **not available** for Hindi, and by extension is unlikely to
be available for any of the other Indic tracks, none of which even has a named
external ladder. Do not spend another agent-hour looking for a South Asian
PCIC. It is not there.

### 2. What replaces it: the project's own checked-in A1 contract

The bounding sources that ARE finite, dated and checkable:

1. **The CEFR Companion Volume A1 descriptors.** Published, finite,
   level-split - but language-neutral. It says a candidate can ask and answer
   questions about personal details; it never says which Hindi words do that.
   Every exponent is ours.
2. **`hindi/assessment-spec.md`, `hindi/task-shapes/a1.json`, and the two timed
   mocks in `hindi/mocks/a1/`.** These ARE Hindi-specific, ARE finite, and ARE
   what a learner is actually scored against. We wrote them - but they were
   written before this inventory and without it in view, so they are evidence
   rather than circular reasoning.

**The mocks turned out to be the single most useful artifact in the repository
for this job.** Every stimulus in them is a construction a candidate must
handle, written down and dated. Reading both mocks item by item and enumerating
the construction each one requires produced most of this file's grammar
column - and, more importantly, produced *citable* gap notes: not "the corpus
probably needs `se`", but "mock 1 reading item 8, mock 1 listening item 10 and
mock 2 item 13 all turn on `se`, and no lesson introduces it."

**Method for the next nineteen tracks:** if the track has mocks, mine the mocks
first. A gap note that cites a numbered mock item is checkable in one step by
someone who does not speak the language.

### 3. Every dimension is `partial`, and that is the honest answer

All four `scope` dimensions are `partial`, so `isExamInventoryComplete` is
false and the `exam-inventory` backlog item is correctly NOT suppressed. This
matches French and German, which are also all-partial; Spanish is the outlier,
not the norm. A track with no published syllabus **cannot** honestly declare a
complete dimension, and a partial inventory that claims completeness is worse
than no inventory at all.

### 4. Probe ids were validated mechanically, not by eye

A probe naming an atom that does not exist reports "not covered" forever and is
indistinguishable from real debt. Every atom in every probe here was checked
against the set the `hindi` track actually introduces (357 atoms across 286
lessons), by a generator that refuses to emit an unknown id. **Do this for
every track.** The alternative is an inventory that quietly measures nothing.

The corollary is the one that shapes the file: **a point the corpus cannot
demonstrate gets `probe: null`, never a plausible-looking invented id.** 56 of
172 points here are null, and each carries a note saying what is missing.

### 5. The result, and the two findings worth acting on

**Hindi covers 116 of 172 A1 points (67%).** Strong where the track has
depth - communicative functions 31/40, core lexis 21/25, script 14/19 - and
weak exactly where a beginner course stops being a phrasebook:

    1/5   Vakya (the sentence)
    3/9   Parsarg (postpositions and case)
    9/19  Kriya (the verb)

Two findings deserve separate work:

- **`HI-A1-S-04` and `HI-A1-S-05` are unprobeable for a fixable reason.**
  `HI-W06-name-sentence-stop` teaches the danda and `HI-W06-two-sentence-card`
  teaches the two-sentence message, and both declare an EMPTY `introduces`
  list, so they contribute no atom. The teaching exists; the knowledge contract
  does not. Fixing one frontmatter field per lesson would make two points
  measurable without writing any content. Eight Hindi writing lessons are in
  this state.
- **The nuqta is taught nowhere and used everywhere.** The corpus's own
  headwords carry it constantly - shukriya, khushi, zarur, safed, darvaza, mez,
  sabzi, zyada, safar, tohfa, bukhar, fasal, khushbu, mulaqat - and no lesson
  ever explains the mark. This is the kind of gap only an external target list
  surfaces, because every corpus-internal metric counts those lessons as taught.

### 6. Conventions the next nineteen tracks should copy

1. Say in `about` what KIND of claim the file is, in the first sentence, and
   name what you checked and did not find. An `editorial` basis stated plainly
   is honest; an `editorial` basis dressed as an awarding body's syllabus is
   not.
2. Never attribute anything to a body that did not publish it. This file says
   NOTHING IN THIS FILE MAY BE ATTRIBUTED TO DBHPS, in those words.
3. `partial` everywhere until a dimension is genuinely closed by a source.
4. Validate every probe id against the corpus mechanically.
5. `probe: null` + a note naming the missing piece, and citing the mock item
   that demands it wherever one does.
6. Put the level's own demands - not the corpus's shape - in the point list. A
   point that exists only because the corpus happens to teach it measures
   nothing.
