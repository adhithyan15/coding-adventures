# CodingAdventures.LogicGates.CSharp

Core digital logic gates with validated bit inputs, NAND-derived equivalents,
and multi-input AND/OR/XOR helpers.

## Input validation

A bit is the `int` 0 or 1. Any other value throws `InvalidBitException`, a
subclass of `ArgumentOutOfRangeException` carrying `ParamName` (the first
invalid parameter, in declaration order) and `Value`. Every gate, including the
NAND-built ones, validates all its own arguments before computing.

C# implicitly widens `char`, `byte`, `sbyte`, `short` and `ushort` to `int`, so
`And('a', 1)` compiles and is rejected at runtime as 97; `long`, `uint`,
`double` and `bool` do not convert and fail to compile.
