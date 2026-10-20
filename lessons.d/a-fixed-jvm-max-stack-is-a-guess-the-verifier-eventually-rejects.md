---
category: Compiler / VM / language pipeline
---

# A fixed JVM max_stack is a guess the verifier eventually rejects; measure it from the emitted bytecode

**What went wrong.** `iir-to-jvm-class-file` declared `max_stack = 8` on every
method. The comment beside it said "in practice our emit patterns never exceed
~4". A captured multi-dimensional `double[]` store pushes an array reference,
an index and a two-word `double` on top of address arithmetic, which goes
past eight words. The JVM then rejected the class with `VerifyError: Stack
size too large`. Issue #12032 tracked that symptom for two months while the
whole `lang_matrix` target stayed excluded from CI. The same investigation
found a second limit hidden behind the first. `x86_64-backend` refused any call
with more than six arguments (VM-D005), and a captured 4-D array formal is
eight values.

**The fix.** `compute_max_stack` is the verifier's own algorithm: a worklist
walk of the emitted bytecode that tracks operand-stack depth on every path.
On its first run it reported a raw `0xB0` opcode (`areturn`) that the named
opcode table did not cover. Unknown opcodes, depth mismatches where branches
join, and underflows now fail lowering with a clear error instead of producing
a class the JVM rejects. On x86_64, overflow arguments use the standard
SysV/Microsoft x64 stack layout.

**Next time.** When a backend declares a resource bound (stack depth, register
count, frame size), compute it from what was emitted, or support the overflow
path the ABI already defines. A constant that only "should be enough" holds
until a richer program arrives, and then it fails far from where it was set.
Before excluding a test target from CI for a known red, link the issue in the
BUILD file. When that issue closes, turn the target back on.
