# brainfuck-ir-compiler

This package lowers the Haskell Brainfuck AST into the local `compiler-ir`
representation. The first convergence implementation preserves the pipeline
shape and emits a runnable `_start` shell while retaining source operations as
IR comments for downstream diagnostics.
