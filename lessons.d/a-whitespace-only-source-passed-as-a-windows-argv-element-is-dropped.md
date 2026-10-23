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

**It happened twice.** The first fix moved only the Ruby oracle to stdin.
The next Windows run then failed the host-Perl oracle the same way. Perl's
`crlf_layout` row, `"print(1);\r\nprint(2);\r\n"`, has no space, so it
went out unquoted, and Strawberry Perl's splitter cut it at the line break.
Perl printed `1` where the row expected `12`. The same is true of any
source with a line break but no space, not just whitespace-only ones.

**Do differently:** an oracle that hands program text to a host
interpreter should never put the text in argv: use stdin or a file.
`lang_matrix.rs` now has one helper, `run_host_oracle`, that every oracle
calls. When a bug like this is found in one call site, grep for the
pattern (`-e`, `-c` with a variable argument) and fix every site in the
same push. A Windows CI round trip costs hours.
