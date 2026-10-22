---
category: Cross-platform & Windows BUILD_windows
---

# A whitespace-only source passed as a Windows argv element is dropped, so feed oracle sources on stdin

LANG84 4c added a Ruby `empty_program` row whose source is `"\n"`. The
host-Ruby oracle in `lang_matrix.rs` ran each row as
`Command::new("ruby").args(["-e", src])`. It passed on Linux, then failed on
`build (windows-2025)` with `ruby 3.3.12 ... [x64-mingw-ucrt] failed "\n"`.

Windows has no argv, only one command-line string. Rust's `Command` quotes
an argument only when it is empty or holds a space or tab, so `"\n"` went
out bare. Ruby's Windows command-line splitter treats the newline as
whitespace and drops it, so Ruby saw `-e` with no code and exited non-zero.
Every other row held a space, was quoted, and survived.

**Fix:** give the program to the interpreter on stdin (`ruby` with no
script argument reads stdin) rather than as an argument. Nothing then
passes through command-line quoting.

**Do differently:** an oracle that hands program text to a host
interpreter should never put the text in argv: use stdin or a file. If
argv is unavoidable, add a row whose source is only whitespace and
confirm it on a Windows runner.
