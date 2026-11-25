---
category: Cross-platform & Windows BUILD_windows
---

# OCaml Windows coverage must use the switch's Cygwin compiler toolchain

Running Dune from a Windows shell with the inherited PATH selected Strawberry
or WinLibs MinGW tools while the active opam switch's OCaml runtime and C
archives were built for its Cygwin toolchain. Compilation reached the native
clock stub and failed at link time on CRT symbols such as `swprintf`; this was
not an OCaml source or coverage-instrumentation failure.

Use a task-local short PATH containing the active switch's `.cygwin/root/bin`,
the switch `bin`, Git tools required by opam, and Windows system directories.
Do not append the machine-wide PATH or substitute a different MinGW runtime.
The OCaml coverage summary instruments the OCaml wrapper but does not measure
the C clock shim; report that native boundary separately.
