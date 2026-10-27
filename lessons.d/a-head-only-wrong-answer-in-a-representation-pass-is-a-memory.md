---
category: Compiler / VM / language pipeline
---

# A HEAD-only wrong answer in a representation pass is a memory-safety bug

Draft 5 of the iir-builtin-lowering 0.43.0 call-boxing fix "accepted" a
small class of wrong answers as memory-safe: the misread value "only reaches
arithmetic". Review showed that a guard defeats that argument. Wrap any
expression `e` the pass misreads in `(if (= W(e) e) A (car 15))`. On the
reference engine the `car` branch is dead. On the miscompiled one the
comparison fails, the branch runs, and the program dereferences an integer.
A second generator built this way found 181 crashes on HEAD that main did not
have, in programs the earlier fuzzer had passed.

So in a pass that changes how values are represented, "wrong answer" and
"memory-unsafe" are the same severity. The acceptance bar is zero wrong
answers, measured relative to the old pass, not "zero crashes plus an
accepted gap". The concrete cause here was cons cells with no single
representation: main boxes a literal stored into a cell but stores a
computed value raw. Any reader convention the pass picked (unbox the `car`
result, or not) was wrong for some writer. The fix was to stop changing what
any cell holds or how any cell read is interpreted until the cells get one
representation, and to keep only the call-boundary changes that cells never
see.

Do differently: when a fuzzer reports a HEAD-only wrong answer, try to turn
it into a crash with a dead-branch guard before classifying it. Build that
guard into the generator (round-trip a value through the risky route and
compare it with the original). If a fix can only trade one wrong-answer class
for another, the real fix is a uniform representation; back the change out
rather than ship either side.
