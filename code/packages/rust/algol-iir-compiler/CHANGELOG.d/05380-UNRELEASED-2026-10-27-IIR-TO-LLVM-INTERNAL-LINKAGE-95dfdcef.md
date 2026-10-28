## Unreleased — iir-to-llvm exports only the entry point (#18416)

- `tests/backend_compat.rs`: the proper-procedure LLVM check expects `define internal void @bump`. `iir-to-llvm` now gives every function except the module's entry point `internal` linkage, so a program cannot replace a libc function the linked runtime calls.
