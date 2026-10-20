## 0.581.0 (UNRELEASED)

- `BUILD` now runs the full-corpus `matrix_every_proven_cell_agrees` aggregate, including every ALGOL row, and `proven_columns_do_not_silently_skip` in CI. It previously ran only the non-ALGOL aggregate. The two reds that kept the ALGOL rows out are fixed in the backends: `x86_64-backend` 0.40.0 passes call arguments beyond the ABI's registers on the stack (VM-D005, captured 4-D array formals), and `iir-to-jvm-class-file` 0.36.0 measures `max_stack` from the emitted bytecode (#12032, `VerifyError: Stack size too large`).
