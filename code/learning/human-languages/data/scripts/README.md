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
| **alphabet** | Latin, Greek, Cyrillic | letters spell vowels + consonants | ltr |
| **abugida** | Devanagari, Bengali, Gujarati, Telugu, Kannada, Malayalam, Tamil | consonant carries an inherent vowel; a **mark** (mātrā) changes it; consonants stack into conjuncts | ltr |
| **abjad** | Arabic, Hebrew | consonants only; vowels are optional diacritic **marks**; letters take contextual **forms** | **rtl** |

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
