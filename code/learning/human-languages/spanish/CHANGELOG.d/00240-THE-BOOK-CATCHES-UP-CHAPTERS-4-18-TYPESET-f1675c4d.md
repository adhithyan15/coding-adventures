## The book catches up — Chapters 4–18 typeset

The lessons had run ahead of the published artifact: 90 authored lessons through
Chapter 18, but the LaTeX book still stopped at Chapter 3 ("Introducing
Yourself"). This closes that gap — **fifteen new book chapters**, written from
the existing `ES-C04`–`ES-C18` lessons and wired into `book.tex`:

- **Ch4** How Are You · **Ch5** Farewells · **Ch6** The First Verbs ·
  **Ch7** The *-er* and *-ir* Verbs · **Ch8** Numbers and Age
- **Ch9** *Ser* and *Estar* · **Ch10** Going, and the Near Future ·
  **Ch11** Stem-Changing Verbs · **Ch12** The *-go* Verb Club ·
  **Ch13** Completing the *-go* Club
- **Ch14** The Preterite · **Ch15** Completing the Preterite ·
  **Ch16** The Imperfect · **Ch17** The Future and the Conditional ·
  **Ch18** The Subjunctive

Each chapter follows the established book conventions: one `\section` per
lesson with a slug `\label`, the `cousinweb` / `grammarlens` / `culture` /
`sounds` boxes, `booktabs` conjugation tables, and every atom traced to its
root. Content is faithful to the lessons — no new etymologies introduced.

The book grows from ~20 pages to **114 pages**; compiles clean with XeLaTeX
(0 errors, 0 missing characters, 0 undefined references, 0 duplicate labels)
and was rasterized and visually QA'd. Practice-section labels are
chapter-qualified (`lesson:chN-practice`) to keep them unique.

Also fixed: a corrupted cognate line in `ES-C04-gracias.md` ("French
**grâce/merci-less** *grâces*") now reads as the intended claim — the Romance
family kept *grātia* even where everyday thanks went elsewhere.

