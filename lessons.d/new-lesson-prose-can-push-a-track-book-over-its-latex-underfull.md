---
category: Repo policy / workflow reminders
---

# New lesson prose can push a track book over its LaTeX underfull baseline, and only a local compile shows where

The Malayalam chapters 115-141 PR passed every `check:*` script and the
full vitest suite, then failed the **Build all human-language books** job:

    malayalam underfull rose to 1 against a baseline of 0; first log line:
    Underfull \hbox (badness 5231) in paragraph at lines 142--143

`scan_latex_log_warnings.py` compares each book's warning counts with
`core/latex-warning-baseline.json`. The log line names source lines but not
the file, and nothing in the TypeScript checks compiles LaTeX, so the only
way to find the paragraph was to build the book.

The culprit was a justified line ending in a long, unbreakable Malayalam
word (`... in the plural like അഭിനന്ദനങ്ങൾ.`). TeX pushed the word to the next
line and stretched the line above it past tolerance. The fix was a
rewording that moves the long word away from the line end.

Do differently: after adding chapters to a track, compile that book locally
before pushing. It takes about a minute:

    apt-get install -y --no-install-recommends fontconfig latexmk lmodern \
      texlive-fonts-recommended librsvg2-bin texlive-lang-arabic texlive-xetex
    bash code/scripts/check-book-compile.sh --strict <track>
    grep -n "Underfull\|Overfull" code/learning/human-languages/<track>/book/book.log

(`texlive-lang-arabic` is needed for every book, not only Arabic ones: it
supplies `bidi.sty`.) The book log then shows the offending text inline.
