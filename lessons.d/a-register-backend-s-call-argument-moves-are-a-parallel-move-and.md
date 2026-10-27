---
category: Compiler / VM / language pipeline
---

# A register backend's call-argument moves are a parallel move, and emitting them in argument order clobbers a source that sits in an earlier argument slot

**What went wrong.** `iir-to-beam` lowered a local `call f(a, b, c, …)` to one
`move {x,src} {x,i}` per argument, in argument order. When an argument's
register is also an earlier argument's destination, the earlier move
overwrites it before it is read. LANG83 PR7c-py's `__py_write_float` was the
first emitted IIR to do this. Its constant `zero` sits in x1, and it calls
`__py_write_digits(d, n, zero, e1)`. So `move n → x1` ran before `move x1 → x2`,
and the callee got `from = n` instead of `0`. On real OTP every digit run that
started at the leading digit printed nothing. `print(1.5)` wrote `.5` and
`print(1e16)` wrote `e+16` (macOS BEAM matrix cell, PR #18400). The bug had
been in the call arm since `lower.rs` first landed. Its comment even claimed
"we collect the (arg_reg, target_x) pairs first to avoid aliasing issues", but
the code never did. Earlier front ends happened to pass arguments whose
registers never collided this way.

It got past local testing for two reasons. Hosts without `erl` skip the
matrix's BEAM column. The only local BEAM check for Python float prints
(`python_float_prints_lower_on_beam_once_the_arithmetic_is_gone`) compiles the
module and never runs it. The opcode model in `iir-to-beam/tests` executes
moves one at a time, just as BEAM does, so it would have caught the bug. But
no model test ran a call whose arguments were shuffled between slots.

**Fix.** `sequence_parallel_moves` orders the moves. It drops moves that are
already in place, and repeatedly emits the first move whose destination no
pending move still reads. When only cycles remain, it parks one destination in
a scratch register at or above `max(next_reg, arity)` and goes on. A call
whose old order was already safe keeps exactly the same instruction stream.
`every_arrangement_of_call_arguments_reaches_the_callee_intact` runs all 625
ways of passing 4 arguments from 5 registers on the model, and checks that
safe orders are unchanged.

**Do differently.** Treat every "move N values into N fixed registers" site as
a parallel move: call arguments, BIF/`call_ext` staging, and phi-like
copies. The other `call_ext` arms in this file already stage through scratch
registers for this reason, and the plain `call` arm was the one that did not.
A comment saying aliasing is handled is not evidence; a test that permutes
the sources is. To reproduce a real-emulator-only failure without `erl`, run
the real front end's module on the opcode model. A scratch crate that
`include!`s `tests/lang83_pr6a.rs` reproduced CI's exact wrong output. OTP 25
(Ubuntu's `erlang-base`) cannot load this backend's modules, which target
OTP 27+ opcodes, so it is no substitute for CI's emulator.
