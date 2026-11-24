# CodingAdventures.LogicGates.FSharp

Core digital logic gates with validated bit inputs, NAND-derived equivalents,
and multi-input AND/OR/XOR helpers.

## N-ary gates

`andN` / `orN` (at least 2 inputs) and `xorN` (any length) accept at most
`LogicGates.maxInputs = 65536` inputs. Validation order: null, too long, too
short (all shape errors), then the first bad element, reported as
`ArgumentOutOfRangeException` with parameter name `inputs[i]`.
