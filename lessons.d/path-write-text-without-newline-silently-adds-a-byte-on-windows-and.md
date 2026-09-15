# `Path.write_text(...)` without `newline=""` silently adds a byte on Windows and breaks exact-size assertions

`logic-builtins`' `test_file_path_metadata_facade` asserted a Prolog `size_fileo/2`
builtin returned `len("fact(a).\n")` (9) for a file it had just written with
`source_path.write_text("fact(a).\n", encoding="utf-8")`. It got 10 on Windows.

**Cause:** `Path.write_text` defaults to platform newline translation unless
`newline=""` is passed (the exact mechanism documented at the CRLF lesson,
line ~3121, but there for a content-generation script — here it silently
corrupted a test's own fixture size instead of a hash). `\n` became `\r\n` on
write, so the file the test created was one byte longer than the string literal's
`len()`, and the byte-count assertion — computed from the string, not the file —
went stale relative to what was actually on disk.

**Fix:** `source_path.write_text("fact(a).\n", encoding="utf-8", newline="")`
whenever a test both writes a fixture file with embedded `\n` AND later asserts
something about that file's exact byte size (not just its content). Content-only
assertions (`read_text()` back and compare) are unaffected because Python
normalizes `\r\n` back to `\n` on text-mode read — only a *size*/byte-count
assertion computed from the pre-write string is at risk. The general write-side
guidance from line 3145 (open in binary, or pass `newline=''`) applies to test
fixtures exactly as much as to generated production files.
