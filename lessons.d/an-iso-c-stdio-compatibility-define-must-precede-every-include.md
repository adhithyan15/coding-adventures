---
category: Testing & coverage
---

# An ISO C stdio compatibility define must precede every include

**What went wrong.** A strict Windows-targeting Clang run treated ISO `fopen`
as deprecated. The test defined `_CRT_SECURE_NO_WARNINGS` after including a
shared test header, which had already included the CRT declarations.

**Fix.** Put the compatibility definition before every header in the test.
This keeps ISO stdio while avoiding the MSVC-only `fopen_s` replacement.

**Next time.** When a C file needs a CRT compatibility definition, put it at
the top of the translation unit and verify it with a compiler that targets the
Windows CRT.
