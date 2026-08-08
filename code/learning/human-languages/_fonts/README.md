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
- `NotoSansCyrillic-Static.ttf` — Russian (Cyrillic); a `fontTools.subset` of NotoSans (latin-greek-cyrillic) to Basic Latin + Cyrillic. **It carries no combining diacritics**: the subset stops at the Cyrillic block, so `U+0301` COMBINING ACUTE ACCENT — the ordinary way to print Russian stress — is not in it, and the acute glyph was not even kept as a component. The Russian track therefore marks stress on the **romanization** (*chitát'*, *pishú*) and leaves the Cyrillic bare, which is what `russian/book/book.tex`'s preface already promised the reader. Do not reintroduce `U+0301` into Russian lesson text without re-subsetting this file first: XeLaTeX drops it silently apart from a `Missing character` line in the log
- `NotoSansHebrew-Static.ttf` — Hebrew (upstream static Regular)
- `NotoSansSC-Subset.ttf` — Chinese (Simplified); a **subset** of the ~17 MB NotoSansSC covering exactly the characters in `../data/scripts/chinese.json`. Regenerate with [`subset-cjk.sh`](./subset-cjk.sh) when Mandarin content adds characters.
- `NotoSansJP-Subset.ttf` — Japanese; a **subset** of the ~9.6 MB NotoSansJP covering **all** of U+3000–U+30FF (CJK punctuation, hiragana, katakana, the length bar, dakuten and handakuten) plus exactly the kanji the Japanese track uses. Regenerate with [`subset-jp.sh`](./subset-jp.sh) when the track adds kanji. One file covers all three of Japan's writing systems, because one Japanese sentence uses all three — the split is asymmetric on purpose: kana are a closed set worth taking whole, kanji are open-ended and only pulled in when written down.

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
