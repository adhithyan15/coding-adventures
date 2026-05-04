# coding_adventures_jit_core

Ruby JIT coordinator for LANG InterpreterIR. It owns tiering decisions and can
register executable handlers on `VMCore` while also exposing the shared backend
registry for JVM, CLR, WASM, and pure VM artifacts.
