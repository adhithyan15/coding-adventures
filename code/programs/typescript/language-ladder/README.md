# Language Ladder

Live app: <https://adhithyan15.github.io/coding-adventures/language-ladder/>

**The HL03 unified curriculum learning app** (it began life as the HL02
`script-writing-visualizer` and has subsumed that app's modes). Five modes.
**Learn** (the default) walks each selected language's validated local path —
one prerequisite-safe micro-lesson at a time — and admits a lesson to mixed
review only after focused retrieval in that language. Shared-spine abilities
and grounded roots still show where independently ready paths connect.
**Browse** and **Practice** work on *script letters*;
**Lessons** drills the *written curriculum* — every lesson in every track — on a
spaced-repetition schedule that persists between visits; and **Concepts** shows
one idea in every language that has it, side by side.

The production app keeps lesson Markdown lazy. Learn fetches only the small
track-local batches containing completed and current-frontier lessons; opening
Lessons or Concepts opts into all 278 batches rather than 1,669 individual
lesson requests. The handwriting model, SVG renderer, and font parser share one
independently cacheable eager chunk, leaving the interactive shell room for later
source-backed paths without making startup asynchronous. BUILD requires that
split and checks both the request ceiling and the unchanged 500,000-byte eager
chunk budget.

## Learn mode (the curriculum session)

The spine of the app is [HL03](../../../specs/HL03-unified-language-learning-app.md)
plus the stricter [HL04](../../../specs/HL04-shared-spine-and-content-pipeline.md)
progression contract. `curriculum.ts` loads every active `curriculum.json` map and
the pure frontier planner returns exactly one safe next lesson per selected
language. A language advances independently; paths are grouped only when their
current lessons share a spine ability. The picker includes every track,
including Russian, Persian, and Urdu, and reports the exact mapped lesson and
extension totals for the mix.

Each frontier card shows the target form, romanization, etymology hook, complete
authored Markdown micro-lesson, shared can-do, local `N of M` position, and any
typed script/grammar/register/etymology extension attached at that point.
Grounded root connections are shown only among languages simultaneously ready
at the same shared ability. Script notes come from explicit local script
extensions and the canonical script data, so Persian and Urdu keep distinct
identities and no global concept cursor guesses where a script belongs.

Before advancing, the learner starts a **focused check**. When a canonical block
has an `hl-activity` contract, the app uses its authored prompt, normalized answer
variants, corrective feedback, and response budget without scraping prose or
showing an answer-bearing summary. Other lexical lessons ask for one English
meaning. A wrong answer leaves the local frontier unchanged; a correct answer
shows feedback before the learner continues. The first objective non-lexical
pilots covered Spanish grammatical gender and punctuation spans; the first
HL-A01 tranches now reach 25 of 119 mapped non-lexical lessons across 18 tracks,
including script, grammar, etymology, culture, and cumulative practice. The
remaining 94 support lessons retain temporary final-recall confirmation while
HL-A01 fills the measured contract backlog.
One successful check completes exactly the current frontier lesson.
`learnprogress.ts` persists stable lesson IDs independently per language and,
on load, keeps only the longest valid local prefix. A newly inserted prerequisite
therefore becomes the frontier instead of being skipped by stale saved state.

Below the frontiers, the randomised SRS review draws only from independently
focused-successful shared lessons. It waits until at least two visually distinct
answers are eligible, then asks *"‹meaning› — in ‹language›?"* with options from
that safe grid. A cross-language comparison can appear only after both local
realizations have passed their own check. Answers still flow through
`applyAnswer`; misses are demoted and recorded for *"what you keep confusing"*.
`reviewstore.ts` persists that SRS state and answer log separately from local
path completion.

A quiet **"Reset progress"** control at the foot of the Learn view clears it all,
including the saved language mix,
(`reset.ts` — only the keys this app owns), behind a two-click confirm so a stray
tap can't wipe everything.

## Concepts mode

The curriculum tags lessons with a shared `concept_tag`, and canonical tags are
deliberately the same across tracks. That makes them a **join key**: *gracias /
merci / danke / धन्यवाद / നന്ദി* are one concept realized eighteen ways.

This mode is that join, and it is the data package's own
`languagesForConcept` — a function shipped from the start, documented as "what
the companion app calls," which until now had **no caller**.

- **42 concepts are shared by two or more tracks**, from 1,066 lessons. A concept
  only one language tags is filtered out: there is nothing to compare it with,
  which also removes almost every namespaced (`ES-…`) tag without a special case.
- Each row shows the **headword**, a **romanization** where it differs, and the
  gloss — so a non-Latin script is legible next to a Latin one.
- The **etymology hooks** sit underneath the comparison, where they do the most
  work: *gracias* ← *gratia* "favour", *merci* ← *mercēs* "wages", *danke* ←
  *denken* "to think", *спасибо* ← *спаси Бог* "God save you". Four languages'
  words for the same courtesy, from four unrelated ideas.

## Lessons mode

Reads all **1,066 lessons across 20 languages** straight from the curriculum via
`@coding-adventures/human-language-data`, and schedules them with the same
Leitner machinery the letter drills use (`scheduler.ts` is generic over an
index; it never needed to know what an item is).

- **It remembers you.** Progress is saved to `localStorage` keyed by **lesson
  id** — never by array index, because indices shift every time a lesson is
  added and saving by position would reattribute your history to the wrong
  lesson. New lessons simply appear as unseen items.
- **It uses the same authored content as the books.** Reveal opens the complete
  Markdown micro-lesson rather than discarding its explanations and practice.
- **It mixes the languages you selected.** Consecutive reviews round-robin across tracks — Arabic,
  then Bengali, then French — rather than marching through one language. That
  interleaving is deliberate: it forces discrimination instead of coasting.
- **Recall, not recognition.** You see the headword in its own script; the
  meaning stays hidden until you ask for it, then you grade yourself
  *Again* / *Got it*.
- Each card also surfaces what the lesson **revisits** (`reviews_of`), the
  curriculum's own review graph.

To clear your progress, delete the `hl-study:progress:v1` key in localStorage.

## Browse & Practice (script letters)

The Human Languages curriculum teaches a non-Latin script *inline* — a letter is
introduced inside the first word that needs it. These modes are the other half
of that promise: they **break each letter apart** into its pieces and show a
**stroke order**, so you can practise *writing it on paper*.

Pick a script, pick a letter, and the detail panel shows:

- the **glyph**, big, with its sound and role;
- **Break it apart** — the letter's component pieces (the "tall joined upper
  loop + rounded lower bowl" of handwritten Cyrillic *в*);
- **Write it** — a conventional stroke order, numbered; and for letters with an
  authored pen path, the **stroke-order filmstrip** below;
- a **⚠ false friend** badge for letters that look like a Latin letter but
  aren't (Cyrillic *в*=v, *р*=r, *с*=s, *н*=n) — the fastest way into the script.

### The stroke-order filmstrip

A numbered list tells you *what* to draw. It does not tell you where the pen
starts, which way it travels, or — the thing a picture of the finished letter
can never show — where the hand **lifts**. For letters that have an authored
pen path, "Write it" becomes a strip of panels instead, each one showing the
letter a little further written:

```
┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐  ┌────────┐
│ ▏      │  │ ▏      │  │ ▏    ▕ │  │ ▏ ⌒  ▕ │  │ ▏ ⌒▕ ▕ │
│ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │  │ ▁▁▁▁▁▁ │
└────────┘  └────────┘  └────────┘  └────────┘  └────────┘
 1. down     2. along    3. up the   4. over     5. down
 the left    the bottom  right side  the top     the middle
```

Behind every panel, in pale grey, sits the **finished letter — the outline read
straight out of the shipped font**, never a drawing of one. In front of it, in
ink, sits as much of the pen path as the hand has travelled so far, with a dot
showing where the pen is. Underneath sits the cited source for the stroke
*order*, because unlike the shape, the order is not something a font can vouch
for.

Three modules meet to make one picture:

| module | knows | checked by |
|---|---|---|
| `src/truetype.ts` | what the letter *looks like* — the real outline | the font itself |
| `src/strokes.ts` | how it is *written* — pen path, parts, lifts | `strokes.test.ts`: every point on real ink, every join < 2 font units, whole letter traced |
| `src/ductusview.ts` | how to *draw that* — SVG frames, no DOM | `ductusview.test.ts` |

Font units are **y-up**; SVG is **y-down**. The glyph and the pen path are both
in font units, and `ductusview.ts` flips them together with exactly **one**
`scale(1,-1)` group — so a mistake cannot leave a plausible-looking stroke
sitting upside down on a correct letter.

**All eleven starter Tamil letters — அ, ஆ, இ, க, ம, வ, ல, ற, ன, ண, and ந —,
all nine Persian starter letters — ا, ب, ت, س, ل, م, ن, ه, and و —, all
thirteen Urdu starter entries — ا, ج, ر, س, ش, ک, ل, م, ن, ں, ہ, ی, and ے —,
all eighteen Arabic starter letters — ا, ب, ت, ج, ح, خ, د, ر, س, ش, ص, ض, ع, ك, ل, ه, و, and ي —,
all twenty-two Hebrew starter letters א, ב, ג, ד, ה, ו, ז, ח, ט, י, כ, ל,
מ, נ, ס, ע, פ, צ, ק, ר, ש, and ת, all twenty-four Chinese starter entries 人, 亻, 口, 女, 子, 日, 讠, 氵, 宀, 你, 好, 我, 是, 不, 名, 字, 谢, 请, 再, 见, 什, 么, 早, and 上,
and Devanagari अ, आ, इ, ई, उ, ऊ, ए, ऐ, ओ, औ, and क
have authored pen paths today.**
`DUCTUS` admits no letter without a citation
for its stroke order, and hand-drawing a letter is forbidden
outright (a subtly wrong Tamil ண looks perfect to exactly the audience that
cannot yet read Tamil, so the error would ship *as the lesson*). அ and ஆ exercise
real two-stroke paths with one lift; ஆ keeps its upright and long-vowel loop in
the same pen-down run. இ keeps five inner-and-lower movements together, lifts
once, then joins its outer-left climb to the final arch; ம remains one unbroken
stroke. க exercises a real three-stroke path: its upper frame and two lower bowls
are separated by two verified lifts. வ joins its spiral body, bottom bar, and
right upright in one five-movement run with no lift. ல carries its outward
spiral through a middle descent and deep right-hand turn to the open tip in one
four-movement run with no lift. ற uses three pen-down runs: its left arch joins
the first middle descent, the adjacent descent restarts after one lift, and a
second lift precedes the right arch's joined sweep and descender. ன joins its
left spiral, single inner arch, and top bar through five movements before one
lift precedes its separate right upright. ண follows the same two-run pattern,
keeping its extra inner arch joined to the rest of the body and top bar before
the sole lift. ந uses three runs: its opening three movements stay joined, the
middle rise joins the top bar after one lift, and a second lift precedes its
right-hand descent and tail. Frame 12's looped handwritten form differs from
Noto's straighter typographic form, so the source records that adaptation while
the mechanical gates keep every authored point on the actual font. Persian ا is
the first right-to-left-script filmstrip: UT Austin's freehand lesson shows its
isolated Naskh stem descending in one continuous movement, and the same gates
fit that path to the vendored Noto Naskh outline. The adjacent ب filmstrip keeps
its shallow bowl in one right-to-left run, then shows the sourced lift before
the dot while retaining the completed bowl. The later ت filmstrip reuses that
bowl, then retains it through separate left-dot and right-dot runs with two
lifts. Its source note records the intervening Persian-added پ demonstration as
deferred inventory work. The later س filmstrip keeps its three teeth and final
bowl in one continuous right-to-left run, so its two learner movements show no
lift or completed-stroke overlay. The later ل filmstrip descends its tall
upright and turns directly into the leftward base curve in the same pen-down
run, again with two movements and no completed-stroke overlay. The
source-adjacent م filmstrip shapes its round head and
flows directly into the descending tail in the same pen-down run, with two
movements and no completed-stroke overlay. The adjacent ن filmstrip sweeps its
bowl right-to-left, then preserves that completed run during the single sourced
lift that places the dot above. The source then demonstrates و before ه; its
filmstrip shapes the small head loop and flows into the leftward curving tail
as one unbroken two-movement stroke. The later ه filmstrip keeps its isolated
looping body in one unbroken movement. Its source uses a simple closed
handwritten loop, while the checked learner path fits that same pen-down run to
Noto Naskh's wider two-counter form and leftward baseline finish. Urdu ا adds a
separate *Zer o Zabar* independent-form filmstrip: one top-to-bottom movement,
explicitly distinct from bottom-to-top final ـا. Script-aware lookup keeps its
Northwestern provenance separate from Persian ا even though both canonical
script files route their checked paths through Noto Naskh. Urdu ج adds a
three-frame independent-form filmstrip from the textbook's jīm chapter: the
below-dot comes first, one lift precedes a pointed hooked head, and the descent
and bowl continue in that same second run. Its source note also preserves the
flat-head alternative as purely aesthetic. Urdu ر adds a two-frame independent
filmstrip from the next chapter: the first movement descends, the second curves
left, and both remain in one uninterrupted zero-lift stroke. Its source note
keeps the distinct final-form motion and Naskh/Nastaliq contrast explicit. Urdu
س adds another two-frame independent filmstrip: its three close teeth flow
right-to-left directly into the final bowl in the same zero-lift run. The
script-scoped source keeps it distinct from Persian س and records the optional
long gentle curve as an especially common handwriting alternative. Urdu ش adds
a five-frame filmstrip from the same chapter: the two body movements stay in
one run, then three sourced lifts place the lower-left, lower-right, and centered
upper dots. Its source note preserves the two-below/one-above arrangement,
centers the dots above either the toothed or optional toothless body, and keeps
the standard learner path on Noto Naskh. Urdu ک adds a three-frame Chapter-1
filmstrip: the stem descends and flows left through the flatter bowl and
pronounced hook in one run; one sourced lift then starts the long upper-right
slash down toward the stem. Its source note preserves the explicit warning not
to collapse those two strokes while the learner path fits Noto Naskh's connected
outline. Urdu ل adds a two-frame Chapter-2 filmstrip: the pen starts at the top,
descends the tall independent upright, and stays down while the line passes
below the baseline through the leftward bowl and turns back up. Its source note
preserves the connector and final-bowl prose distinctions while the zero-lift
learner path follows the Noto Naskh fallback. Urdu م adds a two-frame Chapter-3
filmstrip: its round head flows directly into a tail below the baseline without
lifting. The source note preserves the textbook's distinction between
calligraphy and the constant-width handwritten counterclockwise loop while
their shared head-to-tail motion follows Noto Naskh. Urdu ن adds a two-frame
Chapter-6 filmstrip: its independent bowl sweeps right-to-left below the
baseline before one sourced lift places the dot. The source note preserves the
near-baseline dot and distinct initial/medial tooth while the path follows Noto
Naskh. Urdu ہ adds a one-frame Chapter-4 filmstrip: the pen starts at the
independent teardrop's upper right, loops counterclockwise down and left around
the base, then returns up the right side and crosses the top without lifting.
The source note preserves the distinct initial/medial divot-and-mark forms and
the final up-and-down squiggle while the independent learner path follows Noto
Naskh. Urdu ی adds a two-frame Chapter-4 filmstrip: the pen starts at the upper
right, descends through the independent S curve, then stays down while it sweeps
left around the below-baseline bowl to its rising tip. The source note preserves
the two dots as an initial/medial feature that does not belong to independent
chhoṭī ye while the zero-lift learner path follows Noto Naskh. Urdu ں adds a
one-frame Chapter-6 filmstrip: the independent dotless bowl sweeps
right-to-left below the baseline in the same zero-lift run as ن. The source note
preserves the initial/medial ordinary-nūn forms, and Noto Naskh verifies that
U+06BA exactly shares U+0646's body contour with the dot removed. Urdu ے then
completes the starter inventory with a three-frame Chapter-4 filmstrip: its
upper-right descent sweeps left across the broad bowl, curls back underneath at
the far left, and continues right along the lower fold without lifting. The
source note preserves the distinct initial/medial tooth and independent/final
sound role while the learner path follows Noto Naskh's folded contour.
Arabic ا then adds its own one-frame filmstrip from the University of Oregon's
*Introduction to Arabic* video: independent alif descends top-to-bottom in one
continuous 00:05–00:07 movement with no lift. The adjacent one-way-connector
lesson context remains explicit, and script-aware lookup keeps the Arabic source
separate from the Persian and Urdu records while all three paths are checked
against the same vendored Noto Naskh outline. Arabic ب adds the adjacent
two-frame video demonstration: its shallow bowl sweeps continuously from the
upper-right tip to the turned-up left tip, then one lift precedes the dot below.
The two-way-connector context stays explicit, while script-aware lookup keeps
its Arabic source separate from Persian ب and both learner paths stay on the
same vendored Noto Naskh outline. Arabic ت then adds a three-frame path from the
book's dedicated ب/ت/ث page. Its Taa clip opens with the bowl already complete,
so the learner path cites the page's separately demonstrated Baa body for that
right-to-left sweep, then follows Taa's left and right upper dots as two
individually lifted strokes. The evidence split, two-way-connector context, and
Arabic-scoped provenance stay explicit while the path fits the same Noto Naskh
outline independently of Persian ت. The page's next link is labeled ث, but its
video visibly writes another two-dot ت; the audit records that source mismatch
and leaves ث on the conventional fallback. Arabic ج therefore becomes the next
verified path: its dedicated clip draws the short upper head left-to-right,
continues down and around the bowl in the same pen-down run, then lifts once for
the dot below. The resulting three-frame filmstrip stays on the isolated Noto
Naskh outline, keeps the lesson's two-way-connector context, and remains
script-scoped separately from Urdu's dot-first ج. The same page does not link
Haa in its body, but its WordPress attachment ledger exposes `Haa.mov`. That
clip opens while **ح**'s short left stem is underway, finishes the descender,
then lifts once and restarts near its top before sweeping continuously around
the dotless bowl. Its three-frame filmstrip keeps that stem-first evidence
distinct from Jeem's body-first order while fitting the isolated Noto Naskh
outline. The page's `kha.mov` verifies **خ** independently: its short upper head
travels left-to-right and continues around the bowl in one run, then one lift
precedes the dot above. That three-frame filmstrip follows Khaa's own body-first
evidence rather than copying Haa's restart or merely moving Jeem's dot. The next
page's `letter-daal-2.mp4` verifies **د** independently: its upper tip descends
down-right through the curved shoulder, then turns left along the baseline in
the same pen-down run. The two-frame filmstrip preserves that zero-lift motion,
one-way-connector context, and Arabic-scoped provenance while fitting the
isolated Noto Naskh outline. The same page's `raa.mp4` verifies **ر**
independently: its upper tip descends through the short stroke, then sweeps left
through the lower curve in the same pen-down run. The two-frame filmstrip
preserves that zero-lift motion, one-way-connector context, and Arabic-scoped
provenance independently of Urdu ر while fitting the same isolated Noto Naskh
outline. The next page's `FullSizeRender-8.mov` verifies **س** independently:
it shapes three close teeth right-to-left, then flows directly into the final
bowl in the same pen-down run. The two-frame filmstrip preserves that zero-lift
motion, two-way-connector context, and Arabic-scoped provenance independently
of Persian and Urdu س while fitting the same isolated Noto Naskh outline. The
page's `FullSizeRender-7.mov` then verifies **ش** independently: it draws the
same body in one run before separately placing the lower-left, lower-right, and
centered upper dots. The five-frame filmstrip preserves those three lifts,
two-way-connector context, and Arabic-scoped provenance independently of Urdu ش
while fitting the isolated Noto Naskh outline. The page's `FullSizeRender-6.mov`
then verifies **ص** independently: it closes the oval clockwise and rises into
the short shoulder in one run, then lifts once and restarts at the baseline
junction for the trailing bowl. The three-frame filmstrip preserves that
two-stroke order, two-way-connector context, and Arabic-scoped provenance while
fitting the isolated Noto Naskh outline. The page's embedded Daad lesson then
verifies **ض** independently at 00:43.1–00:46.3: it repeats those two body runs,
lifts a second time, and places the upper dot last. The four-frame filmstrip
preserves that three-stroke order while explicitly recording that the directly
linked short MOV returned HTTP 403 during the audit. The
next page's directly linked `ayn.mov` then verifies **ع** independently at
00:03.1–00:04.0: it shapes the open head from the upper-right tip and flows
directly down and around the lower bowl. The two-frame filmstrip preserves that
one-stroke, zero-lift order and keeps Ayn distinct from adjacent dotted Ghayn.
The `Alphabet ي ك ل` page's directly linked `kaf.mov` then verifies **ك**
independently at 00:11.8–00:13.4: its first run descends the main upright and
turns left along the baseline, then one lift precedes the inner arm drawn from
upper right down-left. The three-frame filmstrip preserves that two-stroke order
while keeping Arabic Kaf distinct from Urdu **ک**'s different Unicode glyph and
source-backed fallback path.
The same page's directly linked `lam.mov` verifies **ل** independently at
00:01.9–00:02.4: its tall upright descends directly into the leftward base bowl
without lifting. The two-frame filmstrip preserves that one-stroke order while
keeping Arabic Lam's provenance distinct from the Persian and Urdu records for
the same Unicode glyph.
The page's directly linked `yaa.mov` verifies independent **ي** at
00:33.2–00:35.0: descend and sweep left through the shallow bowl in one run,
then place the lower-left dot and the lower-right dot in separate runs. The
four-frame filmstrip preserves that three-stroke, two-lift order while keeping
Arabic Yaa U+064A distinct from Urdu Ye U+06CC, whose independent body has no
lower dots and its own source-backed provenance.
The next **ه و ي** page's directly linked `letter-haa.mov` verifies independent
**ه** at 00:04.9–00:06.0: it closes the lower counter, threads through the centre
into the upper-right counter, then sweeps left along the baseline without
lifting. The three-frame filmstrip preserves that one-stroke, zero-lift order,
fits the compact handwriting to the wider isolated Noto Naskh outline, and keeps
Arabic provenance separate from Persian **ه** for the same Unicode glyph.
The same page's directly linked `waw.mov` verifies independent **و** at
00:45.7–00:46.9: sweep left from the lower-right junction to close the small
head loop, then continue down and left through the tail without lifting. The
two-frame filmstrip preserves that one-stroke, zero-lift order, Waw's
one-way-connector and w/long-ū roles, and Arabic provenance distinct from
Persian **و** for the same Unicode glyph.
Hebrew **א** opens the next-smallest remaining inventory with a three-frame
filmstrip from HebrewPod101's dedicated Alef lesson: draw the main diagonal
down and right, lift once, then draw the opposing diagonal from the upper right
through the crossing and down the lower-left leg. The source's compact,
X-like handwritten form differs from Noto Sans Hebrew's block Alef, so the
variation note records that adaptation while the same geometry gates keep both
pen-down runs on the vendored outline.
The same lesson's second, block-style **ב** adds another three-frame filmstrip:
its top bar travels left-to-right and turns directly down the right side, then
one lift precedes the left-to-right baseline. The lesson places an optional
dagesh afterward, but base U+05D1's path and one-lift count correctly exclude
that separate sound-changing mark.
The series' dedicated Gimel/Dalet lesson adds a four-frame printed **ג**
filmstrip: its short top bar, right stem, and short lower-right leg stay in one
run, then one lift precedes the longer diagonal leg down-left. The lesson
explicitly contrasts that angular printed form with a rounded cursive Gimel,
so the source note preserves both while the learner path follows the vendored
Noto Sans Hebrew outline.
The same lesson's cursive **ד** adds a two-frame filmstrip: one broad arch
curls through a small lower loop and continues directly into its tail. The
instructor explicitly calls it "just one curve," so the learner path keeps that
zero-lift order while fitting the movement to Noto Sans Hebrew's angular top
bar, sharp right heel, and downstroke.
The dedicated Hei lesson adds a three-frame printed **ה** filmstrip: its top bar
travels left-to-right and turns directly down the right side, then one lift
precedes the detached left leg from top to bottom. The lesson explicitly asks
learners to keep the handwritten form curved but use sharp angles in print, so
the source note preserves that variation while the learner path follows the
vendored Noto Sans Hebrew outline.
The dedicated Vav lesson adds a two-frame printed **ו** filmstrip: its short head
travels left-to-right and turns directly into the top-to-bottom stem in one
unbroken stroke. The instructor explicitly calls Vav one stroke from top to
bottom; the source note preserves its simpler handwritten form and excludes the
lesson's later Hirik and Shuruk vowel marks from base U+05D5's zero-lift count.
The Zayin/Heit lesson adds a two-frame **ז** filmstrip from its rounded handwritten
demonstration: a short rightward rise continues down the outer curve and around
the base without lifting. The source calls that form handwritten Gimel's mirror
image and warns against collapsing Zayin into Vav; the learner path fits the
same continuous order to Noto Sans Hebrew's broader head and curved stem.
The same lesson adds a three-frame printed **ח** filmstrip: the left-to-right top
bar continues directly down the right side, then one lift precedes the joined
left leg from top to bottom. The source explicitly contrasts rounded handwriting
with sharper print, so the learner path follows the Noto Sans Hebrew block outline
while preserving the handwritten variation in its citation metadata.
The Tet/Yod lesson adds a four-frame printed **ט** filmstrip: the left side
descends into the rightward base, then one lift precedes the lower-right restart,
bottom-up right side, and inward hook. The source calls that bottom-up movement
unusual and shows the rounded handwritten form as one continuous run, so the
learner path keeps the print order on Noto Sans Hebrew without losing the variant.
The same lesson adds a two-frame printed **י** filmstrip: its tiny head travels
left-to-right and turns directly down through the short stem without lifting.
The instructor calls Yod the simplest letter and compares handwriting to a
little comma; the source note preserves that rounded form and print's small angle.
The dedicated Kaf lesson adds a three-frame printed **כ** filmstrip: its top bar
travels left-to-right, rounds down the right side, and turns left along the base
without lifting. The source calls handwriting half a circle to the right and
contrasts its rounded sweep with the same printed movement's sharp corners.
The Lamed/Mem lesson adds a three-frame printed **ל** filmstrip: its tall left
stroke descends to the middle junction, continues right along the bar, and turns
diagonally down-left without lifting. The source's earlier handwritten version
rounds the same one-run idea into a loop, which remains explicit in the citation.
The same lesson adds a five-frame printed **מ** filmstrip: its detached angled
left part comes first, then one lift precedes the joined upper shoulder, right
descent, and leftward base. The source's earlier handwriting
compresses Mem into a narrow N-like zigzag, which remains explicit in the citation.
Aural Writing's full-alphabet demonstration adds a three-frame printed **נ**
filmstrip: its small head travels left-to-right, continues down the right side,
and turns left along the base without lifting. The adjacent purple cursive Nun
rounds that hook, while the citation records why an expository video was rejected.
The same source adds a four-frame printed **ס** filmstrip: its flat top travels
left-to-right, rounds down the right side, sweeps left along the base, and climbs
the left side to close in one clockwise run. The adjacent purple cursive Samekh
keeps the zero-lift loop but rounds it into an oval.
Its printed **ע** demonstration adds a three-frame filmstrip: the right branch
descends into the base, sweeps left, then turns back to climb the left branch in
one run. The adjacent purple cursive Ayin compresses those branches into a loop.
Its printed **פ** demonstration adds a four-frame filmstrip: the top, right side,
and returning base stay joined, then one lift precedes the short inner curl. The
adjacent purple cursive Pe coils inward in one uninterrupted spiral.
Its printed **צ** demonstration adds a three-frame filmstrip: the long diagonal
turns left into the base, then one lift precedes the short upper-right arm. The
adjacent purple cursive Tsadi compresses those branches into one rounded run.
Its printed **ק** demonstration adds a three-frame filmstrip: the top bar turns
down-left through the right body, then one lift precedes the separate inner-left
stem descending below the writing line. The adjacent purple cursive Qof rounds
those parts into one hooked descent.
Its printed **ר** demonstration adds a two-frame filmstrip: the top bar travels
left-to-right, rounds the top-right corner, and continues down the right side in
one run. The adjacent purple cursive Resh keeps that zero-lift order in a rounder
hook.
Its printed **ש** demonstration adds a three-frame filmstrip: the right branch
descends and rounds left through the base into the climbing left branch, then one
lift precedes the middle branch's descent. The adjacent purple cursive Shin
compresses those parts into one rounded loop with a short rightward exit.
Its printed **ת** demonstration adds a four-frame filmstrip: the top bar travels
left-to-right and continues down the right side, then one lift precedes the
separate left leg and its small leftward foot. The adjacent purple cursive Tav
retraces its left stem and arches into the right side in one continuous run.
Hanzi Writer Data then opens Chinese with a two-frame **人** filmstrip: its
pinned, Make Me a Hanzi-derived PRC record orders the left-falling stroke before
the separately started right-falling stroke. The learner path fits those two
source medians to Noto Sans SC without changing their direction or one lift.
The adjacent two-frame **亻** filmstrip uses its own pinned record: a long
left-falling piě comes first, then one lift precedes the vertical shù from the
central junction to the baseline. Its narrow Noto Sans SC fit is independent
of the full 人 proportions.
The four-frame **口** filmstrip then introduces the joined héngzhé corner:
descend the left side, lift for a top bar that turns down the right side without
breaking, then lift and close the bottom left-to-right. The Noto Sans SC fit
keeps the source's three-run, close-last order explicit.
The four-frame **女** filmstrip begins with a different kind of join: descend
left, turn at the lower junction, and sweep down-right without lifting. One
lift precedes the separately left-falling piě, and a second precedes the middle
héng from left to right. The Noto Sans SC fit keeps all three pinned medians'
directions and the first stroke's internal turn explicit.
The five-frame **子** filmstrip follows with two different joined turns: the top
héng sweeps down-left without lifting, then a separately started central
descent hooks left at the base. A second lift precedes the middle héng from left
to right. The Noto Sans SC fit keeps the two hooked runs and final crossing in
the pinned source order.
The five-frame **日** filmstrip returns to the box pattern with an inside bar:
descend the left side, lift for a top bar that turns down the right side without
breaking, lift for the middle horizontal, then lift and close the bottom from
left to right. The Noto Sans SC fit keeps the joined corner and the pinned
inside-before-close order explicit.
The four-frame **讠** filmstrip starts with its down-right dot, then lifts once
for a short horizontal that turns down and rises to the upper right without
breaking. The Noto Sans SC fit keeps both internal turns inside that one second
stroke while recording the font's squarer middle geometry.
The four-frame **氵** filmstrip draws its upper and middle down-right dots as
separate strokes, then lifts again before the bottom stroke's slight up-left
turn and long rise to the upper right. The Noto Sans SC fit keeps that final
turn and rise in one pen-down run while preserving all three sourced strokes.
The four-frame **宀** filmstrip starts with its top dot, lifts for the down-left
stroke on the left, then lifts again before crossing the roof left-to-right and
hooking down-left without breaking. The Noto Sans SC fit preserves the joined
final hook while recording the font's squarer, more vertical roof geometry.
The nine-frame **你** filmstrip writes 亻 first, then the five strokes of 尔:
falling stroke, joined horizontal hook, joined vertical hook, and two separate
lower dots. The Noto Sans SC fit preserves all seven sourced strokes, both
internal joins, and six pen lifts.
The nine-frame **好** filmstrip writes all three strokes of 女 before all three
strokes of 子. The Noto Sans SC fit preserves the bent 女 sweep, 子's top turn
and vertical hook, all six sourced strokes, and five pen lifts.
The nine-frame **我** filmstrip preserves seven sourced strokes and six lifts,
including the joined vertical hook, long curved slash and its upward hook,
separate rising slash, and final upper-right dot.
The ten-frame **是** filmstrip closes 日 in four sourced strokes before drawing
the five-stroke lower body. Its Noto Sans SC fit preserves the joined top-right
corner, all nine strokes, and eight pen lifts.
The four-frame **不** filmstrip draws its top horizontal first, then separately
places the long falling stroke, central vertical, and right-falling dot. Its
Noto Sans SC fit preserves all four sourced strokes and three pen lifts.
The eight-frame **名** filmstrip completes 夕 before drawing 口. Its Noto Sans SC
fit preserves the joined horizontal-to-down-left sweep, 口's joined top-right
corner, all six sourced strokes, and five pen lifts.
The nine-frame **字** filmstrip completes 宀 before drawing 子. Its Noto Sans SC
fit preserves the roof hook, 子's joined top turn and vertical hook, all six
sourced strokes, and five pen lifts.
The seventeen-frame **谢** filmstrip completes 讠 before 身 and 寸. Its Noto Sans
SC fit preserves all five joined turns, all twelve sourced strokes, and eleven
pen lifts.
The fourteen-frame **请** filmstrip completes 讠 before 青. Its Noto Sans SC fit
preserves all four joined turns, all ten sourced strokes, and nine pen lifts.
The eight-frame **再** filmstrip builds the central frame before closing with the
long bottom horizontal. Its Noto Sans SC fit preserves both joined turns, all six
sourced strokes, and five pen lifts.
The seven-frame **见** filmstrip completes the open upper frame before its two lower
runs. Its Noto Sans SC fit preserves all three joined turns, all four sourced
strokes, and three pen lifts.
The four-frame **什** filmstrip completes both strokes of 亻 before writing 十.
Its Noto Sans SC fit preserves all four separately sourced strokes and three pen
lifts.
The four-frame **么** filmstrip keeps its second falling stroke joined to the
rightward base sweep before placing the final dot. Its Noto Sans SC fit preserves
all three sourced strokes, the joined turn, and two pen lifts.
The seven-frame **早** filmstrip completes 日 before writing 十 below. Its Noto
Sans SC fit preserves all six sourced strokes, the joined top-right turn, and
five pen lifts.
The three-frame **上** filmstrip descends the vertical first, then places the
short middle horizontal before the long base. Its Noto Sans SC fit preserves all
three sourced strokes and two pen lifts, completing the Chinese starter inventory.
The five-frame Devanagari **अ** filmstrip carries the upper curve into the lower
bowl without lifting, then separately adds the right-sweeping middle shoulder,
top-to-bottom right stem, and left-to-right shirorekhā. Its Noto Sans Devanagari
fit preserves four sourced strokes and three lifts while recording a published
six-stroke traditional Sanskrit form as explicit variation.
The six-frame Devanagari **आ** filmstrip preserves that joined left body, then
separately adds the middle shoulder, inner and trailing top-to-bottom stems, and
final left-to-right shirorekhā. Its Noto Sans Devanagari fit preserves five
sourced strokes and four lifts while carrying the published base-letter
variation forward.
The five-frame Devanagari **इ** filmstrip descends its upright, turns left around
the upper bowl, sweeps through the lower bowl, and finishes down-right through
the tail in one run before adding the shirorekhā. Its Noto Sans Devanagari fit
preserves the source's two strokes and single lift while identifying that
printed sequence as one teaching form rather than a universal standard.
The six-frame Devanagari **ई** filmstrip reuses that continuous body, then
separately sweeps the upper curl upward and around before adding the shirorekhā.
Its Noto Sans Devanagari fit preserves the source's three strokes and two lifts
while keeping the modern printed sequence distinct from universal practice.
The three-frame Devanagari **उ** filmstrip curves down and left around the upper
bowl, then sweeps back through the waist and around the lower loop without
lifting before adding the shirorekhā. Its Noto Sans Devanagari fit preserves
the source's two strokes and single lift while keeping the modern printed
sequence distinct from universal practice.
The four-frame Devanagari **ऊ** filmstrip reuses that continuous body, then
separately sweeps the right-hand loop upward, around, and down-left before
adding the shirorekhā. Its Noto Sans Devanagari fit preserves the source's
three strokes and two lifts while keeping the modern printed sequence distinct
from universal practice.
The four-frame Devanagari **ए** filmstrip descends its long left stem, curves
through the lower shoulder, and sweeps down the tail without lifting before
separately adding the shorter inward-hooked stem and shirorekhā. Its Noto Sans
Devanagari fit preserves the source's three strokes and two lifts while keeping
the modern printed sequence distinct from universal practice.
The five-frame Devanagari **ऐ** filmstrip reuses both ए base strokes, then
separately sweeps the upper arc upward and left before adding the shirorekhā.
Its Noto Sans Devanagari fit preserves the source's four strokes and three lifts
while keeping the modern printed sequence distinct from universal practice.
The seven-frame Devanagari **ओ** filmstrip reuses आ's joined left body,
separate shoulder, and two stems, then separately sweeps the upper arc upward
and left before adding the shirorekhā. Its Noto Sans Devanagari fit preserves
the source's six strokes and five lifts while keeping the modern printed
sequence distinct from universal practice.
The eight-frame Devanagari **औ** filmstrip reuses the same four-stroke base,
then separately sweeps its lower and taller upper arcs upward and left before
adding the shirorekhā. Its Noto Sans Devanagari fit preserves the source's seven
strokes and six lifts while keeping the modern printed sequence distinct from
universal practice.
The four-frame Devanagari **क** filmstrip starts at the left bowl's upper-right
junction and circles counterclockwise before separately descending the central
stem, sweeping the right-hand arch clockwise, and adding the shirorekhā. Its
Noto Sans Devanagari fit preserves the animated source's four strokes and three
lifts, with the same buildup independently corroborated by the Central Hindi
Directorate's learner deskbook.
The three-frame Devanagari **ग** filmstrip starts at the loop's upper-right
junction, circles counterclockwise, and carries that run directly up the joined
stem before separately descending the right stem and adding the shirorekhā.
Its Noto Sans Devanagari fit preserves the animated source's three strokes and
two lifts, with the same buildup independently corroborated by the Central
Hindi Directorate's learner deskbook.
The three-frame Devanagari **च** filmstrip draws the short upper bar
left-to-right and turns directly through the shoulder into the rounded open
body before separately descending the right stem and adding the shirorekhā.
Its Noto Sans Devanagari fit preserves the animated source's three strokes and
two lifts. The Central Hindi Directorate deskbook confirms component order but
stages the upper bar and body separately, so it corroborates order without
being treated as independent evidence for the animation's first join.
The three-frame Devanagari **त** filmstrip starts at the body's upper-right
junction, sweeps left across the shoulder, and curves down to the open lower
tip before separately descending the right stem and adding the shirorekhā.
Its Noto Sans Devanagari fit preserves the animated source's three strokes and
two lifts, with the same buildup independently corroborated by the Central
Hindi Directorate's learner deskbook.
The three-frame Devanagari **द** filmstrip descends the short stem before one
continuous sweep around the outer body, inward curl, and down-right tail, then
adds the shirorekhā. Its Noto Sans Devanagari fit preserves the animated
source's three strokes and two lifts. The Central Hindi Directorate deskbook
confirms component order but stages the outer body and curl-tail separately,
so it corroborates order without being treated as independent evidence for
the animation's body-to-curl join.
The four-frame Devanagari **ध** filmstrip curls around the upper spiral and
sweeps right through its shoulder before separately drawing the lower bowl,
right stem, and shirorekhā. Its Noto Sans Devanagari fit preserves the animated
source's four strokes and three lifts, with the same buildup independently
corroborated by the Central Hindi Directorate's learner deskbook.
The three-frame Devanagari **न** filmstrip circles clockwise around the left
loop and continues right along its shoulder before separately drawing the right
stem and shirorekhā. Its Noto Sans Devanagari fit preserves the animated
source's three strokes and two lifts, with the same directions independently
corroborated by the Central Hindi Directorate's learner deskbook.
The three-frame Devanagari **प** filmstrip descends the left stem and curves
right around the lower bowl before separately drawing the right stem and
shirorekhā. Its Noto Sans Devanagari fit preserves the animated source's three
strokes and two lifts, with the same directions independently corroborated by
the Central Hindi Directorate's learner deskbook.
The four-frame Devanagari **ब** filmstrip circles counterclockwise around the
oval before separately drawing the right stem, inner diagonal, and shirorekhā.
Its Noto Sans Devanagari fit preserves the animated source's four strokes and
three lifts, with the same directions independently corroborated by the Central
Hindi Directorate's learner deskbook.
The three-frame Devanagari **भ** filmstrip sweeps clockwise through the upper
loop, descends its joined trunk, curls clockwise around the lower bowl, and
continues right through the crossbar before separately drawing the right stem
and shirorekhā. Its Noto Sans Devanagari fit preserves the animated source's
three strokes and two lifts; the Central Hindi Directorate's learner deskbook
corroborates component order while staging the upper and lower body parts
separately.
The three-frame Devanagari **म** filmstrip descends the left stem, curls
clockwise around the lower loop, and continues right through the crossbar before
separately drawing the right stem and shirorekhā. Its Noto Sans Devanagari fit
preserves the animated source's three strokes and two lifts; the Central Hindi
Directorate's learner deskbook corroborates component order while staging the
left stem and loop-crossbar as separate buildup steps.
The four-frame Devanagari **य** filmstrip curves clockwise around the inner curl
before separately drawing the lower bowl, right stem, and shirorekhā. Its Noto
Sans Devanagari fit preserves the three lifts shared by Opiaterein's animation
and the Central Hindi Directorate's four-part learner buildup; JackPotte's
alternate 11-frame animation documents a joined inner-curl-and-bowl form with
two lifts.
The three-frame Devanagari **र** filmstrip descends the stem and curls clockwise
around the lower loop before separately drawing the down-right diagonal tail
and shirorekhā. Its Noto Sans Devanagari fit preserves the two lifts shared by
Opiaterein's animation and the Central Hindi Directorate's three-part learner
buildup; JackPotte's alternate seven-frame animation documents a joined
loop-and-tail body with one lift.
The four-frame Devanagari **ल** filmstrip curves up and clockwise around the open
left loop before separately drawing the up-right diagonal arm, right stem, and
shirorekhā. Its Noto Sans Devanagari fit preserves the three lifts shared by
Opiaterein's loop-first animation and the Central Hindi Directorate's four-part
learner buildup; JackPotte's alternate 12-frame animation documents a stem-first
part order.
The three-frame Devanagari **व** filmstrip starts at the upper-right of the body
and circles counterclockwise around the left loop before separately drawing the
top-to-bottom right stem and left-to-right shirorekhā. Its Noto Sans Devanagari
fit preserves JackPotte's 11-frame animation and two lifts; the Central Hindi
Directorate's learner deskbook independently corroborates the same three-part
buildup while the animation supplies the within-run directions.
The three-frame Devanagari **श** filmstrip starts at the upper loop's lower
inner tip, sweeps clockwise around that loop, descends through the outer curve,
curls around the lower loop, and continues down-right through the diagonal tail
before separately drawing the top-to-bottom right stem and left-to-right
shirorekhā. Its Noto Sans Devanagari fit preserves the two lifts shown by
Opiaterein's 25-frame animation; JackPotte's animation and the Central Hindi
Directorate's learner deskbook independently corroborate the same three-part
buildup.
The four-frame Devanagari **स** filmstrip descends the left stem, curls left
around the central hook, and continues down-right through the diagonal tail
before separately drawing the middle crossbar, top-to-bottom right stem, and
left-to-right shirorekhā. Its Noto Sans Devanagari fit preserves JackPotte's
13-frame animation and three lifts; the Central Hindi Directorate's learner
deskbook corroborates component order while staging the left curve and diagonal
tail separately.
The three-frame Devanagari **ह** filmstrip descends the right stem, sweeps left
through the shoulder, and curves clockwise around the hooked body before
separately drawing the down-left outer curve and down-right tail, then the
left-to-right shirorekhā. Its Noto Sans Devanagari fit preserves Opiaterein's
22-frame animation and two lifts; the Central Hindi Directorate's learner
deskbook corroborates component order while staging the joined first body across
more buildup steps. This completes the source-verified Devanagari starter set.
The two-frame Cyrillic **а** filmstrip sweeps over the upper shoulder and
counterclockwise around the round lower body, then continues down the right-hand
finishing stem without lifting. RussianIrina's native-teacher all-letter lesson
demonstrates the one-run lowercase school hand at 00:50–00:55; the Noto Sans
Cyrillic fit preserves its zero-lift single-storey motion while routing the
entry through the bundled font's extra double-storey printed shoulder.
The two-frame Cyrillic **б** filmstrip circles the lower body counterclockwise,
then continues through the rising shoulder into the rightward top flag without
lifting. The same native-teacher lesson demonstrates that one-run school hand
at 01:13–01:18; the Noto Sans Cyrillic fit preserves its body-to-flag order
while routing the handwritten diagonal transition through the printed glyph's
upper-left shoulder.
The two-frame Cyrillic **в** filmstrip starts at the baseline, climbs through
the tall upper loop, returns down the left stem, and continues counterclockwise
around the lower bowl without lifting. RussianIrina demonstrates that one-run
school hand at 01:33–01:38; the Noto Sans Cyrillic fit preserves its
baseline-to-upper-loop-to-lower-bowl order while routing the cursive ascender
through the printed glyph's compact upper bowl and straight left stem.
The two-frame Cyrillic **г** filmstrip climbs from the baseline through the
upright and top bar, then retraces the top and descends without lifting. The
same native-teacher lesson demonstrates a rounded two-hump cursive г at
01:54–01:57; the Noto Sans Cyrillic fit preserves its zero-lift evidence while
documenting that the isolated block glyph has no smaller exit arch.
The two-frame Cyrillic **д** filmstrip circles its closed body counterclockwise,
then descends through the right foot, sweeps across the base shelf, retraces the
left foot, and finishes rightward without lifting. The same lesson demonstrates
a looped cursive д at 02:14–02:19; the Noto Sans Cyrillic fit preserves its
body-before-descender order and zero-lift evidence while documenting that the
isolated block glyph has a shelf and two feet instead of a descender loop.
The two-frame Cyrillic **е** filmstrip curves around the upper bowl and sweeps
right through the middle, then reverses through the junction and continues
counterclockwise around the lower bowl without lifting. The same lesson
demonstrates the tall looped school hand at 02:26–02:30; the Noto Sans Cyrillic
fit preserves its zero-lift upper-loop-to-middle-to-lower-bowl order while
routing it through the compact printed glyph's long middle bar.
The four-frame Cyrillic **ё** filmstrip completes that same joined body, then
lifts for the left dot and lifts again for the right dot. The native-teacher
lesson demonstrates this body-before-left-dot-before-right-dot order at
02:51–02:56; the Noto Sans Cyrillic fit preserves its two-lift evidence while
tracing the compact printed e body and both circular dots.
The two-frame Cyrillic **ж** filmstrip traces the left wings and rises through
the centre, then retraces the central upright and continues through the right
wings without lifting. The native-teacher lesson demonstrates its rounded
left-to-centre-to-right school-hand order at 03:16–03:21; the Noto Sans Cyrillic
fit preserves that zero-lift evidence while tracing the printed glyph's straight
central upright and four diagonal arms.
The two-frame Cyrillic **з** filmstrip circles the smaller upper lobe and
descends through the middle, then continues around the larger lower lobe and
finishes at the lower right without lifting. The native-teacher lesson
demonstrates its joined upper-lobe-to-lower-lobe school-hand order at
03:34–03:39; the Noto Sans Cyrillic fit preserves that zero-lift evidence while
tracing the compact printed double-lobe glyph and records that the printed form
omits the school hand's rising exit join.
The three-frame Cyrillic **и** filmstrip descends the left stem, rises through
the joined diagonal, then descends the right stem without lifting. The native-
teacher lesson demonstrates its rounded left-stem-to-diagonal-to-right-stem
school-hand order at 03:56–04:02; the Noto Sans Cyrillic fit preserves that
zero-lift evidence while tracing the printed backwards-N glyph and records that
the printed form omits the school hand's entry and exit joins.
The four-frame Cyrillic **й** filmstrip completes the same joined body as **и**,
then lifts once and draws the breve above from left to right as one dipped arc.
The native-teacher lesson demonstrates that body-before-breve order at
04:17–04:24; the Noto Sans Cyrillic fit preserves the one-lift evidence and
left-to-right breve direction while tracing the printed backwards-N body and
separate curved mark.
The three-frame Cyrillic **к** filmstrip descends the left stem, rises through
the upper arm and returns to the middle, then continues through the lower arm
without lifting. The native-teacher lesson demonstrates its looped
stem-to-upper-arm-to-lower-arm school-hand order at 04:45–04:51; the Noto Sans
Cyrillic fit preserves that zero-lift evidence while tracing the printed
vertical and two angular diagonals and records the source's rounded upper loop
and entry and exit joins.
The three-frame Cyrillic **л** filmstrip curves from the baseline hook up the
left leg, sweeps along the top shoulder, then descends the right stem without
lifting. The native-teacher lesson demonstrates its pointed
hooked-left-leg-to-apex-to-right-leg school-hand order at 05:06–05:10; the Noto
Sans Cyrillic fit preserves that zero-lift evidence while tracing the printed
curved left leg, horizontal top shoulder, and straight right stem and records
the source's slanted right leg and entry and exit joins.
The four-frame Cyrillic **м** filmstrip rises through the left stem, descends to
the central valley, rises to the second apex, then descends the right stem
without lifting. The native-teacher lesson demonstrates its rounded two-arch
school-hand order at 05:26–05:31; the Noto Sans Cyrillic fit preserves that
zero-lift evidence while tracing the printed straight upright stems and deep
central V and records the source's entry and exit joins.
The three-frame Cyrillic **н** filmstrip descends the left stem, retraces to the
middle bridge and rises to the upper right, then descends the right stem without
lifting. The native-teacher lesson demonstrates its rounded
left-stem-to-middle-bridge-to-right-stem school-hand order at 05:47–05:52; the
Noto Sans Cyrillic fit preserves that zero-lift evidence while tracing the
printed straight vertical stems and horizontal middle bar and records the
source's rounded bridge and entry and exit joins.
The two-frame Cyrillic **о** filmstrip curves left over the top and descends the
left side, then sweeps through the bottom and rises along the right side to
close without lifting. The native-teacher lesson demonstrates its
counterclockwise upper-right-to-left-side-to-bottom-to-right-side school-hand
order at 05:59–06:03; the Noto Sans Cyrillic fit preserves that zero-lift
evidence while tracing the printed wider upright oval and records the source's
taller, slightly slanted proportions.
The three-frame Cyrillic **п** filmstrip descends the left stem, retraces to the
top shoulder and sweeps right, then descends the right stem without lifting.
The native-teacher lesson demonstrates its rounded
left-stem-to-top-shoulder-to-right-stem school-hand order at 06:26–06:31; the
Noto Sans Cyrillic fit preserves that zero-lift evidence while tracing the
printed squared arch, straight uprights, and horizontal top bar and records the
source's entry and exit joins.
The three-frame Cyrillic **р** filmstrip descends its stem below the baseline,
retraces to the upper shoulder and curves right, then sweeps around the printed
bowl and returns to the stem without lifting. The native-teacher lesson
demonstrates its open stem-before-rounded-bowl school-hand order at
06:46–06:52; the Noto Sans Cyrillic fit preserves that zero-lift evidence while
tracing the printed straight descender and closed rounded bowl and records the
source's long-descender Latin-p-like shape and entry and exit joins.
The two-frame Cyrillic **с** filmstrip curves left over the top and descends the
left side, then sweeps through the bottom and rises to the lower-right tip
without lifting. The native-teacher lesson demonstrates its counterclockwise
upper-right-to-left-side-to-bottom-to-lower-right school-hand order at
07:04–07:08; the Noto Sans Cyrillic fit preserves that zero-lift evidence while
tracing the printed wider upright C-like outline and records the source's tall,
slightly slanted proportions and rising exit.
The three-frame Cyrillic **т** filmstrip descends the central stem, retraces to
the top junction and sweeps left, then retraces through the junction and sweeps
to the right tip without lifting. The native-teacher lesson demonstrates its
left-stem-to-first-arch-to-middle-stem-to-second-arch-to-right-stem school-hand
order at 07:29–07:36; the Noto Sans Cyrillic fit preserves that initial descent
and zero-lift evidence while tracing the printed central stem and horizontal
top bar and records the source's two-arch Latin-m-like shape and rising exit.
The four-frame Cyrillic **у** filmstrip descends the left arm, rises through the
right arm, retraces through the junction into the long descender, then curves
left through its terminal without lifting. The native-teacher lesson
demonstrates its narrow rounded upper body, looped descender, crossing, and
rising exit at 07:50–07:55; the Noto Sans Cyrillic fit preserves that
left-arm-to-right-arm-to-descender order and zero-lift evidence while tracing
the printed straight upper arms and broad unlooped left-curving terminal.
The five-frame Cyrillic **ф** filmstrip descends the long central stem below the
baseline, lifts once to circle the left bowl, crosses the stem, then continues
around the right bowl in the same second run. The native-teacher lesson
demonstrates that stem-before-left-loop-before-right-loop school-hand order at
08:16–08:26; the Noto Sans Cyrillic fit preserves its one-lift evidence while
tracing the printed straight ascender-descender and two wider upright bowls and
records the source's narrower linked loops and rising exit.
The four-frame Cyrillic **х** filmstrip draws the left pair of arms through the
centre crossing, lifts once, then draws the right pair through that same
crossing. The native-teacher lesson demonstrates a right-bulging left curve
followed by a left-bulging right curve at 08:42–08:49; the Noto Sans Cyrillic
fit preserves that top-to-bottom left-run-before-right-run order and one-lift
evidence while straightening both curves into the printed glyph's four diagonal
arms and records the source's rounded curves and rising exit.
The four-frame Cyrillic **ц** filmstrip descends the left stem, sweeps along the
bottom bar and rises through the right stem, retraces down to the tail shoulder,
then descends the short tail without lifting. The native-teacher lesson
demonstrates its rounded left-stem-to-right-stem-to-looped-tail school-hand
order at 09:05–09:10; the Noto Sans Cyrillic fit preserves that zero-lift
evidence while tracing the printed squared U-like body and short right
descender and records the source's diagonal join, lower loop, and rising exit.
The three-frame Cyrillic **ч** filmstrip descends the short left stem, sweeps
through the shallow bowl and rises along the right stem, then descends the full
right stem without lifting. The native-teacher lesson demonstrates its narrow
rounded short-stem-to-long-stem bridge and rising exit at 09:24–09:28; the Noto
Sans Cyrillic fit preserves that zero-lift order while tracing the printed
shorter left stem, shallow bowl, and full-height right stem and records the
source's narrower bridge, curled baseline, and rising exit.
The five-frame Cyrillic **ш** filmstrip descends the left stem, crosses the
first baseline join and rises then retraces the middle stem, crosses the second
baseline join and rises then retraces the right stem without lifting. The
native-teacher lesson demonstrates its rounded left-to-middle-to-right
school-hand order and rising exit at 09:49–09:57; the Noto Sans Cyrillic fit
preserves that zero-lift evidence while tracing the printed three straight
stems and horizontal baseline bars and records the source's diagonal rounded
joins, curled baseline, and rising exit.
The six-frame Cyrillic **щ** filmstrip descends the left stem, crosses the first
baseline join and rises then retraces the middle stem, crosses the second join
and rises then retraces the right stem, crosses the tail shoulder, and descends
the short tail without lifting. The native-teacher lesson demonstrates its
rounded left-to-middle-to-right-to-looped-tail order at 10:17–10:25; the Noto
Sans Cyrillic fit preserves that zero-lift evidence while tracing the printed
three straight stems, horizontal baseline bars, and short right descender and
records the source's diagonal rounded joins and looped exit.
The five-frame Cyrillic **ъ** filmstrip sweeps right along the broad top flag,
descends the main stem, sweeps right along the lower bowl, curves upward around
its right side, then returns left through the upper bowl to close against the
stem without lifting. The native-teacher lesson demonstrates its narrow looped
entry, rounded shoulder, descending stem, and counterclockwise lower bowl at
10:34–10:38; the Noto Sans Cyrillic fit preserves that zero-lift
flag-to-stem-to-bowl order while tracing the printed broad top flag, straight
stem, and closed lower bowl and records the source's entry loop and rounded
shoulder.
The five-frame Cyrillic **ы** filmstrip descends the left stem, sweeps right
along the lower bowl, curves upward around its right side and returns left to
close it, then lifts once and descends the separate right stem. The
native-teacher lesson demonstrates its narrow looped entry, joined
counterclockwise lower bowl, separately descended right stem, and curled exit
at 10:45–10:56; the Noto Sans Cyrillic fit preserves that one-lift
body-before-right-stem order while tracing the printed straight uprights and
wide closed lower bowl and records the source's entry loop and rising exit.
The four-frame Cyrillic **ь** filmstrip descends the stem, sweeps right along
the lower bowl, curves upward around its right side, then returns left through
the upper bowl to close against the stem without lifting. The native-teacher
lesson demonstrates its narrow descending entry and joined counterclockwise
lower bowl at 11:16–11:20; the Noto Sans Cyrillic fit preserves that zero-lift
stem-to-bowl order while tracing the printed straight upright and wide closed
lower bowl and records the source's slanted entry and rounded handwritten join.
The four-frame Cyrillic **э** filmstrip sweeps right across the upper curve,
continues down around the outer right side, sweeps left through the lower
curve, then lifts once and draws the middle tongue from right to left. The
native-teacher lesson demonstrates that outer-before-tongue order at
11:25–11:32; the Noto Sans Cyrillic fit preserves its one-lift evidence while
tracing the printed broad open-left curve and straight middle bar and records
the source's narrower rounded curve and gently hooked tongue.
The five-frame Cyrillic **ю** filmstrip descends the left stem, retraces to the
middle and sweeps right along the connector, curves across the oval's top,
continues down its right side, then sweeps through the bottom and rises to
close without lifting. The native-teacher lesson demonstrates its looped
entry, diagonal connector, and clockwise cursive oval at 11:44–11:58; the Noto
Sans Cyrillic fit preserves that zero-lift stem-to-connector-to-oval order
while tracing the printed straight upright, horizontal middle bar, and wide
closed oval.
The four-frame Cyrillic **я** filmstrip climbs the right stem from the baseline,
curves counterclockwise around the upper bowl, sweeps left through the lower
join, then descends the diagonal leg without lifting. The native-teacher lesson
demonstrates its curved entry, upper loop, long diagonal leg, and short exit at
12:13–12:21; the Noto Sans Cyrillic fit preserves that zero-lift
rise-to-loop-to-leg order while tracing the printed straight right upright,
broad upper bowl, and angular lower-left leg.
The four-frame Gujarati **અ** filmstrip sweeps clockwise around the open left
curve, continues through the lower body and middle shoulder, retraces into the
small right arch, then lifts once and descends the separate right stem into its
foot. t30apps.com's version-1.0 animation demonstrates that body-before-stem
order as two SVG paths; the Noto Sans Gujarati fit preserves its one-lift
evidence while tracing the broader printed joins. The source's own warning that
forms and stroke orders vary remains visible in the canonical evidence note.
The five-frame Gujarati **આ** filmstrip repeats the joined body and lifted
right stem of **અ**, then lifts again to descend the added trailing ā stem.
t30apps.com's next version-1.0 animation exposes those runs as three ordered
SVG paths; the Noto Sans Gujarati fit preserves the two-lift order while
tracing the printed glyph's wider stem spacing. The source's variation warning
continues to qualify the demonstrated school-hand form.
The four-frame Gujarati **ઇ** filmstrip circles the small upper-left loop,
passes through its narrow middle crossing, sweeps clockwise around the broad
lower loop, then rises into the upper-right hook without lifting. t30apps.com's
version-1.0 animation exposes only its first SVG path for this letter; the Noto
Sans Gujarati fit preserves that zero-lift loop-to-loop-to-hook order while
tracing the wider printed body. The source's variation warning continues to
qualify the demonstrated school-hand form.
The four-frame Gujarati **ઈ** filmstrip repeats that unbroken **ઇ** motion, but
continues upward and curls clockwise around the taller top hook. The adjacent
t30apps.com animation again populates only its first SVG path; the Noto Sans
Gujarati fit preserves the zero-lift loop-to-loop-to-extended-curl order while
tracing the printed glyph's rounded high terminal. The source's variation
warning continues to qualify the demonstrated school-hand form.
The three-frame Gujarati **ઉ** filmstrip circles clockwise through the small
upper bowl and middle cusp, sweeps around the broad lower bowl, then climbs the
tall outer-left curve to finish at the upper right without lifting. The next
t30apps.com animation again populates only its first SVG path; the Noto Sans
Gujarati fit preserves that zero-lift upper-bowl-to-lower-bowl-to-outer-curve
order while tracing the wider printed body. The source's variation warning
continues to qualify the demonstrated school-hand form.
The three-frame Gujarati **ઊ** filmstrip repeats the complete unbroken **ઉ**
body, continues across its high shoulder, then descends the long right-side
tail into its lower foot. The adjacent t30apps.com animation keeps all of that
motion in one populated SVG path; the Noto Sans Gujarati fit preserves the
zero-lift complete-u-before-extended-tail order while tracing the printed
glyph's open shoulder and straighter tail. The source's variation warning
continues to qualify the demonstrated school-hand form.
The four-frame Gujarati **એ** filmstrip circles the joined left bowl and lower
body through the small right arch, lifts to descend the full-height right stem,
then lifts again to sweep the separate high arc from left to right. The next
t30apps.com animation exposes those parts as three ordered SVG paths; the Noto
Sans Gujarati fit preserves the body-before-stem-before-arc order and two lifts
while tracing the wider printed joins. The source's variation warning continues
to qualify the demonstrated school-hand form.
The four-frame Gujarati **ઐ** filmstrip restores a previously missing inventory
entry. It writes the complete **એ** body and stem, sweeps the lower high arc,
then lifts a third time to sweep the second, higher arc from left to right. The
adjacent t30apps.com animation exposes four ordered SVG paths; the Noto Sans
Gujarati fit preserves that order while tracing the printed glyph's tighter
stacked arcs. The source's variation warning continues to qualify the
demonstrated school-hand form.
The six-frame Gujarati **ઓ** filmstrip writes the complete **આ** body and two
stems, then lifts a third time to sweep the separate high arc from left to
right. The adjacent t30apps.com animation exposes those parts as four ordered
SVG paths; the Noto Sans Gujarati fit preserves that order while tracing the
wider printed body and shorter arc. The source's variation warning continues
to qualify the demonstrated school-hand form. The next audit target is **ઔ**,
which is missing from the current inventory and adds a second high arc.
The five-frame Gujarati **ઔ** filmstrip restores that missing inventory entry.
It writes the complete **ઓ** body and stems, sweeps the lower high arc, then
lifts a fourth time to sweep the second, higher arc from left to right. The
t30apps.com animation exposes five ordered SVG paths; the Noto Sans Gujarati fit
preserves that order while tracing the printed glyph's tighter stacked arcs.
The source's variation warning continues to qualify the demonstrated
school-hand form. A canonical-order audit now queues missing **ઋ** next.
The three-frame Gujarati **ઋ** filmstrip restores the remaining source-backed
independent-vowel gap. It sweeps across the bent left body and turns down-left,
lifts to descend the central stem, then lifts again to circle the compact right
loop and descend through its tail. The t30apps.com animation exposes three
ordered SVG paths; the Noto Sans Gujarati fit preserves that order while
tracing the broader printed body and more angular loop. The source's variation
warning continues to qualify the demonstrated school-hand form; **ક** is next.
The two-frame Gujarati **ક** filmstrip begins consonant coverage with one
continuous run around the upper loop, diagonally through the middle, and around
the lower body. It then lifts once and sweeps the separate crossing diagonal
from lower left to upper right. The t30apps.com animation exposes those as two
ordered SVG paths; the Noto Sans Gujarati fit preserves their order while
tracing the broader printed loops. The source's variation warning continues to
qualify the demonstrated school-hand form. The two-frame Gujarati **ખ**
filmstrip starts at the upper left and keeps the pen down through the left lobe
and inner curl. It then lifts once, descends the separate full-height right
spine, and turns through its lower foot. The t30apps.com animation exposes those
as two ordered SVG paths; the Noto Sans Gujarati fit preserves their order while
tracing the broader printed lobe and more angular foot. The source's variation
warning continues to qualify the demonstrated school-hand form. The two-frame
Gujarati **ગ** filmstrip circles the rounded left body from its upper-left start
to its lower-left finish. It then lifts once, descends the separate full-height
right spine, and turns through its lower foot. The t30apps.com animation exposes
those as two ordered SVG paths; the Noto Sans Gujarati fit preserves their order
while tracing the broader printed body and more angular foot. The source's
variation warning continues to qualify the demonstrated school-hand form;
the source-and-font audit then exposes **ઘ** as a missing inventory entry. Its
two-frame filmstrip begins at the upper left, circles the upper lobe, turns back
through the middle, and continues around the rounded lower body to its upper
right. It then lifts once, descends the separate full-height right spine, and
turns through its lower foot. The t30apps.com animation exposes those as two
ordered SVG paths; the Noto Sans Gujarati fit preserves their order while
tracing the broader printed lobes and more angular foot. The source's variation
warning continues to qualify the demonstrated school-hand form; missing **ઙ**
is next. Its two-frame filmstrip begins at the upper right, sweeps left through
the upper turn, continues through the middle and rounded lower body, and
finishes at the lower left. It then lifts once and circles the separate
upper-right dot. The t30apps.com animation exposes those as two ordered SVG
paths; the Noto Sans Gujarati fit preserves their order while tracing the
broader printed turns and larger dot. The source's variation warning continues
to qualify the demonstrated school-hand form. The two-frame Gujarati **ચ**
filmstrip circles the upper bowl from its upper-left start, turns through the
small middle loop, and continues around the broad lower body to its upper-right
finish. It then lifts once, descends the separate full-height right spine, and
turns through its lower foot. The t30apps.com animation exposes those as two
ordered SVG paths; the Noto Sans Gujarati fit preserves their order while
tracing the broader printed bowls and more angular foot. The source's variation
warning continues to qualify the demonstrated school-hand form; **છ** is next.
The canonical script-data group is capped at 250 kB per cacheable batch, so
these growing source notes stay within the app's enforced 500 kB eager-chunk
budget without removing learner-facing evidence.
The runtime resolves each cited path back to the owning script and lazily loads that
script's font, while Tamil continues to use Noto Sans Tamil. Unverified letters
still fall back to the numbered prose list, unchanged. Extending the coverage is HL-C09,
and it needs a cited source per letter.

## Where it fits

```
code/learning/human-languages/data/scripts/*.json   ← the source of truth (HL01)
        │  (glyph, components, strokeOrder, notes per letter)
        ▼
language-ladder                                     ← this app renders it (HL03)
```

The app imports those JSON files **directly**, so it can never drift from the
curriculum. Adding a script to the curriculum surfaces it here with a one-line
edit in `src/data.ts`. Ships today with **Cyrillic, Hebrew, Chinese, Arabic,
Devanagari, Gujarati, Tamil**, and the three **Dravidian syllabaries** below.

### Dravidian syllabaries (Telugu / Kannada / Malayalam)

These three are **abugidas** — a base consonant carries an inherent *a*, and a
vowel sign turns it into a syllable (క = *ka*, కి = *ki*, కు = *ku*; ఖ = *kha*).
So each "letter" is a syllable, and `data/scripts/{telugu,kannada,malayalam}.json`
are **generated from Unicode** by `data/scripts/generate_syllabary.py`: every
glyph is composed from code points and its romanization taken from the official
Unicode character name (ISO-15919), never hand-typed. They are **recognition
only** — `strokeOrder` is empty, since the handwriting ductus is a separate,
source-gated effort; the Browse detail hides the stroke-order section when it's
absent rather than showing an empty one.

Each consonant carries its full vowel row: the ten short/long vowels plus the
two diphthongs *ai / au* and the **vocalic R** of Sanskrit-derived words (కృ =
*kr̥*, as in *kr̥ṣṇa* "Krishna") — thirteen syllables per consonant, so **Telugu
455 / Kannada 455 / Malayalam 468**. The vocalic-R romanization is ISO-15919
`r̥` (a plain *r* with a ring below), deliberately not IAST's dot-below `ṛ`
(which in ISO-15919 is the unrelated retroflex ṛ).

**Practice introduces them slowly** (`src/syllabary.ts`). Rather than drill all
~450 syllables at once, the recall drill opens with a *single consonant's vowel
row* — ka kā ki kī ku kū ke kē ko kō kai kau kr̥ — and unlocks the next consonant only once
the current row is mastered (a Leitner box ≥ 3). So recognition is built one
consonant at a time, the "ka, ki, ku … kha, khi, khu" way. On these scripts the
drill's target and its distractors are both confined to the unlocked syllables
(a consonant you haven't met never appears, even as a wrong option), the mastery
read-out is scoped to the open rows, and a cue shows *"Learning consonant N of M
— master this vowel row to unlock the next."* The gate is a pure, unit-tested
helper (with a control that keeps the 2nd consonant locked until row 1 is done);
the alphabets and Mixed mode are unaffected.

**The special consonants are flagged** the way Latin false friends are. The
retroflex **ḷ** and the alveolar **ṟ / ṉ** are exactly the letters an outsider
mistakes for the ordinary *l / r / n* (ల vs ళ), so Browse gives them a **★
special consonant** badge, a *"tell it apart from 'l'"* note grounded in the
retroflex/alveolar distinction, and a tinted tile. The classifier
(`specialConsonant` in `core.ts`, unit-tested with a control) keys on the
script-agnostic ISO-15919 mark — leading ḷ (U+1E37) / ṟ (U+1E5F) / ṉ (U+1E49) —
which appears only on these consonants, so no data changed to add it.

**Browse them as a matrix.** An abugida is really a table, so for the
syllabaries Browse offers a **List / Matrix** toggle (alphabets stay a plain
list). Matrix lays the syllables out as **rows = consonants, columns = vowels**
— the same ka/kā/ki… pattern repeating down every consonant, made visible at a
glance — and clicking a cell opens the usual decomposition panel. The layout is
pure (`buildSyllableMatrix` in `src/matrix.ts`): rows reuse the grounded
consonant boundary from `syllabary.ts`, the vowel column headers are read off
the first consonant's own row, and a ragged script yields **no matrix** rather
than a mislabelled cell (unit-tested with a control). No new data — the same
generated syllables, re-arranged. The **special-consonant rows** (retroflex ḷa,
alveolar ṟa / ṉa) are marked with a ★ in the grid, reusing the same
`specialConsonant` classifier the tiles use so the confusable rows stand out.

**The independent (word-initial) vowels.** Everything above is consonant + vowel
*sign*; a word that *begins* with a vowel writes a different letter — the
independent vowel (అ *a*, ఆ *ā* … ఔ *au*, ఋ *r̥*). Browse shows these as a small
strip above the grid. They are generated the same way (`<SCRIPT> LETTER <V>`,
ISO-15919 roman from the shared vowel table) but kept in a **separate
`independentVowels` field**, not mixed into `letters`, so the syllabary and the
gate/matrix that key on it being all-syllables are untouched. Control-tested
(the 13 grounded glyphs; none leak into `letters`, so `isSyllabary` still holds).

**The script's numerals.** Reading a language means reading its numbers too, and
these scripts write them with distinct glyphs (Telugu ౦౧౨…). Browse shows a
**"Numerals (0–9)"** strip; each digit is generated from `<SCRIPT> DIGIT <N>` and
romanized as its value, kept in a **separate `digits` field** (same non-breaking
pattern as the vowels). Control-tested (the 10 grounded glyphs → 0–9).

**The same syllable in its sister scripts.** The three cousins write one sound
three ways — కి / ಕಿ / കി are all *ki* — and once you can read one, the others
are a short hop. When you select a syllable in Browse, the detail panel shows it
as the *other* syllabaries write it, under **"Same sound, sister scripts"**, so
the family connection (the spiral model's core memory hook) is visible on the
page. The pure `crossScriptSiblings` (`src/siblings.ts`) matches by the shared
ISO-15919 romanization — safe because the trio come from one generator, so "ki"
is byte-identical everywhere — and is **restricted to fully-syllabic scripts**
(`isSyllabary`), so Tamil / Devanagari / Gujarati and the alphabets are never
mis-matched, and a Malayalam-only row (alveolar *ṉa*) correctly shows none.
Control-tested (Telugu *ki* → the real Kannada + Malayalam glyphs, never itself;
read-only — `letters` / `isSyllabary` / the matrix untouched).

## Design

- **`src/core.ts`** — the pure, unit-tested heart: `buildScriptView`,
  `scriptSummary`, `isFalseFriend`, `falseFriends`. No DOM, no globals; this is
  where the pedagogy is tested.
- **`src/data.ts`** — the only place that imports the canonical script JSON.
- **`src/truetype.ts`** — a small zero-dependency TrueType reader, so every
  letter this app *draws* comes from the font rather than from a hand.
- **`src/strokes.ts`** — the pen-path model: strokes as pen-down runs, segments
  as labelled parts that must meet head-to-tail, with cited provenance.
- **`src/ductusview.ts`** — the two above, composed into SVG. Pure: it returns a
  tree of plain objects plus a serialiser, and never touches `document`.
- **`src/main.ts`** — a deliberately framework-free vanilla-DOM shell. It walks
  the `ductusview` tree with `createElementNS`/`setAttribute`/`textContent`;
  there is no `innerHTML` anywhere in the app.

## Develop

```sh
npm install
npm run dev        # local dev server
npm run typecheck  # strict source + test typecheck
npm test           # unit tests (vitest)
npm run build      # production build to dist/
npm run preview    # serve the production build
```

## Practice mode (recall drill)

Toggle **Practice** to drill *recall*: the app shows a **sound** and you pick the
matching **glyph** from four options. Wrong answers are the **confusable** ones
(same role / same false-friend status), reveal shows the answer's decomposition,
and a running **score** tracks correct / total / %. The drill logic lives in the
pure, unit-tested `src/drill.ts` (`buildDrillQuestion`, `confusabilityOrder`,
`checkAnswer`, scoring); all randomness is injected by the UI so the core stays
deterministic.

**Spaced repetition.** Which letter you're asked isn't random — a Leitner /
SM-2-lite scheduler (`src/scheduler.ts`, measured in *sessions*, no `Date`)
picks the most-overdue letter each question. Get it right and it drifts into the
future (1 → 3 → 7 → 15 → 30 sessions); miss it and it comes straight back. A
**"mastered N / total"** read-out shows progress. The scheduler is pure and
unit-tested — per `HL02` it's "the core module … where test coverage matters
most."

**Interleaving (Mixed mode).** Toggle **Practice → Mixed (all scripts)** to drill
every script at once. `src/interleave.ts` lays all letters into one
round-robin pool and the scheduler picks across it, so a Cyrillic prompt is
followed by Hebrew, then Devanagari, … — the mixing that HL02 says "forces
discrimination and transfers better." Distractors still come from the target's
own script; mastery counts across the whole pool.

## Scope and what's next

Today the app does **read + decompose** (Browse) and **recall** (Practice). Still
to come toward the full `HL02` spec: the **interleaving scheduler** (spaced,
cross-language review, measured in sessions) and a **write/produce** mode. See
`code/specs/HL02-companion-practice-app.md`.
