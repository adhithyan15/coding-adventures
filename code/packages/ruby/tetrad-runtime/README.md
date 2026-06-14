# coding_adventures_tetrad_runtime

Ruby Tetrad frontend and runtime implemented directly on the LANG VM chain.
Source lowers to InterpreterIR, then runs on the pure Ruby VM or routes through
JIT/codegen backends for JVM, CLR, WASM, and pure VM artifacts.
