# brainfuck-ir-compiler

This package lowers the Haskell Brainfuck AST into the local `compiler-ir`
representation. It emits tape-backed IR with byte loads/stores, pointer
movement, structured loop labels, and syscall markers for Brainfuck I/O.
