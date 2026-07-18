# Vendored fonts (non-Latin scripts)

These are **static Regular** files of Google's [Noto](https://fonts.google.com/noto)
fonts, vendored so the non-Latin-script books (Arabic, Hindi, Marathi, Tamil,
Kannada, Telugu, Malayalam, Punjabi, Bengali) compile **identically** on any
machine and in CI — with no dependency on whatever font packages happen to be
installed.

- `NotoNaskhArabic-Static.ttf` — Arabic
- `NotoSansDevanagari-Static.ttf` — Hindi **and Marathi** (both Devanagari)
- `NotoSansTamil-Static.ttf` — Tamil
- `NotoSansKannada-Static.ttf` — Kannada
- `NotoSansTelugu-Static.ttf` — Telugu
- `NotoSansMalayalam-Static.ttf` — Malayalam
- `NotoSansGurmukhi-Static.ttf` — Punjabi (Gurmukhi)
- `NotoSansBengali-Static.ttf` — Bengali
- `NotoSansCyrillic-Static.ttf` — Russian (Cyrillic); a `fontTools.subset` of NotoSans (latin-greek-cyrillic) to Basic Latin + Cyrillic
- `NotoSansHebrew-Static.ttf` — Hebrew (upstream static Regular)
- `NotoSansSC-Subset.ttf` — Chinese (Simplified); a **subset** of the ~17 MB NotoSansSC covering exactly the characters in `../data/scripts/chinese.json`. Regenerate with [`subset-cjk.sh`](./subset-cjk.sh) when Mandarin content adds characters.

(The Gurmukhi and Bengali files are the upstream static `-Regular.ttf` from the
`notofonts.github.io` repo — already single-weight, so no instancing needed.
The others were flattened from variable fonts; see below.)

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

(`../../_fonts/` because books live at `<lang>/book/`.)

## License

Noto fonts are licensed under the SIL Open Font License 1.1 — see `OFL.txt`.
Redistribution (including vendoring here) is permitted under its terms.
