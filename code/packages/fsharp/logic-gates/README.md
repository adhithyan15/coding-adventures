# CodingAdventures.LogicGates.FSharp

Core digital logic gates with validated bit inputs, NAND-derived equivalents,
and multi-input AND/OR/XOR helpers.

## Input validation

Every gate accepts `int` only and rejects anything other than 0 or 1 with
`InvalidBitException` (a subclass of `ArgumentOutOfRangeException`) exposing
`Parameter` (`"a"` or `"b"`) and `Value`. All arguments are validated before
any evaluation, so the first invalid parameter is reported under the gate's own
name, including for the NAND-built gates.

F# has no implicit numeric conversions, so `andGate 1L 0`, `andGate 1.0 0`,
`andGate true 0` and `andGate 'a' 1` do not compile: type confusion is
rejected at compile time and only out-of-range ints can fail at run time.

## N-ary gates

`andN` / `orN` (at least 2 inputs) and `xorN` (any length) accept at most
`LogicGates.maxInputs = 65536` inputs. Validation order: null, too long, too
short (all shape errors), then the first bad element, reported as
`InvalidBitException` with parameter name `inputs[i]`.

## Hardening

Warnings are errors (including nullness and FS0025), the library is compiled
with `--checked+`, the public API is pinned by `LogicGates.fsi`, and exception
messages use the invariant culture. Tests run in Debug and Release.
