# I built the exact vacuous check I had spent the day criticising

Publishing the SwiftUI app needed proof that the engine was linked into the
binary, since SwiftUI is the one backend that links `engram-capi` statically
instead of loading it at runtime. I deliberately avoided `nm` — a toolchain
check is one that can be absent, and this repo has already shipped a platform
unverified exactly that way. So I scanned the binary's bytes for `eg_*` names,
calibrated it against controls (9 in the real app, 0 in `/bin/ls`), and wrote
in the release notes that "the release lane asserts those symbols are in the
packaged binary."

It does not. Mach-O stores **defined and undefined symbols identically** in the
string table, so a byte scan gives the same answer for a binary that contains
the engine and one that merely expects it from a library that will not be
there. Two `clang` fixtures, same five symbols:

```
dynlinked  (engine NOT in binary): defined 0, undefined 5  -> my scan PASSED
statlinked (engine IS in binary):  defined 5, undefined 0  -> my scan PASSED
```

The undefined case is precisely the broken build the check existed to catch.
In the same commit, I had *tightened* the shell script's `nm` check because it
accepted undefined symbols — and then reproduced that exact bug in Python, one
function away, while believing the new check was the stronger one.

What made it invisible: **my controls only tested the axis I was already
thinking about.** `/bin/ls` has no `eg_` strings at all, so it confirmed
"absent means absent" and told me nothing about "present but undefined." A
control that cannot fail the way the real defect fails is not a control.

The fix is to parse the symbol table — `LC_SYMTAB`, `nlist_64`, and the
`N_TYPE` field that says `N_SECT` (defined here) or `N_UNDF` (expected from
elsewhere). About 60 lines of `struct`, no toolchain, works wherever Python
does. Test fixtures are synthesised Mach-O rather than compiled ones, so they
run on the Linux runner too, and were cross-checked against `clang` output to
confirm the synthetic layout matches reality.

Three things generalise:

- **A string is not a symbol.** Any check that greps a binary is answering
  "does this text appear", which is almost never the question.
- **Pick controls that fail the way the real bug fails.** Absent-vs-present was
  the easy axis; defined-vs-undefined was the one that mattered.
- **Criticising a failure mode does not immunise you against it.** I had
  written the words "a verification that silently skips is not a verification"
  into this file hours earlier. Knowing the pattern is not the same as checking
  whether your own code has it — so check.
