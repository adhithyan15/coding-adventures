---
category: Compiler / VM / language pipeline
---

# Moving work from a builtin into compiler-emitted IIR multiplies every VM step budget that counted the builtin as one step

LANG83 PR7c-js replaced the `write_f64_js` builtin with a module-local
`__js_write_number` helper. A print went from one VM step (`call_builtin`)
to about 110–165 steps, but the JavaScript runner still had a flat
`max_instructions = 100_000`, sized when every print was one step. Valid
programs printing a few hundred numbers (`console.log(1e20, 1e20, 1e20)`
×200, 5.8 KB) were refused with "possible infinite loop". All 551
`lang_matrix` rows passed; the security review found it with a probe.

Fix: derive the budget from the limits that already bound the program
(`MAX_AST_NODES × 256`, the worst per-node cost) and test the largest
program those limits accept, not a typical one.

Whenever a PR moves a builtin's work into emitted IIR (PR7c-py, PR7e, PR7f
do the same for Python floats and Lisp `dyn_*` semantics), grep every
`max_instructions` / step / fuel budget on the runners that execute that
IIR and re-derive it from the input limits.

PR7c-py's first fix overshot the other way: a flat ceiling sized for the
largest *possible* module (257M steps) let a hand-built `label L; jmp L`
handed to the public runner spin for ~26 s. Where the runner can see the
module, derive the budget from that module (its straight-line length plus
the per-helper-call bound per call site) and keep the global figure only
as a cap.
