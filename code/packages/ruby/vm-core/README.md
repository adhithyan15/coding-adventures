# coding_adventures_vm_core

Generic Ruby register VM for LANG InterpreterIR modules. Frontends compile to
IIR once, then this VM executes the pure interpreter path and exposes the same
profiling hooks used by the JIT and backend packages.
