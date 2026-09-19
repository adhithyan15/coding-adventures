---
category: CI & GitHub Actions
---

# `check-book-compile.sh --strict` says `ok` for a book whose LaTeX warnings will fail CI

Local verification of Malayalam chapter 107 ended with:

```
ok   malayalam
compiled 1, skipped 0, failed 0
STRICT: every selected track was compiled and verified.
```

CI then failed the *Build all human-language books* job:

```
::error::malayalam overfull rose to 1 against a baseline of 0
::error::malayalam font_substitution rose to 2 against a baseline of 0
```

**The compile script and the CI gate measure different things.**
`check-book-compile.sh` asks whether XeLaTeX produced a PDF. The workflow runs a
*second* step afterwards — `code/scripts/scan_latex_log_warnings.py` against
`core/latex-warning-baseline.json` — which fails a track whose `book.log`
warning counts **exceed** the recorded baseline. A book can compile perfectly and
still regress that baseline, and nothing local reports it.

**The two things that tripped it, both worth knowing on their own:**

- **Bold around inline code produces an undefined font shape.** Markdown
  `**… `CODE` …**` renders as `\textbf{… \texttt{CODE} …}`, and the book preamble
  has no bold monospace: *"Font shape `TU/LatinModernMono(0)/b/n' undefined"*.
  Keep the backticks outside the bold — rewrite the sentence so the citation sits
  in plain text.
- **A third table column of monospace identifiers overflows the text block.** A
  25-character `\texttt{}` in the last column of a three-column `tabularx` gave
  *"Overfull \hbox (37.94887pt too wide)"*. Lesson identifiers belong in the
  prose after a table, not in a column of it.

**Do this instead.** After `check-book-compile.sh`, run the scanner on the same
`book.log` before pushing:

```sh
bash code/scripts/check-book-compile.sh --strict <track>
python3 code/scripts/scan_latex_log_warnings.py \
  --book-root code/learning/human-languages \
  --baseline code/learning/human-languages/core/latex-warning-baseline.json
```

The scanner prints one line per track and marks the offender `[over baseline]`.
It takes about a second once the book is compiled, and it is the only local
signal that matches what CI will decide.
