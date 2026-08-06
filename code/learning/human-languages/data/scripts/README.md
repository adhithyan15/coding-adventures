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
      "strokeOrderNote": "conventional"        // never claimed as canonical
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

### `penLifts` absent means NOT VERIFIED

Two different things get confused here constantly, so they are named apart:

| Field | What it records | Where it comes from |
|---|---|---|
| `strokeOrder` | the **parts** a reader can see in the finished letter, in the order they are taught | a cited teaching source; prose |
| `penLifts` | how many times the hand **leaves the paper** | only a font-checked pen path with cited provenance |

A prose step list says nothing about lifts. "Bar, loop, two arches, vertical" is
four *parts*; it may be one unbroken motion or four separate ones, and the list
cannot tell you which. So:

- **`penLifts` absent means NOT VERIFIED.** It never means *none*.
- **`penLifts` must never be inferred from `strokeOrder.length`.** That inference
  reads a claim about parts as a claim about the hand, which is exactly the error
  the two fields exist to keep apart.

A letter earns a `penLifts` only when a pen path has been authored for it in
`language-ladder`'s `strokes.ts` and has passed that module's three font
invariants (every pen point on real glyph ink, consecutive segments meeting within
2 font units, the path passing near all the letter's ink) **with a non-empty
citation and URL**. Where no citable source for the order exists, the letter is
simply not authored. Fewer letters, honestly, over more letters, invented.

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
