# A hash mismatch with no visible content difference: compare byte counts to line counts

`check:books` failed in CI with `ch394-...tex: generated output is missing or stale`, while the
identical check passed locally with exit 0. That phrasing reads like a generator bug, and the
first instinct is to suspect the generator or the environment. It was neither.

**The diagnostic that settled it in one step:** compare each source file's working-tree size to
its blob size, and compare the difference to the file's line count.

| lesson | worktree | blob | delta | lines |
|---|---|---|---|---|
| `ES-C394-guitarra` | 5652 | 5546 | **106** | 106 |
| `ES-C394-medico` | 5469 | 5354 | **115** | 115 |
| `ES-C394-universidad` | 5309 | 5309 | 0 | 103 |

**Delta exactly equal to the line count means exactly one byte per line was dropped**, which is
CRLF in the working tree against LF in the blob. Nothing else produces that signature. The
generator hashes lesson *sources*, so the hash computed on Windows against CRLF could never match
the hash CI computes against the LF blobs — and only the chapter containing those files went
stale, which is why the failure looked oddly narrow.

`.gitattributes` had `text=auto, eol=lf` all along, and `git add` printed
`CRLF will be replaced by LF the next time Git touches it` for exactly those files. **That warning
is the whole diagnosis, printed in advance and scrolled past.** Read the add warnings.
