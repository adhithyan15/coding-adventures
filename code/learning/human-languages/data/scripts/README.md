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

`components` is the point: each glyph broken into named parts you can practise one
at a time on paper. `strokeOrder` is the common handwriting convention and is
always flagged as such — freely-licensed authoritative stroke data does not exist
for most of these scripts.

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

Only **ம** currently has an authored ductus. The remaining prose stroke orders —
192 letters across nine scripts (`arabic` 21, `chinese` 24, `cyrillic` 33,
`devanagari` 28, `gujarati` 31, `hebrew` 22, `perso-arabic` 9, `tamil` 10,
`urdu-nastaliq` 13) — are **unverified for pen lifts** and are tracked as
`HL-C19` in the [backlog](../../BACKLOG.md).

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
