# Vendored fonts (non-Latin scripts)

These are **static Regular instances** of Google's [Noto](https://fonts.google.com/noto)
fonts, vendored so the non-Latin-script books (Arabic, Hindi, Tamil, Kannada,
Telugu, Malayalam) compile **identically** on any machine and in CI — with no
dependency on whatever font packages happen to be installed.

- `NotoNaskhArabic-Static.ttf` — Arabic
- `NotoSansDevanagari-Static.ttf` — Hindi (Devanagari)
- `NotoSansTamil-Static.ttf` — Tamil
- `NotoSansKannada-Static.ttf` — Kannada
- `NotoSansTelugu-Static.ttf` — Telugu
- `NotoSansMalayalam-Static.ttf` — Malayalam

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
