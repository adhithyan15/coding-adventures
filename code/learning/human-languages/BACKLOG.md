# Human Languages — backlog

Note on provenance: this file was EMPTY in git for its whole history until now.
Findings from the pre-A1 tranche work were recorded in commit messages and PR
bodies, which are durable and searchable, but a reader opening this file found
nothing. The entries below are the ones that change how the work is done.

## HL-C213 — build all 22 books LOCALLY before pushing; it takes ~100 seconds

Measured on a 14-core laptop, clean rebuild, 8-way parallel:

```
all 22 books        ~100s wall clock
sequential sum      ~367s
spanish alone         74s  (1337pp — sets the parallel floor)
```

The same work in CI takes **5 to 58 minutes**, and once hung **6 hours** in `apt`
fetching the TeX toolchain, having compiled nothing. So the LaTeX work is not the
cost — provisioning and queueing are.

`data/scripts/build_all_books.sh` runs the lot and checks the four things that
matter. **The exit code alone is not one of them:**

| check | why the exit code misses it |
|---|---|
| missing characters | a font gap prints NOTHING and still exits 0 — telugu once shipped 89 |
| overfull boxes | spanish crossed 1000pp, contents numbers gained a digit, 14 lines overflowed, exit 0 |
| underfull boxes | the fix for overfull can trade one for the other |
| exit code | catches only a hard LaTeX error |

`sh build_all_books.sh --self-test` proves each detector fires on a known-dirty log
and stays silent on a clean one. It is not decoration: **it caught two real bugs in
this script before either ever ran on the corpus** —

1. `grep -c` EXITS 1 when the count is zero, so `|| echo 0` appended a second zero
   and the integer test errored instead of comparing. That silently disabled the
   missing-character check — the very thing the script exists for.
2. The overfull/underfull probes never fired, because `printf '%s'` does not
   interpret backslashes in its ARGUMENT, so `\\hbox` stayed two backslashes.

A rebuild overwrites `book.log`, so a defect planted in a real log cannot survive
long enough to test the detector. That is why classification is factored into
`classify_log` and tested against synthetic logs instead.

## Three rules this work keeps re-deriving

**A measurement that did not happen looks exactly like one that passed.** Check the
magnitude, not just the verdict: a vitest run with a bad `--reporter` flag failed at
startup and EXITED 0; a scan with a wrong cwd read zero files and reported clean;
`grep $'\x00'` is an empty pattern that matches every file. Read the exit code, the
`Test Files N passed (N)` line, AND the count against a known baseline.

**Verify a checker against a known-dirty case before believing a clean result.**
Eleven detectors in this work reported clean while blind — from `\w` excluding the
combining marks under inspection, to an unassigned codepoint splitting a
mixed-script word into two clean halves, to a heredoc mangling a detector's own
fixtures.

**Fix the thing measured, never the threshold.** A gate on the LARGEST eager chunk
was once satisfied by splitting one 502 kB chunk into four — the page still
downloaded the same bytes. The real fix made the data lazy: 502 kB → 287 kB, and no
corpus growth reaches the eager graph at all.
