# CV04a — Windows emitted-C consumer validation repair

Status: spec before changes; part of the shared-lexer source foundation validation,
not a new compiler transformation. Follow-up to CV04-exact-lexical-source-intervals.

The actual broad consumer run has six failures in `c-to-semantic-ir`: four unit
runtime roundtrips and two three-way C/Ruby conformance tests. Both emitted-C
helpers append `-lm` unconditionally. Their existing reference-C helper explicitly
omits the flag on Windows, where Clang translates it to `m.lib` and CRT supplies
math functions. A minimal native Clang program succeeds without the flag and,
with no other change, fails exit1181 with `cannot open input file m.lib` on stdout.
The installed linker/library environment is already operational.

Before implementation, retain those actual red logs/probe bytes and commit this
specification. Change only `src/lib.rs::tests::run_c` and
`tests/three_way_conformance.rs::run_emitted_c` to construct the same argv and
append `-lm` only when `!cfg!(windows)`, matching the reference leg. Keep C99,
source/output paths, discovered compiler, execution, normalization and every
existing runtime/reference equivalence assertion unchanged. No test skips,
compiler-selection fallback, output expectation changes, production AST/lowering
changes, crate dependencies or platform gate narrowing are permitted.

Run the original full package tests on Windows with actual Clang/Ruby, then strict
all-target lint. Broad consumer validation must report any remaining failure
honestly and be rerun on the final candidate. Linux/macOS retain their original
libm argv. Independent security review must cover the final changed head before
push. Actual native CI success on all three platforms remains required.
