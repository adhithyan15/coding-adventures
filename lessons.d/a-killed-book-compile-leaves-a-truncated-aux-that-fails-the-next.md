---
category: Testing & coverage
---

# A killed book compile leaves a truncated .aux that fails the next run

`check-book-compile.sh hindi` reported

```
FAIL hindi
! File ended while scanning use of \@writefile.
compiled 0, skipped 0, failed 1
```

on a tree whose `.tex` was valid and whose `check:books` hash gate was green.
Nothing was wrong with the LaTeX. An earlier run of the same script had been
stopped part-way through, and XeLaTeX had left `<track>/book/book.aux` cut off
mid-`\@writefile`. The next run read that half-written aux on its first pass
and died on it.

The error names `\@writefile`, which is aux-file machinery, not anything an
author writes — that is the tell. A genuine LaTeX bug in generated content
names a character, an environment or a command that appears in the `.tex`.

Fix: delete the build artifacts and rerun. They are all gitignored and all
regenerable, so nothing is at risk:

```
rm -f <track>/book/book.{aux,log,out,toc,xdv,fls,fdb_latexmk,pdf}
bash code/scripts/check-book-compile.sh <track>
```

The general rule: **a book compile is not safe to interrupt.** If one has to be
stopped, clear that track's build directory before believing the next result,
in either direction — a stale aux can also carry forward a reference that makes
a broken tree look fine.
