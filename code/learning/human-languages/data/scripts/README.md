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
  The remaining **195** prose part orders across seven scripts (`arabic` 21,
  `chinese` 24, `cyrillic` 33, `devanagari` 28, `gujarati` 33, `hebrew` 22,
  `japanese` 34) are explicitly **unverified for pen lifts**.
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
