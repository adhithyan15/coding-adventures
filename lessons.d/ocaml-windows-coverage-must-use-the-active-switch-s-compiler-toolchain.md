---
category: Cross-platform & Windows BUILD_windows
---

# OCaml Windows coverage must use the active switch's compiler toolchain

Running Dune from a Windows shell with the inherited PATH selected Strawberry
or WinLibs MinGW tools while the active opam switch's OCaml runtime and C
archives were built for a different toolchain. Compilation reached the native
clock stub and failed at link time on CRT symbols such as `swprintf`; this was
not an OCaml source or coverage-instrumentation failure. A separate Windows
2025 failure occurred earlier: the Cygwin-mode setup omitted the configured
`x86_64-w64-mingw32-gcc` and `ar` commands while building OCaml development
dependencies.

Use the setup action's provisioned environment for the active switch
(Cygwin for the dedicated Windows 2022 OCAML03/OCAML07 jobs; MSYS2 for generic
Windows 2025 CI), the switch `bin`, Git tools required by opam, and Windows
system directories. Do not append an unrelated machine-wide MinGW path or
substitute another C runtime. When the switch's compiler emits a prefixed
MinGW command, verify that the exact command and matching archiver resolve
inside `opam exec` before dependency installation, then compile and archive a
small C probe with them. The OCaml coverage summary instruments the OCaml
wrapper but does not measure the C clock shim; report that native boundary
separately.
