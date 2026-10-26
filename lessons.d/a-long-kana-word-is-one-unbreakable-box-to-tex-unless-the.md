---
category: Testing & coverage
---

# A long kana word is one unbreakable box to TeX unless the Japanese snippet turns on ICU line breaking

The Japanese B1 vocabulary builder wrote each verb's past as its own note,
`Past (polite): **せんせいが うながしました。**, *sensei ga unagashimashita*.`,
which becomes its own two-line paragraph in the book. Eight of those 442
paragraphs came out underfull (badness 2269 to 10000), and
`scan_latex_log_warnings.py` put Japanese over the warning baseline. Every
package check, the full vitest and the language-ladder suite passed; only the
strict compile plus the scan saw it.

The first fix was wrong. Merging the past into the paragraph that gives the
polite form (the way the core chapters set it) made it worse: 19 underfull and 2
overfull. In every case a justified line ended just before a long kana word
(きょうりょくしました), which TeX treats as one box because Japanese is broken only
at the spaces the lessons put between words, so the line had to stretch its few
spaces.

The real fix was in the book: `\japanesefont` now also sets
`\XeTeXlinebreaklocale "ja"` and a small `\XeTeXlinebreakskip`, local to each
snippet's group, so a line may break between kana and kanji under the Japanese
kinsoku rules. The setting rides on the font command, not on `\ja`, because the
glyph-coverage gate reads `\ja`'s one-line `{{\japanesefont #1}}` shape to learn
which font renders a snippet; a two-line `\ja` hid the font from it. The scan went to 0 in every
class. `\hbadness`, `\hfuzz`, `\tolerance` and the baseline were not touched.

Do differently: run `check-book-compile.sh --strict <track>` and the warning scan
before calling generated output ready, and when a warning repeats with one
shape, find what TeX cannot break before changing the content. Any track whose
script has no spaces inside words (Chinese, Thai) needs its snippet macro to set
the line-break locale too.
