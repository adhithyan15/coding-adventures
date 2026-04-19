# compiler-ir

Haskell `compiler-ir` defines the tiny register-based intermediate
representation shared by the Haskell Brainfuck and Nib compiler pipelines.

The package is intentionally small: instructions carry an opcode, operands, and
a stable id; programs carry data declarations, an entry label, and a linear
instruction stream.
