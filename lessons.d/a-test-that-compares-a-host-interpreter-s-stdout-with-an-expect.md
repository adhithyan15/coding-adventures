---
category: Cross-platform & Windows BUILD_windows
---

# A test that compares a host interpreter's stdout with an .expect file must fold CRLF to LF, because Windows interpreters write text-mode stdout

**What went wrong.** In #17869 (Ruby 5a), `ruby-iir-compiler/tests/release_subsets.rs`
runs the host `ruby` on each corpus program and compares its stdout with the
`.expect` file. It passed on Linux and macOS but failed on `build (windows-2025)`:
`ruby 3.3.12 [x64-mingw-ucrt]` printed `"3\r\n5\r\n…"` where the
expectation is `"3\n5\n…"`. Ruby on Windows opens stdout in text mode,
so every `puts` ends in CRLF. Perl (`:crlf` layer) and Python (text-mode
`print`) do the same on Windows.

**The fix.** Fold `"\r\n"` to `"\n"` in the host's stdout before comparing.
The `.expect` files record the language's output, not the platform's line
ending. Our compiled programs write `putchar 10` and are unaffected.

**Next time.** Any oracle test that shells out to a host interpreter
(ruby, perl, python3, node) and compares its output byte for byte should
normalize line endings from the first commit. CI runs these tests on
Windows whenever the interpreter is on PATH there.
