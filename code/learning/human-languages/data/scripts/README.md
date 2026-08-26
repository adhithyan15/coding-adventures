# Scripts — teaching any writing system

Each `*.json` here describes one **writing system** in enough detail for the app
to teach reading it *and* to break every character into learnable pieces for
writing it by hand. The schema is deliberately general (spec:
[`HL01`](../../../../specs/HL01-concept-taxonomy-and-data-layer.md)) so the same
model teaches an alphabet, an abugida, or an abjad — and so **adding a new script
is data, not code.**

## Safe regeneration of the Dravidian syllabaries

`generate_syllabary.py` rebuilds the Telugu, Kannada, and Malayalam Unicode
grids, but the committed JSON is also its merge boundary for later authored
work. Unicode owns only `glyph`, `sound`, `role`, and `inherentVowel`. Existing
components, sourced stroke data, notes, new evidence fields, core-external rows,
and downstream collections such as `marks` and `finalConsonants` survive the
rebuild. Missing output, malformed collections, and duplicate glyph identities
stop regeneration instead of allowing a destructive rewrite.

Run `python test_generate_syllabary.py` before regeneration. Its corpus test
rebuilds all three scripts in memory and proves that merging the committed
extensions is semantically idempotent. A subsequent
`python generate_syllabary.py` should leave all three tracked JSON files clean.

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
  "finalConsonants": [                       // atomic vowel-free letters outside a generated syllable grid
    { "glyph": "ൽ", "sound": "l", "role": "consonant", "components": ["…"],
      "strokeOrder": ["…"], "strokeOrderNote": "…" }
  ],
  "marks": [                                   // vowel signs / harakat / niqqud
    { "mark": "ी", "sound": "ī", "role": "vowel-sign",
      "attachesAs": "a vertical to the right",
      "example": { "base": "न", "combined": "नी", "sound": "nī" } }
  ]
}
```

## The letter ledger — what order to teach them in

A `<script>.json` says what the letters ARE. A `<script>-ledger.json` says what
order a reader meets them in, which is a different question with a much less
obvious answer.

Every one of these scripts has a traditional order — அ ஆ இ ஈ உ ஊ, अ आ इ ई उ ऊ —
organised by phonology, for a learner who already **speaks** the language and is
learning to write it. It front-loads independent vowels, which in an abugida
appear in relatively few words, because the vowel that does the work in running
text is the *sign* on a consonant. Measured against this corpus, **twelve glyphs
in recitation order complete zero words.**

This curriculum's reader is the opposite person: they cannot yet speak, and a
letter is worth exactly what it unlocks. So HL11 orders letters by **the words
they make writable**, and the ledger records that decision:

```jsonc
{
  "script": "tamil",
  "tracks": ["tamil"],          // Hindi and Sanskrit share the Devanagari ledger
  "openingWords": 30,           // distinct words in the first 40 lessons
  "letters": [
    { "position": 10,
      "glyph": "…", "codePoint": "U+0BB1", "unicodeName": "TAMIL LETTER RRA",
      "kind": "letter",
      "family": "…",            // letters that share a shape, taught together
      "familySource": "tamil.json notes: \"several letters share a straight top bar…\"",
      "unlocks": [ { "word": "…", "romanization": "thank you",
                     "lesson": "TA-C01-nandri" } ] }
  ]
}
```

Measured over the six tracks' opening lessons, ordering this way reaches
**நன்றி at the tenth Tamil glyph and வணக்கம் at the eleventh**; Devanagari
reaches नमस्ते at the twelfth. Roughly a third of each track's opening
vocabulary is writable within 24 positions.

`propose_letter_ledger.py` computes a candidate order and shows its work:

```bash
python3 propose_letter_ledger.py           # print the proposal for every script
python3 propose_letter_ledger.py --write   # write <script>-ledger.json
```

**The committed ledger is authored intent**, in the same sense `chapters.json`
is: a human reads the proposal, adjusts it, and commits the result. No validator
rewrites it. "Not yet decided" and "decided and recorded" are different states,
and a generator that overwrites the second erases the difference.

Two rules the payoff ordering does not get to override:

1. **A vowel sign may not precede the first base letter.** These are abugidas: a
   mark modifies a letter, and the vowel-killer removes a vowel a letter is
   carrying. A ledger opening on one describes a lesson that cannot be written
   down. This costs a position or two and is not negotiable.
2. **Letters that share a shape are taught together.** Splitting a family across
   the ledger to chase payoff trades a reading ramp for a writing confusion.
   Families are extracted *mechanically* wherever a letter's `components` name
   another letter of the same script (Devanagari records "ध: like द with an extra
   inner loop"); where a script file states one in prose instead, the ledger
   carries the sentence that justifies it.

Not one target-script character is typed into the generator. Glyphs are looked
up by their official Unicode **name**, the same grounding rule
`generate_syllabary.py` follows, so a maintainer who cannot read the script can
still audit every line.

Each row also carries its **code point**, because a rendered glyph is not an
audit surface: it can be a lookalike from another script, and it can carry code
points that render as nothing at all. `U+0BB1` beside `TAMIL LETTER RRA` is
checkable without trusting what the character looks like, and the validator
rejects a glyph that is more than one code point, a code point that disagrees
with the glyph, or a name from the wrong script.

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

> Only **independent vowels**, **atomic consonants** (base consonants and
> standalone chillus), and **vowel signs** are ever authored. A syllable's
> figure is composed from its parts' figures, never authored separately.

In a generated syllabary file independent vowels have their own small inventory;
the base consonants are the entries whose
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
second row of Frame 5 gives independent **எ** six connected movements through
its left climb, top bar, inner spiral, and lower foot, then one lift before the
separate right upright rises. This row is distinct from dependent **ெ** in
Frame 6, whose placement metadata makes no standalone ductus claim. The
independent Tamil **உ** follows Appendix I Frame 16's three joined movements:
the compact spiral opens into a broad descending curve, then continues along
the long baseline to the right without lifting. The Noto Sans Tamil fit keeps
all three movements in that single source-attested run. The
Tamil **ழ** adds Appendix I Frame 7's six numbered movements as three
pen-down runs: a joined left body and bar, a joined inner upright and broad
right bowl, and the detached lower hook. Noto Sans Tamil simplifies the
manual's looped left body and high bar to a retraced upright with a low
crossbar, so the font-fitted path preserves the source's run boundaries rather
than copying its display geometry literally. The Tamil starter inventory is
now fully verified. Persian **ا** adds the first
right-to-left-script ductus: UT Austin's freehand lesson shows one top-to-bottom
Naskh stem at 00:08–00:11, with no lift. The adjacent **ب** demonstration at
00:11–00:15 sweeps its shallow bowl right-to-left, then lifts once for the dot
below. The full-alphabet source then demonstrates Persian-added **پ** at
00:16–00:21: the same bowl comes first, followed by three separate dots below
in left, right, then lower-center order. This added corpus row does not change
HL-C09's fixed 228 prose entries. The next starter entry, **ت**, repeats the
right-to-left bowl at 00:22–00:27, lifts to place the left dot above, then lifts
  again for the right dot. The **خ** demonstration at 00:49–00:54 completes its
  short head and deep bowl before lifting once for the dot above; this row stays
  Persian-scoped rather than borrowing Arabic or Urdu provenance. The earlier
  **چ** demonstration at 00:35–00:41 draws that same body first, then lifts for
  the lower-left, lower-right, and lower-center dots. Its Persian-scoped source
  remains independent from Urdu **چ**, even though both use the same fitted Noto
  geometry. All five authored paths are checked against their vendored isolated
  Noto Naskh outlines.
  The **د** row at 01:04–01:06 folds from
its upper tip through the shoulder and left along the baseline without lifting.
The following **ر** row at 01:10–01:12 likewise keeps one pen-down run: descend
through the short stroke, then sweep left through the lower curve. Its source
remains Persian-scoped rather than borrowing the independently verified Arabic
or Urdu path for the same Unicode glyph. The later **س** row at 01:29–01:35 keeps
its three right-to-left teeth and final bowl in one unbroken movement; its
two-part learner path has zero lifts and stays on the vendored isolated Noto
Naskh outline. Persian **ف** at 02:09–02:13 loops its small closed head
clockwise, continues left through the broad bowl without lifting, then lifts
once for the upper dot. Its script-owned source remains distinct from the
separately verified Arabic and Urdu records for the same Unicode glyph. The
later **ل** demonstration at 02:29–02:32 descends its tall
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
  **پ** comes from the same chapter's independent handwriting animation: draw
  the be-series bowl in one right-to-left run, then place the lower-left and
  lower-right dots nearer the main line before the lower-center dot. The
  chapter's prose independently requires that triangular arrangement, and the
  script-owned source remains separate from Persian **پ** while both paths fit
  the same Noto Naskh fallback outline. Urdu **ھ** comes from *Zer o Zabar*'s
  independent calligraphic and handwriting animations: circle the right eye,
  continue left along the baseline, reverse at its far edge, circle the left
  eye, and complete the low leftward finish without lifting. The adjacent
  prose names the letter for its two eyes, explains its aspiration job, and
  warns about the direction reversal in medial and final forms. The learner
  path preserves that zero-lift order separately from chhoṭī he **ہ**. Urdu
  **چ** comes from *Zer o Zabar*'s independent calligraphic and handwriting
  animations: draw the pointed jīm-series body in one joined run, then place
  the lower-left, lower-right, and lower-center dots in that order. The prose
  confirms the three dots below the head and the *ch* sound; the script-owned
  path preserves this body-first order separately from **ج**. Urdu **ج**
  follows from *Zer o Zabar*'s independent handwriting animation: place
  the dot below, lift once, then keep the pointed hooked head, descent, and bowl
  in one continuous run. The chapter's alternate flat head is purely aesthetic,
  so the learner path preserves the pointed form without inventing a different
  lift count. Urdu **خ** follows from Chapter 7: its independent handwriting
  animation completes the pointed head and deep bowl in one body-first run, then
  lifts once for the dot above. The prose identifies it as the **ج** shape with
  its dot moved above, while the script-scoped row remains independent of Arabic
  and Persian. Urdu **ر** follows from the next source-backed chapter as one
  uninterrupted downward line that curves left. Its zero-lift learner path
  preserves the chapter's separate final-form motion and its Naskh/Nastaliq
  distinction without conflating them with the independent form. Urdu **و**
  follows in that same chapter: its independent handwriting animation shapes
  the looped head and continues down-left through the tail in one unbroken run.
  The learner path preserves the prose's handwritten loop, solid printed-head
  alternative, nonconnector status, and context-dependent consonant and vowel
  readings while keeping Urdu provenance separate from Arabic and Persian.
  Urdu **س**
  follows from the source-adjacent chapter: both independent animations keep
  its three close teeth and final bowl in one right-to-left, zero-lift run. The
  learner path uses that standard toothed form while preserving the chapter's
  optional long gentle curve as an especially common handwriting alternative.
  Urdu **ش** follows in the same chapter: both independent animations complete
  that **س** body first, then place the lower-left, lower-right, and centered
  upper dots as three separately lifted strokes. The learner path preserves the
  chapter's two-below/one-above arrangement, centered dots, and optional
  toothless body. Urdu **ف** then follows from Chapter 8: its independent
  handwriting animation loops the rounded head clockwise above the main line,
  continues left through the shallow curved tail without lifting, then places
  the single dot after one lift. The prose preserves the looped handwritten
  head, optional solid calligraphic head, and tail depth between kāf and nūn;
  Urdu provenance remains distinct from Persian and Arabic **ف**. Urdu **ک**
  returns to Chapter 1: both independent animations
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
  Sans SC path keeps both source turns visible. Chinese **氵** next draws two
  separately falling down-right dots, then lifts again for a bottom stroke
  that turns slightly up-left before sweeping to the upper right. Its
  four-frame Noto Sans SC path preserves all three source runs and keeps the
  bottom turn joined to its rise. Chinese **宀** then draws a down-right dot,
  lifts for its left-side down-left stroke, and lifts again before crossing
  the roof left-to-right and hooking down-left without breaking. Its four-frame
  Noto Sans SC path keeps the source's joined horizontal hook visible. The
  full character **你** follows: write 亻 first, then the five strokes of 尔,
  keeping its horizontal hook and central base hook joined before placing the
  two lower dots separately. Its nine-frame Noto Sans SC path preserves all
  seven source strokes and six lifts. **好** then writes all three strokes of
  女 before all three strokes of 子, preserving the joined turns in each
  component. Its nine-frame Noto Sans SC path preserves six source strokes and
  five lifts. **我** adds seven separately ordered strokes, keeping only its
  vertical and base hook joined; its nine-frame Noto Sans SC path also keeps
  the long curved slash's upward hook joined and preserves all six lifts. **是**
  then closes 日 in four strokes before adding five lower strokes; its ten-frame
  Noto Sans SC path preserves nine source strokes and eight lifts. **不** follows
  with four separately placed strokes: top horizontal, long falling stroke,
  central vertical, and right-falling dot. Its four-frame Noto Sans SC path
  preserves three lifts. **名** then completes 夕 in three strokes before drawing
  口 in three more; its eight-frame Noto Sans SC path preserves both joined turns
  and five lifts. **字** then completes 宀 before drawing 子; its nine-frame Noto
  Sans SC path preserves the roof hook, 子's two joined turns, and five lifts.
  **谢** writes 讠 before 身 and 寸 in twelve strokes; its seventeen-frame Noto
  Sans SC path preserves all five joined turns and eleven lifts. **请** writes 讠
  before 青 in ten strokes; its fourteen-frame Noto Sans SC path preserves all
  four joined turns and nine lifts. **再** builds its central frame before the
  long closing bottom horizontal; its eight-frame Noto Sans SC path preserves
  both joined turns and five lifts. **见** completes the open upper frame before
  its two lower runs; its seven-frame Noto Sans SC path preserves all three
  joined turns and three lifts. **什** then completes both separate strokes of
  亻 before writing 十's separately placed horizontal and vertical; its
  four-frame Noto Sans SC path preserves all three lifts. **么** follows with an
  upper left-falling stroke, a second fall joined to its rightward base sweep,
  and a final dot; its four-frame Noto Sans SC path preserves the joined turn
  and two lifts. **早** completes 日's four strokes before writing 十's
  horizontal and vertical below; its seven-frame Noto Sans SC path preserves
  the joined top-right turn and five lifts. **上** then descends its vertical,
  places the short middle horizontal, and finishes with the long base; its
  three-frame Noto Sans SC path preserves both lifts and completes Chinese.
  Devanagari **अ** then opens the next actionable inventory with a source audit:
  its upper curve continues around the lower bowl without lifting, then three
  lifted runs add the middle shoulder, top-to-bottom right stem, and
  left-to-right shirorekhā. Its five-frame Noto Sans Devanagari path follows the
  cited four-run modern printed form while recording a published six-stroke
  traditional Sanskrit form as real variation. **आ** preserves that joined left
  body, then separately adds the middle shoulder, inner stem, trailing stem,
  and left-to-right shirorekhā. Its six-frame Noto Sans Devanagari path follows
  the cited five-run modern printed form with four lifts while carrying the
  published base-letter variation forward. **इ** then descends its upright,
  turns through both bowls, and finishes down-right through the tail without
  lifting before a separate left-to-right shirorekhā. Its five-frame Noto Sans
  Devanagari path follows the cited two-run modern printed form with one lift.
  **ई** reuses that continuous body, then separately sweeps its upper curl
  upward and around before adding the left-to-right shirorekhā. Its six-frame
  Noto Sans Devanagari path follows the cited three-run form with two lifts.
  **उ** then curves down and left around its upper bowl before sweeping back
  through the waist and around the lower loop in the same run; a separate
  left-to-right shirorekhā follows. Its three-frame Noto Sans Devanagari path
  follows the cited two-run form with one lift. **ऊ** reuses that body, then
  separately sweeps its right-hand loop upward, around, and down-left before
  adding the left-to-right shirorekhā. Its four-frame Noto Sans Devanagari path
  follows the cited three-run form with two lifts. **ए** then joins its long
  left stem to the curved shoulder and descending tail, separately descends the
  shorter right stem into its inward hook, and finishes with the shirorekhā. Its
  four-frame Noto Sans Devanagari path follows the cited three-run form with two
  lifts. **ऐ** reuses both base strokes, then separately sweeps its upper arc
  upward and left before adding the left-to-right shirorekhā. Its five-frame
  Noto Sans Devanagari path follows the cited four-run form with three lifts.
  **ओ** reuses आ's joined left body, separate shoulder, and two stems, then
  separately sweeps its upper arc upward and left before adding the
  left-to-right shirorekhā. Its seven-frame Noto Sans Devanagari path follows
  the cited six-run form with five lifts. **औ** reuses the same four-stroke
  base, then separately sweeps its lower and taller upper arcs upward and left
  before adding the left-to-right shirorekhā. Its eight-frame Noto Sans
  Devanagari path follows the cited seven-run form with six lifts. **क** opens
  the consonants from the older animated source collection: its left bowl
  starts at the upper-right junction and circles counterclockwise before three
  lifted runs descend the central stem, sweep the right-hand arch clockwise,
  and finish the shirorekhā left-to-right. Its four-frame Noto Sans Devanagari
  path follows the cited four-run animation with three lifts, corroborated by
  the Central Hindi Directorate's four-part learner buildup. **ग** starts at
  the loop's upper-right junction, circles counterclockwise, and carries that
  first run up the joined stem before two lifted runs descend the right stem
  and finish the shirorekhā left-to-right. Its three-frame Noto Sans
  Devanagari path follows the cited three-run animation with two lifts,
  corroborated by the Central Hindi Directorate's three-part learner buildup.
  **च** then draws its short upper bar left-to-right and turns directly through
  the shoulder into the rounded open body before two lifted runs descend the
  right stem and finish the shirorekhā. Its three-frame Noto Sans Devanagari
  path follows the cited three-run animation with two lifts. The Directorate's
  deskbook confirms component order but stages the upper bar and body
  separately, so it corroborates order rather than the animation's first join.
  **त** starts at the body's upper-right junction, sweeps left across the
  shoulder, and curves down to its open lower tip before two lifted runs
  descend the right stem and finish the shirorekhā. Its three-frame Noto Sans
  Devanagari path follows the cited three-run animation with two lifts,
  independently corroborated by the Directorate's three-part learner buildup.
  **द** then descends its short stem, lifts for one continuous sweep around the
  outer body, inward curl, and down-right tail, and lifts again for the
  shirorekhā. Its three-frame Noto Sans Devanagari path follows the animated
  three-run form with two lifts. The Directorate's deskbook confirms component
  order but stages the outer body and curl-tail separately, so it corroborates
  order rather than the animation's body-to-curl join. **ध** then curls around
  the upper spiral and sweeps right through its shoulder before three lifted
  runs draw the lower bowl, descend the right stem, and finish the shirorekhā.
  Its four-frame Noto Sans Devanagari path follows the animated four-run form
  with three lifts, independently corroborated by the Directorate's matching
  four-part learner buildup. **न** then circles clockwise around its left loop
  and continues right along the shoulder before two lifted runs descend the
  right stem and finish the shirorekhā. Its three-frame Noto Sans Devanagari
  path follows the animated three-run form with two lifts, independently
  corroborated by the Directorate's matching three-part learner buildup and
  directions. **प** then descends its left stem and curves right around the
  lower bowl before two lifted runs descend the right stem and finish the
  shirorekhā. Its three-frame Noto Sans Devanagari path follows the animated
  three-run form with two lifts, independently corroborated by the
  Directorate's matching three-part learner buildup and directions. **ब** then
  circles counterclockwise around its oval body before three lifted runs
  descend the right stem, cross the body down-right, and finish the
  shirorekhā. Its four-frame Noto Sans Devanagari path follows the animated
  four-run form with three lifts, independently corroborated by the
  Directorate's matching four-part learner buildup and directions. **भ** then
  starts at the upper loop's lower inner tip, sweeps left and clockwise around
  that loop, descends its joined trunk, curls clockwise around the lower bowl,
  and continues right through the crossbar before two lifted runs descend the
  right stem and finish the shirorekhā. Its three-frame Noto Sans Devanagari
  path follows the animated three-run form with two lifts. The Directorate's
  deskbook confirms component order but stages the upper loop and trunk before
  the lower bowl and crossbar, so it corroborates order rather than the
  animation's continuous body join. **म** then descends the left stem, curls
  left and clockwise around the lower loop, and continues right through the
  crossbar before two lifted runs descend the right stem and finish the
  shirorekhā. Its three-frame Noto Sans Devanagari path follows the animated
  three-run form with two lifts. The Directorate's deskbook confirms component
  order but stages the left stem before the lower loop and crossbar, so it
  corroborates order rather than the animation's continuous join. The remaining
  **य** then curves clockwise around its inner curl before three lifted runs
  curve down and right around the lower bowl, descend the right stem, and finish
  the shirorekhā. Its four-frame Noto Sans Devanagari path follows Opiaterein's
  animated four-run form with three lifts, independently corroborated by the
  Directorate's matching four-part buildup and directions. JackPotte's separate
  11-frame animation joins the inner curl and lower bowl, so that documented
  three-run alternative remains explicit. **र** then descends its stem and
  curls left and clockwise around the lower loop before two lifted runs draw
  the diagonal tail down-right and finish the shirorekhā. Its three-frame Noto
  Sans Devanagari path follows Opiaterein's animated three-run form with two
  lifts, independently corroborated by the Directorate's matching three-part
  buildup and directions. JackPotte's separate seven-frame animation joins the
  descending stem, lower loop, and tail, so that documented two-run alternative
  remains explicit. **ल** then curves up and clockwise around its open left loop
  before three lifted runs sweep the diagonal arm up-right, descend the right
  stem, and finish the shirorekhā. Its four-frame Noto Sans Devanagari path
  follows Opiaterein's animated loop-first form with three lifts, independently
  corroborated by the Directorate's matching four-part buildup and directions.
  JackPotte's separate 12-frame animation instead orders the right stem,
  diagonal arm, left loop, and headline, so that documented stem-first
  alternative remains explicit. **व** then starts at the upper-right of its
  body, travels left around the top, and continues counterclockwise around the
  loop before two lifted runs descend the right stem and finish the
  shirorekhā. Its three-frame Noto Sans Devanagari path follows JackPotte's
  11-frame animation with two lifts. The Directorate's deskbook independently
  confirms the same loop, right-stem, and headline buildup, while the animation
  supplies the within-run directions and lift evidence. **श** then starts at
  the upper loop's lower inner tip, sweeps left and clockwise around that loop,
  descends through the outer curve, curls around the lower loop, and continues
  down-right through the diagonal tail before two lifted runs descend the right
  stem and finish the shirorekhā. Its three-frame Noto Sans Devanagari path
  follows Opiaterein's 25-frame animation with two lifts. JackPotte's separate
  26-frame animation and the Directorate's deskbook independently corroborate
  the same joined-body, right-stem, and headline buildup. **स** then descends
  its left stem, curls left around the central hook, and continues down-right
  through the diagonal tail before three lifted runs draw the middle crossbar,
  descend the right stem, and finish the shirorekhā. Its four-frame Noto Sans
  Devanagari path follows JackPotte's 13-frame animation with three lifts. The
  Directorate's deskbook confirms component order but stages the left curve and
  diagonal tail separately, so it corroborates order rather than the animation's
  continuous join. **ह** then descends its right stem, sweeps left through the
  shoulder, and curves clockwise around the hooked body before two lifted runs
  sweep down-left and then down-right through the outer tail and finish the
  shirorekhā. Its three-frame Noto Sans Devanagari path follows Opiaterein's
  22-frame animation with two lifts. The Directorate's deskbook confirms the
  same component order but stages the joined first body across more buildup
  steps, so the animation supplies the three-run lift evidence. Devanagari's
  starter inventory is now source-verified. Cyrillic then breaks its tie with
  Gujarati: RussianIrina's native-teacher all-letter lesson demonstrates
  lowercase **а** at 00:50–00:55 as a rounded body flowing directly into its
  right-hand finishing stem, with no intervening lift. Its two-frame Noto Sans
  Cyrillic path preserves that one-run school-hand motion while fitting the
  source's single-storey form through the font's extra double-storey printed
  shoulder. The same lesson demonstrates lowercase **б** at 01:13–01:18:
  its counterclockwise lower body closes before the pen rises into the
  rightward top flag, again without lifting. The two-frame font fit preserves
  that one-run order while routing the handwritten diagonal transition through
  Noto Sans Cyrillic's printed upper-left shoulder. Lowercase **в** follows at
  01:33–01:38: the pen starts at the baseline, climbs through its tall upper
  loop, descends to the baseline, and continues counterclockwise around the
  lower bowl without lifting. Its two-frame font fit preserves that one-run
  order while routing the cursive ascender through Noto Sans Cyrillic's compact
  printed upper bowl and straight left stem. Lowercase **г** follows at
  01:54–01:57 as one rounded two-hump cursive run: rise from the baseline,
  descend and turn there, then continue through a smaller exit arch without
  lifting. Its two-frame Noto fit preserves that zero-lift evidence by climbing
  the block glyph's upright, sweeping and retracing its top bar, and descending;
  it records that the printed form omits the cursive exit arch. Lowercase **д**
  follows at 02:14–02:19: its rounded body closes counterclockwise before the
  same run descends below the baseline, loops left, and rises into a rightward
  exit. Its two-frame Noto fit preserves that body-before-descender order and
  zero-lift evidence while tracing the block glyph's trapezoidal body, joined
  base shelf, and two retraced feet; it records that the printed form replaces
  the cursive descender loop with those feet. Lowercase **е** follows at
  02:26–02:30: the pen begins at the upper right, curves left around the upper
  loop, crosses through the middle, and continues counterclockwise around the
  rounded lower bowl without lifting. Its two-frame Noto fit preserves that
  upper-loop-to-middle-to-lower-bowl order while routing the tall open school
  hand through the compact printed glyph's upper bowl and long middle bar.
  Lowercase **ё** follows at 02:51–02:56: the pen completes that same looped
  body first, then lifts for the left dot and lifts again for the right dot.
  Its four-frame Noto fit preserves the source's body-before-left-dot-before-
  right-dot order and two-lift evidence while tracing the compact printed e
  body and both circular dots. Lowercase **ж** follows at 03:16–03:21: the pen
  rises from the lower left through a rounded left arch and tall central loop,
  descends through the middle, continues into a rounded right arch, and
  finishes through a smaller rightward exit without lifting. Its two-frame
  Noto fit preserves that left-to-centre-to-right order and zero-lift evidence
  while tracing the printed glyph's straight central upright and four diagonal
  arms. Lowercase **з** follows at 03:34–03:39: the pen circles the smaller
  upper lobe to the right, descends through the middle, and continues around
  the larger lower lobe into a rising exit without lifting. Its two-frame Noto
  fit preserves that upper-lobe-to-lower-lobe order and zero-lift evidence
  while tracing the compact printed double-lobe glyph; it records that the
  printed form omits the school hand's exit join. Lowercase **и** follows at
  03:56–04:02: the pen descends the left stem, turns directly into a rising
  diagonal, descends the right stem, and finishes through a small rising exit
  without lifting. Its three-frame Noto fit preserves that stem-to-diagonal-to-
  stem order and zero-lift evidence while tracing the printed backwards-N
  glyph; it records that the printed form omits the school hand's rounded entry
  and exit joins. Lowercase **й** follows at 04:17–04:24: the pen completes the
  same joined body as **и**, lifts once, then draws the breve from left to right
  as one dipped arc. Its four-frame Noto fit preserves that body-before-breve
  order, left-to-right breve direction, and one-lift evidence while tracing the
  printed backwards-N body and separate curved mark. Lowercase **к** follows at
  04:45–04:51: the pen descends the left stem, rises through a looped upper-right
  arm and returns to the middle, then continues through the lower-right arm and
  a small rising exit without lifting. Its three-frame Noto fit preserves that
  stem-to-upper-arm-to-lower-arm order and zero-lift evidence while tracing the
  printed vertical and two angular diagonals; it records that the school hand
  rounds the upper arm and carries entry and exit joins. Lowercase **л** follows
  at 05:06–05:10: the pen curves left around a small baseline hook, rises
  steeply to a high apex, descends through the right leg, and finishes through
  a small rising exit without lifting. Its three-frame Noto fit preserves that
  hooked-left-leg-to-apex-to-right-leg order and zero-lift evidence while
  tracing the printed curved left leg, horizontal top shoulder, and straight
  right stem; it records the school hand's pointed apex, slanted right leg, and
  entry and exit joins. Lowercase **м** follows at 05:26–05:31: the pen curves
  left around a small entry hook, rises to the first apex, descends through the
  central valley, rises to the second apex, descends through the right leg, and
  finishes through a small rising exit without lifting. Its four-frame Noto fit
  preserves that entry-to-first-apex-to-valley-to-second-apex-to-baseline order
  and zero-lift evidence while tracing the printed straight upright stems and
  deep central V; it records the school hand's rounded arches and entry and exit
  joins. Lowercase **н** follows at 05:47–05:52: the pen descends the left stem,
  turns upward through a rounded middle bridge, rises to the right shoulder,
  descends the right stem, and finishes through a small rising exit without
  lifting. Its three-frame Noto fit preserves that left-stem-to-middle-bridge-
  to-right-stem order and zero-lift evidence while tracing the printed straight
  verticals and horizontal middle bar; it records the school hand's rounded
  bridge and entry and exit joins. Lowercase **о** follows at 05:59–06:03: the
  pen begins at the upper right, curves left across the top, descends the left
  side, sweeps through the bottom, rises along the right side, and closes the
  oval without lifting. Its two-frame Noto fit preserves that counterclockwise
  closure order and zero-lift evidence while tracing the printed wider upright
  oval; it records the school hand's taller, slightly slanted proportions.
  Lowercase **п** follows at 06:26–06:31: the pen descends the left stem, turns
  upward through a rounded top shoulder, descends the right stem, and finishes
  through a small rising exit without lifting. Its three-frame Noto fit
  preserves that left-stem-to-top-shoulder-to-right-stem order and zero-lift
  evidence while tracing the printed straight uprights and horizontal top bar;
  it records the school hand's rounded Latin-n-like shape and entry and exit
  joins. Lowercase **р** follows at 06:46–06:52: the pen descends its stem below
  the baseline, retraces upward through the same stem, curves right through a
  rounded shoulder, descends to the baseline, and finishes through a small
  rising exit without lifting. Its three-frame Noto fit preserves that
  stem-before-bowl order and zero-lift evidence while tracing the printed
  straight descender and closed rounded bowl; it records the school hand's open
  long-descender Latin-p-like shape and entry and exit joins. Lowercase **с**
  follows at 07:04–07:08: the pen begins at the upper right, curves left across
  the top, descends the left side, sweeps through the bottom, and rises into a
  small lower-right exit without lifting. Its two-frame Noto fit preserves that
  counterclockwise open-curve order and zero-lift evidence while tracing the
  printed wider upright C-like outline; it records the school hand's tall,
  slightly slanted proportions and rising exit. Lowercase **т** follows at
  07:29–07:36: the pen descends the left stem, rises through a rounded first
  arch, descends the middle stem, rises through a rounded second arch, descends
  the right stem, and finishes through a small rising exit without lifting. Its
  three-frame Noto fit preserves that initial-descent-before-joined-top-
  movements order and zero-lift evidence while tracing the printed central stem
  and horizontal top bar; it records the school hand's two-arch Latin-m-like
  shape and rising exit. Lowercase **у** follows at 07:50–07:55: the pen
  descends the left arm, rises through the right arm, retraces into a long
  descender, curls left through a lower loop, crosses the descender, and rises
  into a short exit without lifting. Its four-frame Noto fit preserves that
  left-arm-to-right-arm-to-descender order and zero-lift evidence while tracing
  the printed straight upper arms and broad left-curving terminal; it records
  the school hand's loop-descender Latin-y-like shape and rising exit. Lowercase
  **ф** follows at 08:16–08:26: the pen descends the long central stem below the
  baseline, lifts once, restarts near the upper junction, circles the left
  loop, crosses the stem, circles the right loop, and finishes through a small
  rising exit. Its five-frame Noto fit preserves that stem-before-left-loop-
  before-right-loop order and one-lift evidence while tracing the printed
  straight ascender-descender and two wider upright bowls; it records the
  school hand's narrower linked loops and rising exit. Lowercase **х** follows
  at 08:42–08:49: the pen draws a right-bulging left curve from the upper left
  through the middle crossing to a lower-left terminal, lifts once, then draws
  a left-bulging right curve from the upper right through the same crossing to
  a small lower-right exit. Its four-frame Noto fit preserves that left-run-
  before-right-run order and one-lift evidence while straightening the facing
  curves into the printed glyph's four diagonal arms; it records the school
  hand's rounded curves and rising exit. Lowercase **ц** follows at
  09:05–09:10: the pen descends the left stem, turns through a rounded join,
  rises and descends the right stem, then continues directly into a small lower
  tail loop and rising exit without lifting. Its four-frame Noto fit preserves
  that left-stem-to-right-stem-to-tail order and zero-lift evidence while
  tracing the printed squared U-like body, horizontal bottom bar, and short
  right descender; it records the school hand's rounded diagonal join and
  looped exit. Lowercase **ч** follows at 09:24–09:28: the pen descends the
  short left stem, turns through a rounded join, rises to the top of the right
  stem, descends that full stem, and curls into a rising exit without lifting.
  Its three-frame Noto fit preserves that short-stem-to-bowl-to-long-stem order
  and zero-lift evidence while tracing the printed shorter left stem, shallow
  rounded bowl, and full-height right stem; it records the school hand's
  narrower bridge, curled baseline, and rising exit. Lowercase **ш** follows at
  09:49–09:57: the pen descends the left stem, rises through a rounded first
  join, descends the middle stem, rises through a rounded second join, descends
  the right stem, and curls into a rising exit without lifting. Its five-frame
  Noto fit preserves that left-to-middle-to-right order and zero-lift evidence
  while tracing the printed three straight stems and two horizontal baseline
  joins; it records the school hand's diagonal rounded joins, curled baseline,
  and rising exit. Lowercase **щ** follows at 10:17–10:25: the pen descends the
  left stem, rises and descends through the joined middle and right stems, then
  continues directly into a small lower tail loop and rising exit without
  lifting. Its six-frame Noto fit preserves that
  left-to-middle-to-right-to-tail order and zero-lift evidence while tracing
  the printed three straight stems, two horizontal baseline joins, and short
  right descender; it records the school hand's diagonal rounded joins and
  looped exit. Lowercase **ъ** follows at 10:34–10:38: the pen curls through a
  narrow entry loop, sweeps through the rounded top shoulder, descends the main
  stem, then circles the joined lower bowl counterclockwise and closes it
  without lifting. Its five-frame Noto fit preserves that
  flag-to-stem-to-bowl order and zero-lift evidence while tracing the printed
  broad horizontal top flag, straight main stem, and closed lower bowl; it
  records the school hand's looped entry and rounded shoulder. Lowercase **ы**
  follows at 10:45–10:56: the pen curls through a narrow entry loop, descends
  the left stem, circles the joined lower bowl counterclockwise and closes it,
  lifts once, then descends the separate right stem into a rising exit. Its
  five-frame Noto fit preserves that body-before-right-stem order and one-lift
  evidence while tracing the printed straight left upright, wide closed lower
  bowl, and separate straight right stem; it records the school hand's looped
  entry and curled exit. Lowercase **ь** follows at 11:16–11:20: the pen
  descends the stem, turns at the baseline, circles the joined lower bowl
  counterclockwise, and closes it against the stem without lifting. Its
  four-frame Noto fit preserves that zero-lift stem-to-bowl order while tracing
  the printed straight upright and closed lower bowl; it records the school
  hand's narrow entry stroke and rounded bowl. Lowercase **э** follows at
  11:25–11:32: the pen draws the outer backwards-C curve from upper left around
  the right side to lower left, lifts once, then draws the middle tongue from
  right to left. Its four-frame Noto fit preserves that outer-before-tongue
  order and one-lift evidence while tracing the printed broad open-left curve
  and straight middle bar; it records the school hand's narrower curve and
  hooked tongue. Lowercase **ю** follows at 11:44–11:58: the pen descends the
  left stem, turns through a rising connector, and continues clockwise around
  the right oval to close without lifting. Its five-frame Noto fit preserves
  that zero-lift stem-to-connector-to-oval order while tracing the printed
  straight left upright, horizontal middle bar, and wide closed oval; it
  records the school hand's looped entry, diagonal connector, and cursive oval.
  Lowercase **я** closes the Cyrillic lowercase inventory at 12:13–12:21: the
  pen rises from a curved baseline entry, circles the upper loop
  counterclockwise, descends the long diagonal leg, and turns into a short exit
  without lifting. Its four-frame Noto fit preserves that zero-lift
  rise-to-loop-to-leg order while tracing the printed straight right upright,
  broad upper bowl, and angular lower-left leg; it records the school hand's
  curved entry, narrow loop, slanted leg, and exit join. Gujarati **અ** then
  opens the next actionable inventory: t30apps.com's version-1.0 animation
  writes the joined left curve, lower body, middle shoulder, and small right
  arch first, lifts once, then descends the separate right stem into its foot.
  Its four-frame Noto Sans Gujarati path preserves that body-before-right-stem
  order and one-lift evidence while fitting the font's broader printed
  proportions. The source and learner notes retain the app's explicit warning
  that its depicted form and order are one variant, not a universal standard.
  Source-adjacent Gujarati **આ** repeats that full sequence, then lifts a
  second time to descend the added trailing ā stem. Its five-frame Noto Sans
  Gujarati path preserves the animation's body-before-first-stem-before-
  trailing-stem order and two-lift evidence while fitting the printed glyph's
  wider stem spacing. The same variation warning remains explicit. The
  remaining **68**
  prose part orders across three scripts
  (`arabic` 3,
  `gujarati` 31, `japanese` 34) are explicitly **unverified for pen lifts**.
  The data validator rejects a lift count without a citation (or a
  citation without a count), and Language Ladder's ductus test proves every
  verified claim has the same cited, font-checked path. That closes `HL-C19`;
  future entries join one side of this enforced boundary automatically.

The initial source-availability gap recorded by HL-C41 is now being closed one
verified shape at a time. Telugu has a cited independent **అ** row; Kannada has
a cited independent **ಅ** row; Malayalam has cited independent **അ** and **എ**
rows, base consonant **ഴ**, and standalone chillus **ൽ** and **ൻ**. An absent
`penLifts` remains unverified, never an implied zero.

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
