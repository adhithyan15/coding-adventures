---
category: Testing & coverage
---

# check-book-compile --strict does not run the LaTeX warning scan; run scan_latex_log_warnings.py before pushing book content

**What went wrong.** Malayalam B1 (#17984) failed CI's "Build all human-language
books": `scan_latex_log_warnings.py` found 8 overfull and 127 underfull hboxes
against a baseline of 0. Locally the book had passed `check-book-compile.sh
--strict`, which compiles and verifies the PDF but does not read book.log for
typesetting warnings. The Telugu, Kannada and Tamil B1 branches, built the same
way, carried the same defect (Tamil: 69 overfull, 303 underfull).

**Cause.** The B1 vocabulary chapters set each example sentence, its
romanization and its translation in a quote block, a narrower measure than the
body, and an Indic sentence is a few long agglutinated words with two or three
spaces to stretch. CI has no hyphenation patterns for these scripts, so a long
word cannot break either.

**Fix.** Quote blocks set ragged right in each preamble (the HL-C88 remedy the
callouts already use; \hbadness, \hfuzz and the baseline untouched), then the
few body notes still forcing a loose line reworded (HL-C109c: fix the text).

**Do differently.** After compiling any book whose content changed, run from the
repo root
`python3 code/scripts/scan_latex_log_warnings.py --book-root code/learning/human-languages --baseline code/learning/human-languages/core/latex-warning-baseline.json`
and require every track to report `[ok]` or `[under baseline]`. Attribute a box
to its chapter by tracking the last `(./chapters/...tex` opened before it in
book.log.
