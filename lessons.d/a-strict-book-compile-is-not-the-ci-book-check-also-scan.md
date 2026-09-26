---
category: Repo policy / workflow reminders
---

# A strict book compile is not the CI book check: also scan the log for typesetting warnings

**What went wrong.** The Hindi A1 tranche passed `check-book-compile.sh --strict
hindi` locally. It then failed CI's "Build all human-language books" job. After
compiling every book, that job runs `code/scripts/scan_latex_log_warnings.py`
against `core/latex-warning-baseline.json`. Hindi's overfull count went from 0
to 78.

The tranche was the first to push a book's *lesson* pages past 999, at chapter
151. `book.cls` sets contents page numbers in a 1.55em box, which holds three
digits. Every contents line with a four-digit page ran 4.93pt wide. German and
Tamil had already passed 1000 pages, but only in their back matter, which the
contents does not list, so no earlier track tripped on it.

**The fix.** Hindi's preamble sets `\@pnumwidth` to 2.4em and `\@tocrmarg` to
3.4em. With those, the contents holds four bold digits.

**Do differently.**
1. After any local book compile that a change could affect, run:

       python3 code/scripts/scan_latex_log_warnings.py \
         --book-root code/learning/human-languages \
         --baseline code/learning/human-languages/core/latex-warning-baseline.json

   Treat any `[over baseline]` line as a CI failure.
2. When a tranche adds many chapters, check whether the book's contents will
   list pages above 999. If they will, give that track's preamble the same
   `\@pnumwidth`/`\@tocrmarg` widening.
