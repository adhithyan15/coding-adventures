---
category: Compiler / VM / language pipeline
---

# Differentially fuzz a representation pass against a reference VM and the old pass before trusting hand-picked probes

The iir-builtin-lowering 0.43.0 call-boxing fix went through five security
reviews. Each review hand-picked a new program that crashed or gave a wrong
answer, and each fix was checked only against the programs already known. A
representation pass (raw vs tagged words) has too many interacting shapes for
that to converge: `let` rebinding a parameter, `mov` joins, closure
captures, the closure dispatcher's unbox, records, forward globals, and every
consumer of each.

What converged was a differential fuzzer, kept out of the repo. A small typed
random generator of Twig programs (functions, `let` shadowing, `if`/`match`,
cons cells, lambdas, records, forward globals; literals whose low bits are
`0b111`, like 7, 15 and 1229782938247303439) ran on three engines: the VM
(which never runs the pass, so it is an independent reference),
native-on-main (with main's pass sources swapped in) and native-on-HEAD. Then
two filters: HEAD crashes and main doesn't; HEAD is wrong and main is right.
Ten thousand programs took about eight minutes on four cores, and found two
real bugs the reviews had not (a parameter `mov` cycle, and a cell-reading
function's result read raw) plus a third instance of a known gap class.

Do differently: for any pass that changes how values are represented, build
the three-way differential harness *before* the first review, and treat
"zero HEAD-only crashes over N thousand programs" as the entry bar. Generate
typed programs (so most are valid on the reference) and bias them toward the
pass's risk: values flowing across calls, through joins and into the heap.
Run every reviewer probe file through the same harness, so a fixed finding
stays fixed.

Check that the generator's programs actually test what they claim. The first
generator appended `(define gN lit)` *after* the main expression, so the
program's exit code was the trailing define's value and the expression under
test was never observed: 5,789 of 17,904 "agreeing" programs were masked. Put
globals first, and spot-check that the exit value depends on the expression.
Adjudicate programs the reference refuses (here the VM refuses `pair?` on an
integer and `<` on a parameter) with an independent reference interpreter,
not by dropping them — they were where the remaining HEAD-only wrong answers
hid. The corrected run (37,614 programs, four generators) found one HEAD-only
crash, which was a downstream bug (#18483) that a correct value exposed.
