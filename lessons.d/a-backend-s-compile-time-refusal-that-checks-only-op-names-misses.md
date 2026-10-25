---
category: Compiler / VM / language pipeline
---

# A backend's compile-time refusal that checks only op names misses values a literal can carry

**What went wrong.** LANG84 6e made literal-only JavaScript modules run on
BEAM by skipping the `-0` test for literals, so `console.log(42)` carries no
`f64_*` op that BEAM refuses. The matrix rule was "BEAM refuses `f64_*` at
compile time", and the tests pinned exactly which rows BEAM refuses. But a
literal can be non-finite: `1e999` decodes to `Infinity`, it is still a
literal, and the module carried only `const`, `write_f64_js` and `putchar`.
The BEAM validator checked op names and the float constant's type hint, so it
accepted the module. Lowering then put an infinity into the literal table,
where an Erlang loader cannot read it. The 6e security review caught this
with a probe build, not a test.

**Fix.** `validate_for_beam` now rejects a non-finite `f64` constant with an
`UnsupportedType` error. The check sits in the backend, so it covers every
frontend, not only JavaScript. The JavaScript `decode_number` also refuses
spellings that no edition can lex (`-0`, `inf`, `NaN`).

**Next time.** When a backend's refusal depends on the absence of an op,
list the values that can reach it *without* that op: literals, folded
constants, and parameters. Then check that the validator rejects each value
the target runtime cannot represent. A refusal is a property of the values
as well as of the opcodes.

**And the follow-up CI failure.** Tightening the validator broke
`lang-aot/tests/op_conformance.rs`. Its PR6a float-table cases declared BEAM
"Lowered" because every op they use is lowered there, but the table holds
`inf`/`nan` constants. Before the push only the iir-to-beam tests and the
JavaScript/BEAM `lang_matrix` filters ran, so the conformance binary first
failed in CI. The test's `lowered_on` now also asks whether BEAM can hold the
case's constants. When you change what a backend accepts, run every lang-aot
test binary that compiles to that backend (`op_conformance`, `lang_matrix`),
not only the backend crate's own tests.
