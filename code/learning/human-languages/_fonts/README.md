# Vendored fonts (non-Latin scripts)

These are **static** files of Google's [Noto](https://fonts.google.com/noto)
fonts, vendored so the non-Latin-script books — including Arabic, Urdu,
Devanagari, Dravidian, Cyrillic, Hebrew, Chinese, and Japanese tracks — compile
**identically** on any machine and in CI, with no dependency on whatever font
packages happen to be installed. Unless a Bold face is named explicitly below,
the file is Regular weight.

- `NotoNaskhArabic-Static.ttf` — Arabic and the accessibility fallback for
  Urdu when its course face cannot load
- `NotoNastaliqUrdu-Static.ttf` and `NotoNastaliqUrdu-Bold-Static.ttf` — Urdu
  Regular and Bold, upstream static TTFs from the
  [official Noto distribution](https://github.com/notofonts/notofonts.github.io/tree/46074e15f8956b502051eb4a7796ed8c7d4f3076/fonts/NotoNastaliqUrdu/full/ttf)
  at commit `46074e15f8956b502051eb4a7796ed8c7d4f3076`. SHA-256:
  `06f5fe0febcbab39be2e338758eb8f8dc8a887f833851c9ee4051b4324e44801`
  (Regular) and
  `1bd71f39445c6af6af8605165a5fdd91d0271328b6cc04b8cbaccb5e7b700cbf`
  (Bold). Both report Noto Nastaliq Urdu version 4.000 and carry the Urdu
  contextual-shaping and localization tables.
- `NotoSansDevanagari-Static.ttf` — Hindi **and Marathi** (both Devanagari)
- `NotoSansTamil-Static.ttf` — Tamil
- `NotoSansKannada-Static.ttf` — Kannada
- `NotoSansTelugu-Static.ttf` — Telugu
- `NotoSansMalayalam-Static.ttf` — Malayalam
- `NotoSansGurmukhi-Static.ttf` — Punjabi (Gurmukhi)
- `NotoSansBengali-Static.ttf` — Bengali
- `NotoSansGujarati-Static.ttf` — Gujarati (the "headless" script — Devanagari without the top line)
- `NotoSansCyrillic-Static.ttf` — Russian (Cyrillic); a `fontTools.subset` of NotoSans (latin-greek-cyrillic) to Basic Latin + Cyrillic. **It carries no combining diacritics**: the subset stops at the Cyrillic block, so `U+0301` COMBINING ACUTE ACCENT — the ordinary way to print Russian stress — is not in it, and the acute glyph was not even kept as a component. The Russian track therefore marks stress on the **romanization** (*chitát'*, *pishú*) and leaves the Cyrillic bare, which is what `russian/book/frontmatter.tex`'s preface already promised the reader. Do not reintroduce `U+0301` into Russian lesson text without re-subsetting this file first: XeLaTeX drops it silently apart from a `Missing character` line in the log
- `NotoSansHebrew-Static.ttf` — Hebrew (upstream static Regular)
- `NotoSansSC-Subset.ttf` — Chinese (Simplified); a **subset** of the ~17 MB NotoSansSC covering exactly the characters in `../data/scripts/chinese.json`. Regenerate with [`subset-cjk.sh`](./subset-cjk.sh) when Mandarin content adds characters.
- `NotoSansJP-Subset.ttf` — Japanese; a **subset** of the ~9.6 MB NotoSansJP covering **all** of U+3000–U+30FF (CJK punctuation, hiragana, katakana, the length bar, dakuten and handakuten) plus exactly the kanji the Japanese track uses. Regenerate with [`subset-jp.sh`](./subset-jp.sh) when the track adds kanji. One file covers all three of Japan's writing systems, because one Japanese sentence uses all three — the split is asymmetric on purpose: kana are a closed set worth taking whole, kanji are open-ended and only pulled in when written down.

- `LatinPrint-Subset.ttf` — **not a book font**: the outline the Latin-script
  stroke-order filmstrips (Spanish, French, German, Italian, Portuguese, Latin)
  are drawn on. No book preamble loads it; the books' body text stays in Latin
  Modern Roman. It exists because a filmstrip traces a school model's pen path
  over the printed letter, so the printed letter must have the model's shape:
  every cited source teaches the **one-storey a**, and Noto Sans (the Latin in
  the Devanagari and Cyrillic files above) prints a two-storey a. It is a
  renamed subset of **Andika 7.000** by SIL Global, a literacy typeface whose
  *default* a is the one-storey a (`a.SngStory`) and whose default g is
  single-storey (`g.SngBowl`), so no glyph is remapped.
  - Source: the release asset
    [`Andika-7.000.zip`](https://github.com/silnrsi/font-andika/releases/download/v7.000/Andika-7.000.zip)
    of [`silnrsi/font-andika`](https://github.com/silnrsi/font-andika) at tag
    `v7.000` (commit `6944336aca1d4763179c50e1dcb3028fe8ef60db`). SHA-256 of
    the zip: `88ba6ea41ef4a8e5214b090df8fa2983be1babe4843efaa99cdb6078b0e2c070`;
    of `Andika-Regular.ttf` inside it:
    `27484fdc98d0d63f90407f8266e28295f6fb16d2b13c5024df0214f17152919a`.
  - Regenerate with [`subset-latin.sh`](./subset-latin.sh) (pass a local
    `Andika-Regular.ttf` as `$1`, or let it fetch the zip; both digests are
    checked). It keeps printable Basic Latin, ß ñ Ñ ¿ ¡, the acute, diaeresis
    and tilde vowels, and every non-ASCII Latin character of every Latin-track
    headword and of `../data/scripts/latin.json` (141 characters today);
    flattens composite glyphs into plain contours; scales the em from 2048 to
    1000 units like every other file here; and pins `head.modified` with
    `SOURCE_DATE_EPOCH`, so the same inputs give the same bytes (26,292 bytes,
    SHA-256 `88437a253d204cedba17c586cd5cd6892bb538050efb3c9c0dd23afdb753cfab`).
  - **Renamed on purpose.** Andika's licence reserves the font names "Andika"
    and "SIL", and a subset is a Modified Version under the OFL, so this file
    is called "Latin Print Subset" (PostScript name `LatinPrintSubset-Regular`)
    and its description record says where it came from. The copyright and
    licence records are kept verbatim; Andika's own licence text is
    [`OFL-Andika.txt`](./OFL-Andika.txt).

(The Gurmukhi, Bengali, and Gujarati files are the upstream static `-Regular.ttf`
from the `notofonts.github.io` repo — already single-weight, so no instancing
needed. The others were flattened from variable fonts; see below.)

## Why static instances?

The upstream Noto files are **variable** fonts. XeLaTeX (which every book here
compiles with) cannot handle variable-font metrics — it fails with
`Transform components aren't all known`. These files were flattened to a
single Regular weight with `fonttools varLib.instancer <var>.ttf wght=400
[wdth=100]`, which XeLaTeX renders correctly.

## How the books use them

Each non-Latin book's `preamble.tex` loads the font by relative path, e.g.:

```latex
\newfontfamily\arabicfont[Path=../../_fonts/, Script=Arabic]{NotoNaskhArabic-Static.ttf}
```

(The Japanese book omits `Script=` — kana and kanji need no complex shaping, and
naming a script fontspec cannot resolve fails the build for no benefit.)

(`../../_fonts/` because books live at `<lang>/book/`.)

## License

Noto fonts are licensed under the SIL Open Font License 1.1 — see `OFL.txt`.
Redistribution (including vendoring here) is permitted under its terms. The
Nastaliq copyright notice comes from the
[font's source repository](https://github.com/notofonts/nastaliq/blob/08ae316851f3a841fb1e3d10e9f4012cff0cd981/OFL.txt),
and the official distribution's `fonts/` directory is likewise entirely
OFL-1.1.

`LatinPrint-Subset.ttf` is derived from Andika, © 2004-2025 SIL Global, also
under the SIL Open Font License 1.1 but with its own copyright notice and
Reserved Font Names ("Andika", "SIL"); its licence is in `OFL-Andika.txt`.
