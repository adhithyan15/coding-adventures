## The book catches up -- Chapters 3-16 typeset

The lessons had run ahead of the published artifact: 61 authored lessons through
Chapter 16, but the LaTeX book still stopped at Chapter 2 ("Introducing
Yourself"). Because the CI book build only compiles what is wired into
`book.tex`, the missing chapters were invisible to CI and the gap drifted
silently. This closes it -- **fourteen new book chapters**, written from the
existing `GE-C03`-`GE-C16` lessons and wired into `book.tex`:

- **Ch3** How Are You (danke, bitte, gehen, wie geht es, es geht)
- **Ch4** Farewells (auf Wiedersehen, tschuess, bis bald, bis morgen)
- **Ch5** The First Verbs (wohnen, machen, lernen, ich lerne Deutsch)
- **Ch6** Numbers One to Ten * **Ch7** The Days of the Week (and Mittwoch)
- **Ch8** Telling the Time * **Ch9** Months and Seasons (Herbst/harvest)
- **Ch10** Family * **Ch11** Bread, Water, Wine
- **Ch12** Numbers Eleven to Twenty (elf/zwoelf, the "-lif = left over" story)
- **Ch13** Colours * **Ch14** To Have, and How Old You Are (the habere false
  cognate)
- **Ch15** The Two Past Tenses (Perfekt, Praeteritum)
- **Ch16** To Be, and the Past That Takes It (sein -- three ancient verbs in one
  paradigm -- and the Perfekt built on it)

Each chapter follows the established book conventions: one `\section` per lesson
with a slug `\label`, the `cousinweb` / `culture` / `grammarlens` / `sounds` /
`etymology` / `morphologybox` boxes, `booktabs` conjugation tables, and every
atom traced to its root -- the German/English cognate webs are the spine.
Content is faithful to the lessons -- no new etymologies introduced.
Practice-section labels are chapter-qualified (`lesson:chN-practice`).

The book grows to **84 pages**; compiles clean with XeLaTeX (0 errors, 0 missing
characters, 0 undefined references, 0 duplicate labels) and was rasterized and
visually QA'd -- the umlauts, the eszett, `fui` with macron, and the PIE
superscripts all render correctly.

