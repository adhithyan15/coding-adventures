---
category: Cross-platform & Windows BUILD_windows
---

# Case-variant pin fixtures must replace the original filename before writes

CI run 38190146667 built 533 Windows packages but failed script-ductus's
uppercase ownership-pin rejection assertion. The fixture's map held both
`kannada.json` and `Kannada.json`. On case-insensitive NTFS, the second write
replaced the contents of the first file while its directory entry retained the
lowercase spelling. The loader correctly accepted the actual lowercase pin.

Remove the lowercase map entry before adding its uppercase replacement, then
write the complete fixture into a new directory. Assert the directory's exact
filename inventory before checking the loader's result. That makes setup prove
the uppercase entry exists on Windows, macOS and Linux, rather than relying on
two names that the filesystem may collapse. Keep content and symlink rejection
checks intact; a file-symlink privilege failure is a separate local limitation.

Validation on the local NTFS host reproduced the original fixture's preserved
lowercase entry and loader acceptance. The repaired fixture has exactly the
requested filenames and the loader rejects `Kannada.json` as unsafe. The existing
ownership suite passes six cases; the malformed-pin case passes every filename
and content rejection before reaching its unchanged file-symlink fixture, which
fails locally with EPERM. TypeScript compilation passes. CI's Windows runner can
create file symlinks and must run the whole ownership suite before merge.
