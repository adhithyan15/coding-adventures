# coding_adventures_dartmouth_basic_ir_compiler

Ruby Dartmouth BASIC frontend for the LANG VM chain. It lowers line-numbered
BASIC into InterpreterIR labels, branches, calls, and VM builtins, then runs on
the pure VM or routes through shared JVM, CLR, WASM, and pure VM artifacts.
