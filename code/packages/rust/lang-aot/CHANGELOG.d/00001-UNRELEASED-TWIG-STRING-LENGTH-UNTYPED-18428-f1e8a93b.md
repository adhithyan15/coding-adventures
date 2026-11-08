## Unreleased — Twig string-length on untyped operands (#18428)

Dynamic `string-length` on untyped operands is now supported on the VM and JIT matrix columns.
`iir-builtin-lowering::vm_lisp` rewrites `call_builtin "string-length"` to `str_len`, and `vm-core::builtins` registers `"string-length"` on `BuiltinRegistry`.
Non-string arguments raise a clean `TypeError` (trapping as expected).
Added 2 matrix rows (79 rows, 511 cells declared for Twig).
