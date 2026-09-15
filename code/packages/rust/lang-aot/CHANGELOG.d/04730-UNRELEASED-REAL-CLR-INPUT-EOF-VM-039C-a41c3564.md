## Unreleased — real CLR input/EOF (VM-039c)

Textual CIL now lowers input_more through Console.In.Peek without consuming
input. Numeric input uses width-matched TryParse and scratch locals, returning
zero for EOF or malformed values while allowing I/O errors to propagate.
Four FLOW-MATIC matrix rows execute on real CoreCLR; direct regressions cover
EOF, repeated peeks, mixed string/numeric input and 32/64-bit destinations.
The encoded CIL simulator path still does not implement numeric/string input.

