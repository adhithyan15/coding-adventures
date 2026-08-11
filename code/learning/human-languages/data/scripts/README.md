# Scripts — teaching any writing system

Each `*.json` here describes one **writing system** in enough detail for the app
to teach reading it *and* to break every character into learnable pieces for
writing it by hand. The schema is deliberately general (spec:
[`HL01`](../../../../specs/HL01-concept-taxonomy-and-data-layer.md)) so the same
model teaches an alphabet, an abugida, or an abjad — and so **adding a new script
is data, not code.**

## The three families this schema covers

| Family | Examples | How vowels work | Direction |
|---|---|---|---|
| **alphabet** | Latin, **Cyrillic** (Russian), Greek | letters spell vowels + consonants | ltr |
| **abugida** | Devanagari, Bengali, Gujarati, Telugu, Kannada, Malayalam, Tamil | consonant carries an inherent vowel; a **mark** (mātrā) changes it; consonants stack into conjuncts | ltr |
| **abjad** | Arabic, **Hebrew** | consonants only; vowels are optional diacritic **marks**; letters take contextual **forms** (Hebrew: final-position variants) | **rtl** |
| **logographic** | **Chinese** (Hanzi) | no vowel letters at all; each character is a morpheme, its `sound` is tone-marked pinyin | ltr |

For **logographic** scripts a "letter" is a **character** (`role: "logograph"`) or a
recurring **radical** (`role: "radical"`); its `components` are the radicals/strokes
it decomposes into, `sound` carries the tone-marked pinyin, and `tone` optionally
holds the structured tone (`"1"`–`"4"` | `"neutral"`). No `forms`, no `marks`.

## Shape of a script file

```jsonc
{
  "script": "devanagari",        // id, matches the filename
  "name": "Devanagari",
  "font": "_fonts/NotoSansDevanagari-Static.ttf",
  "direction": "ltr",            // "ltr" | "rtl"
  "system": "abugida",           // "alphabet" | "abugida" | "abjad" | …
  "complete": false,             // true → the validator ENFORCES glyph coverage
  "combination": "how letters join / stack / take forms",
  "letters": [
    {
      "glyph": "क", "sound": "ka", "role": "consonant",
      "inherentVowel": "a",                    // abugida only
      "forms": { "isolated": "…", "initial": "…", "medial": "…", "final": "…" }, // cursive/abjad only
      "components": ["the pieces you draw, in words"],
      "strokeOrder": ["step 1", "step 2"],
      "strokeOrderNote": "conventional",       // never claimed as canonical
      "penLifts": 0,                           // OPTIONAL — only with a verified pen path
      "strokeOrderSource": {                   // OPTIONAL — required if penLifts is claimed
        "citation": "author, title, page/frame",
        "url": "https://…",
        "variation": "how much the order varies, and where"
      }
    }
  ],
  "marks": [                                   // vowel signs / harakat / niqqud
    { "mark": "ी", "sound": "ī", "role": "vowel-sign",
      "attachesAs": "a vertical to the right",
      "example": { "base": "न", "combined": "नी", "sound": "nī" } }
  ]
}
```

`components` is the point: each glyph is broken into named parts you can practise
one at a time on paper. `strokeOrder` puts those visible parts in their usual
writing order; it is not a count of pen-down strokes. Language Ladder labels every
uncited list **"Shape parts — usual order (pen lifts unverified)"**. Only a cited,
font-checked ductus may turn that into a verified pen path.

## Handwriting: parts, not the grid

The abugida files are large — Telugu, Kannada and Malayalam hold 455, 455 and 468
syllables each — and it would be a category error to author handwriting for all of
them. A syllable is not a shape the hand learns. It is an **assembly** of two shapes
the hand already knows: a **base consonant** and a **vowel sign**. క is a shape; కి
is క with the *i* sign on it.

So handwriting data follows one rule:

> Only **base consonants** and **vowel signs** are ever authored. A syllable's
> figure is composed from its parts' figures, never authored separately.

In a generated syllabary file the base consonants are the entries whose
`components` list has a single line (`క  ka — base consonant (inherent "a")`); the
vowel signs are the `marks`, where a file has them. Both inventories are small —
about 36 consonants and a dozen signs per script — which is what makes authoring
them by hand, from a cited source, tractable at all.


## `strokeOrder` lists PARTS. It must not imply pen lifts.

**Read this before writing a `strokeOrder`.** It is the one place in this schema
where a well-meaning entry can teach something false to a reader who has no way
to catch it.

A letter's *parts* and its *strokes* are different things:

| | what it counts | how a learner acts on it |
|---|---|---|
| **part** | a named piece of the finished shape ("the left upright") | *where* to put ink |
| **stroke** | one pen-DOWN run, start to lift | *whether the hand comes off the paper* |

One stroke can contain many named parts. `strokeOrder` counts **parts**. The app
renders it as a numbered list under the heading *"Write it — stroke order"*, and a
numbered list of three reads to almost everyone as *three strokes, so two lifts*.
That inference is **not** something the data has ever verified.

**The case that forced this section.** Tamil **ம** carried
`strokeOrder: ["left vertical", "bottom horizontal", "right arch"]` with no
citation — three items, and so read as two pen lifts. Language Ladder's
[`strokes.ts`](../../../../programs/typescript/language-ladder/src/strokes.ts)
independently carries an authored pen path for ம: **one unbroken stroke** of five
joined segments — *zero* lifts — cited to Radhakrishnan's *Tamil Script Learners
Manual* (Frame 1, UT Austin) and mechanically checked against the font outline
(every pen point lands on real ink; consecutive segments meet within 2 font units,
which is what "the parts connect, so nothing lifts" means numerically; the path
passes near all of the letter's ink).

The two were **not** in conflict about the ink or the direction of travel — the
prose was a coarser, three-way naming of the same left → bottom → right motion.
They conflicted about **pen lifts**, and only the ductus had evidence. So the
prose was rewritten to the cited five movements, each worded "without lifting",
and given the same citation.

**The rules that follow:**

1. Write each step so it cannot be misread as a lift. If a verified pen path says
   the hand stays down, say *"without lifting"* in the step itself — the heading
   and the list numbering will otherwise say the opposite.
2. Never infer `penLifts` from `strokeOrder.length`. Set `penLifts` **only** when
   an authored, font-checked pen path supports it, and pair it with
   `strokeOrderSource`. Absent means *not verified*, which is not *none*.
3. Where a ductus exists for a glyph, the ductus wins: it is cited and machine-
   checked, and the prose is not. Bring the prose to it, and copy its citation.
4. Where no ductus exists, do not invent lifts — keep the steps as part order and
   leave `penLifts` out.

Tamil **அ**, **ஆ**, **இ**, **க**, **வ**, **ல**, **ற**, **ன**, **ண**, and **ந** now join **ம** with authored ductus paths. The primer's
Frame 4 shows four connected movements for their shared curl-and-loop body,
followed by one lift and the separate right upright; ஆ then continues without
another lift into its long-vowel loop. Frame 4's next row gives இ five joined
inner-and-lower movements, one lift, then a joined outer-left climb and final
arch. The font-backed paths and learner prose preserve those orders exactly. The
Frame 3 row for க separates its upper frame and two lower bowls into three
pen-down runs with two verified lifts. Frame 9 joins வ's spiral body, bottom
bar, and right upright as five movements in one unbroken run; its next row
traces ல's outward spiral, middle descent, deep right-hand turn, and open tip
as four movements without lifting. Frame 10 joins ற's left arch to its first
middle descent, restarts for the adjacent descent, then joins the right arch to
the below-baseline sweep and descender: five movements, three runs, and two
lifts. Frame 13's first row joins ன's left spiral, single inner arch, and top
bar as five movements, then lifts once before the separate right upright. Its
next row keeps the analogous ண path joined through an extra inner arch and the
top bar for six movements before the same one lift and separate upright. Frame
12 gives ந three joined opening movements, one lift before its joined rising
middle stem and top bar, then a second lift before the right-hand descent. The
Tamil starter inventory is now fully verified. Persian **ا** adds the first
right-to-left-script ductus: UT Austin's freehand lesson shows one top-to-bottom
Naskh stem at 00:08–00:11, with no lift. The adjacent **ب** demonstration at
00:11–00:15 sweeps its shallow bowl right-to-left, then lifts once for the dot
below. The full-alphabet source then demonstrates Persian-added **پ** at
00:16–00:21; that letter remains deferred inventory work and does not change
HL-C09's fixed 228 prose entries. The next starter entry, **ت**, repeats the
right-to-left bowl at 00:22–00:27, lifts to place the left dot above, then lifts
again for the right dot. All three authored paths are checked against their
vendored isolated Noto Naskh outlines. The later **س** row at 01:29–01:35 keeps
its three right-to-left teeth and final bowl in one unbroken movement; its
two-part learner path has zero lifts and stays on the vendored isolated Noto
Naskh outline. The later **ل** demonstration at 02:29–02:32 descends its tall
upright and turns directly into the leftward base curve in the same unbroken
movement; its two-part learner path has zero lifts and remains on the vendored
isolated Noto Naskh outline. The source-adjacent **م** demonstration at
02:33–02:36 shapes its round head and flows directly into the descending tail
in the same unbroken movement; its two-part learner path has zero lifts and
remains on the vendored isolated Noto Naskh outline. The adjacent **ن**
demonstration at 02:37–02:43 supplies
one right-to-left bowl, then one lift for the dot above, with both strokes on
the isolated Noto Naskh outline. A source-sequence check then corrects the old
queue: **و**, not ه, follows ن. Its 02:43–02:45 demonstration loops the small
  head and flows directly into the leftward curving tail in one unbroken stroke;
  its two-part learner path has zero lifts and stays on the isolated Noto Naskh
  outline. The later **ه** demonstration at 02:47–02:50 closes one simple
  handwritten loop without lifting. Its one-movement learner path preserves
  that single pen-down run while fitting the wider two-counter isolated Noto
  Naskh form and its leftward baseline finish, completing the Persian starter
  inventory. Urdu **ا** is independently verified from Northwestern's *Zer o
  Zabar*: its independent form travels top-to-bottom in one continuous stroke,
  explicitly unlike the bottom-to-top final form. Script-aware ductus identity
  prevents that Urdu source from colliding with Persian **ا** while both paths
  are checked against their canonical data files' Noto Naskh fallback. Urdu
  **ج** follows from *Zer o Zabar*'s independent handwriting animation: place
  the dot below, lift once, then keep the pointed hooked head, descent, and bowl
  in one continuous run. The chapter's alternate flat head is purely aesthetic,
  so the learner path preserves the pointed form without inventing a different
  lift count. Urdu **ر** follows from the next source-backed chapter as one
  uninterrupted downward line that curves left. Its zero-lift learner path
  preserves the chapter's separate final-form motion and its Naskh/Nastaliq
  distinction without conflating them with the independent form. Urdu **س**
  follows from the source-adjacent chapter: both independent animations keep
  its three close teeth and final bowl in one right-to-left, zero-lift run. The
  learner path uses that standard toothed form while preserving the chapter's
  optional long gentle curve as an especially common handwriting alternative.
  Urdu **ش** follows in the same chapter: both independent animations complete
  that **س** body first, then place the lower-left, lower-right, and centered
  upper dots as three separately lifted strokes. The learner path preserves the
  chapter's two-below/one-above arrangement, centered dots, and optional
  toothless body. Urdu **ک** returns to Chapter 1: both independent animations
  and the prose write the main-line stem, flatter bowl, and pronounced final
  hook in one run, then lift once for the long downward slash from the upper
  right toward the stem. The learner path preserves the explicit warning not to
  write kāf in one penstroke. Urdu **ل** then returns to Chapter 2: both
  independent animations begin at the top, descend the tall upright, and keep
  the pen down while the line passes below the baseline through the leftward
  bowl and back up its outer side. The learner path preserves that zero-lift
  independent motion and the prose's connector and final-bowl distinctions.
  Urdu **م** advances to Chapter 3: both independent animations keep its round
  head and below-baseline tail in one zero-lift run. The prose distinguishes
  calligraphy from the constant-width handwritten counterclockwise loop, while
  the learner path reconciles their shared head-to-tail order with Noto Naskh.
  Urdu **ن** returns to Chapter 6: both independent animations draw the
  below-baseline bowl first, then lift once for the dot near the baseline. The
  learner path keeps the source's distinct initial/medial tooth form explicit.
  Urdu **ہ** returns to Chapter 4: both independent animations start at the
  upper right and close the oval-or-teardrop body as one counterclockwise loop
  with no lift. The learner path keeps the source's distinct initial/medial
  divot-and-mark forms and final up-and-down squiggle explicit. Urdu **ی** then
  follows in that chapter: both independent chhoṭī ye animations start at the
  upper right and keep the dotless S-shaped body and below-baseline bowl in one
  continuous sweep to the rising left tip. The learner path preserves the
  source's two dots as an initial/medial feature rather than inventing lifted
  marks on the independent form. Urdu **ں** returns to Chapter 6: both
  independent nūn-e ġhunna animations keep the same right-to-left,
  below-baseline bowl as **ن** in one zero-lift run. The prose identifies final
  and independent nūn-e ġhunna as nūn without its dot, while the initial and
  medial forms remain ordinary nūn; Noto Naskh's U+06BA contour exactly matches
  the U+0646 body with that dot removed. Urdu **ے** then completes the starter
  inventory from Chapter 4: both independent baṛī ye animations descend from
  the upper right, sweep left across the broad bowl, curl back underneath at
  the far left, and continue right along the lower fold in one zero-lift run.
  The prose's initial/medial tooth and independent/final sound distinctions
  remain explicit while the learner path follows Noto Naskh's folded contour.
  Arabic **ا** then opens the smallest remaining starter inventory from the
  University of Oregon's *Introduction to Arabic* video: the independent alif
  descends top-to-bottom in one uninterrupted 00:05–00:07 movement. Its
  zero-lift learner path stays on the isolated Noto Naskh outline, while a
  script-scoped key keeps its Arabic source distinct from Persian and Urdu ا;
  the adjacent lesson's one-way-connector label and printed final form remain
  explicit. The page's adjacent **ب** video then starts at the upper-right tip,
  sweeps continuously right-to-left through the shallow bowl, turns up at the
  left tip, and lifts once for the dot below. Its two-frame learner path stays
  on the isolated Noto Naskh outline, while Arabic-scoped provenance remains
  distinct from Persian ب and the lesson's two-way-connector context remains
  explicit. The later **ت** clip opens with that shared bowl already complete,
  then places the left and right dots above as two separate strokes. Its
  three-frame learner path therefore cites the page's Baa demonstration for the
  independently shown bowl and the Taa clip for the two lifted dots instead of
  inferring a hidden body motion. Arabic-scoped provenance remains distinct from
  Persian ت and the two-way-connector context stays explicit. The page's next
  link is labeled **ث**, but the underlying `taa.mp4` is another Taa lesson: its
  first form has only two upper dots, so ث remains unverified instead of gaining
  an invented third-dot order. The next viable **ج** video draws the short upper
  head left-to-right, continues down and around the bowl without lifting, then
  lifts once for the dot below. Its three-frame path preserves the lesson's
  two-way-connector context, Arabic-scoped provenance, and the deliberate
  contrast with Urdu's dot-first ج while following the isolated Noto Naskh
  outline. The page body does not link Haa, but its WordPress attachment ledger
  exposes `Haa.mov`. That clip opens while **ح**'s short left stem is already
  underway, visibly lengthens it downward through 00:00.15, then lifts once and
  restarts near its top at 00:00.32. The second run sweeps down-right and around
  the dotless bowl through 00:00.82. Its three-frame path therefore preserves a
  source-specific stem-first order instead of copying Jeem's body-first motion.
  The page's `kha.mov` then verifies a different order for **خ**: draw the short
  upper head left-to-right and continue around the bowl in one run from
  00:02.8–00:03.9, then lift once for the upper dot at 00:04.2–00:04.4. Its
  three-frame path follows Khaa's own body-first evidence rather than assuming
  either Jeem's lower dot or Haa's restarted stem. The next alphabet page's
  `letter-daal-2.mp4` verifies independent **د** at 00:07.0–00:07.6: begin at
  the upper tip, descend down-right through the curved shoulder, then turn left
  along the baseline without lifting. Its two-frame path preserves that
  zero-lift motion, one-way-connector context, and Arabic-scoped provenance.
  The same page's `raa.mp4` verifies independent **ر** at 00:08.8–00:09.3:
  begin at the upper tip, descend through the short stroke, then sweep left
  through the lower curve without lifting. Its two-frame path preserves that
  zero-lift motion, one-way-connector context, and Arabic-scoped provenance
  independently of Urdu ر. The next page's `FullSizeRender-8.mov` verifies
  independent **س** at 00:01.6–00:02.8: shape the three close teeth
  right-to-left, then flow directly into the final bowl without lifting. Its
  two-frame path preserves that continuous motion, two-way-connector context,
  and Arabic-scoped provenance independently of Persian and Urdu س. The page's
  `FullSizeRender-7.mov` then verifies independent **ش**: draw the same body in
  one run at 00:00.7–00:02.2, then lift for the lower-left, lower-right, and
  centered upper dots through 00:03.0. Its five-frame path preserves those
  three lifts, two-way-connector context, and Arabic-scoped provenance
  independently of Urdu ش. The page's `FullSizeRender-6.mov` then verifies
  independent **ص** in two runs: close the oval clockwise and rise into its
  short shoulder at 00:01.1–00:02.4, then lift once and restart at the baseline
  junction for the trailing bowl at 00:02.6–00:03.3. Its three-frame path keeps
  Saad's motion distinct from adjacent Seen and Shiin. The page's embedded Daad
  lesson then verifies **ض** independently at 00:43.1–00:46.3: repeat those two
  body runs, lift a second time, and place the upper dot last. Its four-frame
  path records that three-stroke order while the directly linked short MOV's
  audit-time HTTP 403 remains explicit. The next page's directly linked
  `ayn.mov` verifies independent **ع** in one run at 00:03.1–00:04.0: shape the
  open head from the upper-right tip, then continue down and around the lower
  bowl without lifting. Its two-frame path preserves zero lifts independently
  of adjacent Ghayn. The `Alphabet ي ك ل` page's directly linked `kaf.mov`
  then verifies independent **ك** in two runs at 00:11.8–00:13.4: descend the
  main upright and turn left along the baseline without lifting, then lift once
  and draw the inner arm from upper right down-left. Its three-frame path keeps
  Arabic Kaf distinct from Urdu **ک**'s different Unicode glyph and provenance.
  The same page's directly linked `lam.mov` verifies independent **ل** in one
  run at 00:01.9–00:02.4: descend the tall upright, turn left through the base
  bowl, and rise at its outer edge without lifting. Its two-frame path keeps
  Arabic Lam distinct from the Persian and Urdu records for the same Unicode
  glyph. The directly linked `yaa.mov` verifies independent **ي** at
  00:33.2–00:35.0: descend and sweep left through its shallow bowl without
  lifting, then place the lower-left and lower-right dots in separate runs.
  Its four-frame path keeps Arabic Yaa U+064A distinct from Urdu Ye U+06CC,
  whose independent body has no lower dots. The next **ه و ي** page's directly
  linked `letter-haa.mov` verifies independent **ه** in one run at
  00:04.9–00:06.0: close the lower counter, thread through the centre into the
  upper-right counter, then sweep left along the baseline without lifting. Its
  three-frame path fits that compact demonstration to the wider isolated Noto
  Naskh outline while keeping Arabic and Persian provenance separate. The same
  page's directly linked `waw.mov` verifies independent **و** in one run at
  00:45.7–00:46.9: sweep left from the lower-right junction to close the small
  head loop, then continue down and left through the tail without lifting. Its
  two-frame path preserves Waw's one-way-connector and w/long-ū roles while
  keeping Arabic and Persian provenance separate. Hebrew **א** then opens the
  next-smallest inventory from HebrewPod101's
  dedicated Alef lesson: draw the main diagonal down and right, lift once, then
  draw the opposing diagonal from the upper right through the crossing and down
  the lower-left leg. Its three-frame path fits that compact handwritten order
  to the vendored Noto Sans Hebrew block outline and records the variation
  explicitly. The same lesson's second, block-style **ב** then draws its top bar
  into the right descent, lifts once, and draws the baseline left-to-right. Its
  three-frame path excludes the separately placed optional dagesh from base
  U+05D1's lift count. The dedicated Gimel/Dalet lesson then verifies printed
  **ג**: its short top bar joins the right stem and short lower-right leg,
  then one lift precedes the longer diagonal leg down-left. The four-frame path
  follows the Noto Sans Hebrew outline while preserving the lesson's visibly
  different rounded cursive form as a documented variation. The
  source-adjacent cursive **ד** then sweeps one broad arch through a small
  loop and into its descending tail without lifting. Its two-frame path keeps
  that explicitly described "one curve" order while fitting Noto Sans Hebrew's
  angular top bar and right downstroke. The dedicated Hei lesson then verifies
  printed **ה**: its left-to-right top bar continues down the right side, then
  one lift precedes the detached left leg from top to bottom. Its three-frame
  path follows Noto Sans Hebrew's angular outline while preserving the lesson's
  explicitly contrasted curved handwritten form. The dedicated Vav lesson
  verifies printed **ו** as one stroke: draw the small head left-to-right, then
  turn directly down the stem without lifting. Its two-frame path follows Noto
  Sans Hebrew and excludes the lesson's later vowel marks from the base letter's
  zero-lift count. The Zayin/Heit lesson then verifies handwritten **ז** as one
  rounded run: rise briefly to the right, then curve down and around the base
  without lifting. Its two-frame block-font adaptation keeps Zayin distinct from
  both mirrored handwritten Gimel and the narrower Vav. The same lesson then
  verifies printed **ח**: draw the top bar left-to-right and continue down the
  right side, lift once, then draw the joined left leg from top to bottom. Its
  three-frame Noto Sans Hebrew path keeps the printed corners sharp while
  preserving the lesson's rounded handwritten form as a documented variation.
  The Tet/Yod lesson then verifies printed **ט** in two strokes: the left side
  descends and turns right along the base, then after one lift the right side
  climbs from the lower-right and turns down-left into the inward hook. Its
  four-frame Noto Sans Hebrew path preserves the source's unusual bottom-up,
  single-run rounded handwriting as a documented variation. The same lesson
  verifies printed **י** as one tiny run: draw its head left-to-right and turn
  directly down through the short stem. Its two-frame Noto Sans Hebrew path
  preserves the comma-like handwritten form and the source's small print angle.
  The dedicated Kaf lesson then verifies printed **כ** in one continuous run:
  draw the top bar left-to-right, turn down the rounded right side, and turn
  left along the base. Its three-frame Noto Sans Hebrew path preserves the
  source's rounded handwritten half-circle while sharpening the printed corners.
  The Lamed/Mem lesson next verifies printed **ל** in one tall run: descend the
  left stroke, continue right along the middle bar, then turn diagonally
  down-left. Its three-frame Noto Sans Hebrew path preserves the source's
  rounded looping handwriting while keeping the demonstrated angular order.
  The same lesson verifies printed **מ** in two runs: draw the detached angled
  left part, lift once, then climb through the upper shoulder, turn down the
  right side, and return left along the base. Its five-frame Noto Sans Hebrew
  path preserves the narrow N-like cursive form while keeping print's bottom-left
  gap explicit. Aural Writing's full-alphabet demonstration then verifies printed
  **נ** in one run: draw the small head left-to-right, continue down the right
  side, and turn left along the base. Its three-frame Noto Sans Hebrew path
  preserves the source's rounder purple cursive hook while replacing a queued
  expository video that never exposed pen order. The same source next verifies
  printed **ס** as one clockwise loop: draw the flat top left-to-right, round
  down the right side, sweep left along the base, then climb the left side to
  close without lifting. Its four-frame path preserves the adjacent purple
  cursive form's rounder oval while keeping the demonstrated zero-lift order.
  Printed **ע** then descends the right branch into the base, sweeps left, and
  turns back to climb the left branch without lifting. Its three-frame path
  preserves the adjacent purple cursive form's compact loop while fitting the
  demonstrated one-run order to Noto Sans Hebrew. Printed **פ** next draws its
  outer top, right side, and returning base in one run, lifts once, then adds
  the short inner curl left-to-right. Its four-frame Noto Sans Hebrew path
  preserves the adjacent purple cursive form's one-run inward spiral. The
  source demonstrates final Pe **ף** next; because it is already represented by
  `פ.forms.final`, the later Tsadi demonstration is the next counted entry.
  Printed **צ** then descends its long upper-left diagonal and turns left along
  the base in one run, lifts once, and curves the short upper-right arm down-left
  into the middle. Its three-frame Noto Sans Hebrew path preserves the adjacent
  purple cursive form's compact one-run shape. The source demonstrates final
  Tsadi **ץ** next; because it is already represented by `צ.forms.final`, the
  later Qof demonstration is the next counted entry. Printed **ק** then draws
  its top bar left-to-right and turns down-left through the right body in one
  run, lifts once, and descends the separate inner-left stem below the writing
  line. Its three-frame Noto Sans Hebrew path preserves the adjacent purple
  cursive form's one-run hooked descent. The adjacent Resh demonstration is the
  next counted entry. Printed **ר** then draws its top bar left-to-right, rounds
  the top-right corner, and continues down the right side without lifting. Its
  two-frame Noto Sans Hebrew path preserves the adjacent purple cursive form's
  rounder one-run hook. Printed **ש** next descends the right branch, rounds
  left through the base, and climbs the left branch in one run, then lifts once
  for the middle branch descending into the base. Its three-frame Noto Sans
  Hebrew path preserves the adjacent purple cursive form's compact one-run loop.
  Printed **ת** closes the Hebrew inventory: its top bar travels left-to-right
  and continues down the right side in one run, then one lift precedes the
  separate left leg and its small leftward foot. Its four-frame Noto Sans
  Hebrew path preserves the adjacent purple cursive form's one-run retracing
  stem and right arch. Chinese **人** then opens the smallest actionable
  inventory from Hanzi Writer Data's pinned, Make Me a Hanzi-derived PRC stroke
  record: draw the left-falling stroke, lift once, then draw the right-falling
  stroke. Its two-frame Noto Sans SC path preserves the source medians' order
  and direction while documenting the source font's different proportions.
  Chinese **亻** follows from its own pinned record: draw the long left-falling
  stroke, lift once, then descend the vertical from the central junction. Its
  two-frame path fits the narrow Noto Sans SC radical independently instead of
  mechanically squeezing 人. Chinese **口** next establishes a joined corner:
  descend the left side, lift for the top bar and right side in one continuous
  héngzhé run, then lift and close the bottom left-to-right. Its four-frame Noto
  Sans SC path makes the close-last rule visible. Chinese **女** follows with a
  bent first run: descend left, turn without lifting, and sweep down-right;
  lift for the separately left-falling stroke, then lift again for the middle
  horizontal. Its four-frame Noto Sans SC path keeps the source's three strokes
  and two lifts distinct. Chinese **子** then adds two joined turns: draw the
  top horizontal and sweep down-left without lifting, lift for the central
  descent and its leftward base hook, then lift again for the middle horizontal.
  Its five-frame Noto Sans SC path keeps both turns visible. Chinese **日**
  follows the same box family with one added inside bar: descend
  the left side, lift for the joined top-and-right héngzhé, lift for the middle
  horizontal, then lift once more to close the bottom. Its five-frame Noto Sans
  SC path makes the inside-before-close rule visible. Chinese **讠** follows
  with a down-right dot, then one lift before its horizontal, descending turn,
  and rising finish stay joined in a single second stroke. Its four-frame Noto
  Sans SC path keeps both source turns visible. The remaining **148** prose
  part orders across six scripts (`arabic` 3, `chinese` 17,
  `cyrillic` 33, `devanagari` 28,
  `gujarati` 33, `japanese` 34) are explicitly **unverified for pen lifts**.
  The data validator rejects a lift count without a citation (or a
  citation without a count), and Language Ladder's ductus test proves every
  verified claim has the same cited, font-checked path. That closes `HL-C19`;
  future entries join one side of this enforced boundary automatically.

**Telugu, Kannada and Malayalam currently have zero authored letters** and
therefore no `penLifts` anywhere. See [`BACKLOG.md`](../../BACKLOG.md) HL-C41 for
the source-availability finding that keeps it that way.

## Adding a script (e.g. Gujarati, Bengali, Hebrew)

Three data steps, **no code changes**:

1. **Author `<script>.json`** here, following the schema above. Pick the right
   `system` and `direction`; use `forms` for cursive/abjad letters and `marks`
   for vowel signs or diacritics. Start with `"complete": false` so coverage gaps
   are warnings while you build the inventory; flip to `true` when it's whole.
2. **Vendor the font** — drop the static Noto TTF into
   [`_fonts/`](../../_fonts/) and point the file's `font` at it.
3. **Point a track at it** — a language track selects its script either through
   the built-in map or a per-track `track.json` (`{ "script": "hebrew" }`), so a
   brand-new-script language needs no shared-map edit.

The `script` id is an open string end-to-end (`Script = string` in
`human-language-data`), so nothing about the type system has to know your script
in advance. Today's files: `devanagari.json` (abugida), `arabic.json` (abjad,
rtl). Reviewed and expanded incrementally.
