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


