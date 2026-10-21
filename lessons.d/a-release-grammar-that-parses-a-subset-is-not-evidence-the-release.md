---
category: Compiler / VM / language pipeline
---

# A release grammar that parses a subset is not evidence the release runs it; check the release's runtime sources before claiming an IIR profile

**What went wrong.** LANG84 R0 gave Ruby 0.49 and 0.50 grammar pairs that
parse the `rb-int-puts` subset (`puts(1 + 2)`, `puts(-7 / 2)`), at stage P.
The obvious next step in phase 5a was to compile the subset for them with the
3.3 semantics. Reading the tags' runtime sources showed both releases would
have been mislabelled:

- `io.c` at `v0_49` defines `puts` only as `IO#puts`
  (`rb_define_method(C_IO, "puts", ...)`); there is no Kernel `puts`, so a
  bare `puts(1)` at top level raises NameError.
- `numeric.c` at `v0_49` divides fixnums with C `/` and `%` on `int`, which
  truncate: `-7 / 2` is `-3` there, `-4` in 3.3.

The grammar only says the text is well formed. What a program *does* lives in
the interpreter's library and runtime, and those change across releases
independently of `parse.y`.

**The fix.** 0.49 and 0.50 get no `ReleaseProfile`; compiling for them is a
typed `ReleaseNotAvailable` ("no IIR profile"), and `releases.csv` keeps
their `iir_status` at `-`. The truncating division is recorded as
`IntDiv::Truncate` for the first early release that does get a profile.

**Next time.** Before a release row claims `iir_status`, cite the runtime
source for every semantic the subset depends on (here: the method table for
the callee, and the arithmetic helpers), not only the grammar. A pair at
stage P is a parse claim; I(S) is a meaning claim and needs its own evidence.
