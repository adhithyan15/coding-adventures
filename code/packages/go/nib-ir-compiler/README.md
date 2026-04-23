# nib-ir-compiler

Go compiler stage that lowers typed Nib ASTs into the shared IR.

`CallSafeConfig` stages arguments through virtual registers `v2+`, copies
function parameters into local virtual registers on entry, and returns through
virtual register `v1`. `ReleaseConfig` keeps the compact legacy register layout
used by the Intel 4004 path.
