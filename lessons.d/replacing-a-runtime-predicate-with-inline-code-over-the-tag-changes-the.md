---
category: Compiler / VM / language pipeline
---

# Replacing a runtime predicate with inline code over the tag changes the answer for malformed words; reproduce the runtime's exact test, and test words outside the well-formed set

**What went wrong.** LANG83 PR7f-2 moved Lisp truthiness out of the C
runtime into compiler-emitted IIR. The runtime's `__dyn_truthy` was
`(v == LISPY_FALSE || v == LISPY_NIL) ? 0 : 1` — a test of the *whole
word*. The inline replacement tested the *tag*: `(dyn_tag x ≠ 3) ×
(dyn_tag x ≠ 1)`. For every well-formed value the two agree, because nil
and `#f` are whole-word singletons. For a malformed word they do not: a raw
machine 9 (`0b1001`) has nil's tag without being nil. Such words exist —
Twig passes an unboxed parameter straight into `dyn_pair_new` — so
`(define (f n) (if (car (cons n (list))) 1 2)) (f 9)` exited 1 before the
change and 2 after it, on LLVM and native x86-64. The unit tests compared
the new IIR with an oracle that *was* the whole-word rule, but only over
well-formed values, where tag and word agree; a security review found it.

**Fix.** Truthiness compares the word itself: `cmp_ne x, 3` and
`cmp_ne x, 1` typed `i64`, then `mul` — still straight-line and
JIT-compilable, and one instruction shorter. The reference-machine tests
now include malformed words (9, 11, 13, 17, 19, 25, 27, high-bit words, a
raw pair-tagged word), and `lang_matrix` runs the Twig repro on LLVM,
native x86-64 and the VM expecting the pre-change exit code.

**Do differently.** When re-expressing a runtime function in another form
(inline IIR, a helper, a different backend), transcribe the function's
exact test rather than a description of what it means ("false iff nil or
`#f`"); two formulations that agree on every well-formed input can still
differ on the inputs a known gap produces. Put inputs *outside* the
well-formed set — values the representation can hold but no constructor
should make — in the equivalence test, and take the expected results of
an end-to-end regression from the old build, not from the new code.
