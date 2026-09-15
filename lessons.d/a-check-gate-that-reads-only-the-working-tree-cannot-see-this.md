# A `--check` gate that reads only the working tree cannot see this class at all

`book-cli --check` regenerates from the working tree and compares to the working tree. When both
sides carry the same CRLF, it agrees with itself and exits 0 — **the disagreement it needs to
find is between the working tree and the blob, and it never looks at the blob.** So the gate is
green locally and red in CI, by construction, for every line-ending or filter-normalization skew.

This is a real blind spot, not merely an operator error: a check that validates one copy against
itself is vacuous with respect to what will actually be committed. The fix worth considering is
for `--check` to compare against `git show :path` (the staged blob) rather than the file on disk.
