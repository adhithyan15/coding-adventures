---
category: Compiler / VM / language pipeline
---

# A call boundary needs one representation that caller and callee both read from the same signature

On the tagged-word pipeline (native AOT + LLVM), Twig `(define (h y) (+ y 1))
(h 1)` exited 9, `(h 1 2)` with `(+ y z)` exited 24, and `(+ 50 (g (lambda (y)
(+ y 1))))` exited 98 — while the VM, WASM and JVM gave 2, 3 and 56. Two passes
in `iir-builtin-lowering` each decided the boundary representation on their own:

- `lower_dynamic_arith` read the bare-`any` parameter `y` as a raw integer and,
  because it re-boxed the sum, retyped `h -> ref<any>`.
- `lower_dyn_repr` then put `h` on the tagged boundary *because of that return
  type* and boxed the caller's literal `1` to `8`. Callee read raw, caller wrote
  tagged: `8 + 1 = 9`.

The 98 was a second instance of the same mistake inside one pass:
`lower_dynamic_arith` rewrote `main`'s `+` while `call g` still carried the
frontend's `any`, and relabelled the call `ref<any>` only afterwards, so the
boxed result `6 << 3` was added as a machine integer.

Two tempting fixes were both wrong. Boxing an argument only when the callee
*declares* a `ref<…>` parameter moves the bug to `(define (f x) (cons x 0))
(car (f 5))`, where the same bare-`any` parameter must be tagged. Making the
boundary uniformly tagged — callee unboxes its `any` parameters, caller boxes
every "raw-looking" argument — failed review as memory-unsafe: a bare `any` is
either a heap pointer (the same word in both models) or an integer (`n` vs
`n << 3`), so boxing an identity function's result or unboxing a captured
value turned pointers into wild reads, and `mov` aliases were missed.

What worked was converting only what can be proven. A parameter read by nothing
but arithmetic (through `mov` copies, identity calls, or forwarding to another
such parameter) can never be dereferenced, so the caller passes it a raw
integer: literals stay raw, and an argument traced back to a `box` (never a
cell read) is unboxed. Everything else keeps its old lowering. (An attempt to
fix the 98 by settling `lower_dynamic_arith`'s call-result labels early was
later backed out — see the seventh review below.)

A second review found the same mistake one level down: a proof is only as
good as the invariant it leans on. Step 3b treated a `car` result as a tagged
integer, but cells filled with a computed raw value (`(cons (+ 2 3) nil)`,
`(cons (string-length s) nil)`, a record constructor's literal fields) were
never boxed, so the "proof" unboxed a raw word. Fixing it meant enforcing the
invariant at every store — box provable raw integers going into `cons`,
`field_store` and tagged call arguments — and checking that a function summary
("returns its argument") still holds after a later step (`box_raw_scalar_return`)
rewrites the return. Several programs had only ever worked by two cancelling
mistakes (stored raw, read raw); correcting one side exposes the other, so add
rows for them before changing either side.

A third review found the analyses themselves running ahead of the rewrite.
They were computed before lowering under assumptions the rewrite did not keep:
that a parameter register is never written (Twig's `let` reuses the name, so
`(let ((x 15)) x)` overwrote it and the "tagged parameter" was returned raw —
SIGSEGV in `pair?`), and that "provably raw" values were exactly what step 3c
boxes (3c skipped anything a `mov` join had put in `boxed_regs`). The fix was
structural: compute every fact with the same functions the rewrite uses, on a
prediction of the rewrite's own classification, and give each assumption a
guard (`is_reassigned`). Also, a fix that changes one consumer of a shared
slot (record fields, cons cells) changes the contract of every producer: the
record-field boxing that fixed literal stores broke forward-global stores, so
records went back to raw fields and the dynamic-arith labelling was narrowed to
results this pass itself boxed.

A fourth review caught the same trap in a smaller fix: dropping one half of a
round trip. `lower_closures_to_heap` boxes every closure argument and the
dispatcher unboxes it; skipping the unbox "because this lambda only conses the
value" kept the box, so a cell pointer arrived as `p << 3`. A conversion
emitted by another pass is half of a pair — find its partner before removing
it, and test with a pointer, not just an integer.

A fifth review found the over-boxing case of the same proof problem: boxing
a provably raw argument is only right when the callee reads the parameter as
tagged *everywhere*. A mixed-use parameter (`(if (= x 15) x (car l))`) is read
raw by arithmetic, so boxing it sent the program down a dead branch into
`car 15`. The rule is the dual of raw-only: convert at a call only for a
parameter whose every use wants that representation.

The sixth review ended the cell experiments: cons cells had no single
representation (main boxes literals stored into a cell, stores computed values
raw), so boxing more values into cells, or reading cell results as tagged,
broke whichever writer still stored raw. The change had to back out to "cells
hold exactly what they held on main" (#18427 deferred to the uniform
representation work, #18451/#18480) — see the lesson on HEAD-only wrong
answers being memory-safety bugs.

The seventh review backed out the last two conversions that touched shared
state. Labelling a call result `ref<any>` early in `lower_dynamic_arith` put it
in `boxed_regs`, which spread through the caller's `mov` join to a raw literal
on the other arm; step 3 then boxed the literal after the consumer had already
been lowered as a raw read (`(+ (if (= 1 2) (f 1 #t) 5) 1)` gave 41, and a
boxed `#f` read as true). Boxing a provably raw value for `pair?` made
`(pair? 7)` correctly false — and exposed a raw cell read that main had been
answering right by accident. What survived is only what changes no shared
slot: literals for raw-only parameters, unboxing proven tagged arguments for
them, and returning a tagged parameter unchanged. Even that exposed one more
coincidence: a value now computed correctly as -1 met the native backend's
unsigned `<` (#18483), which main's wrong positive value had dodged. A fix in
a representation pass ships stacked on the fixes for the bugs its correct
values uncover.

Do differently: when two passes touch the same function boundary, find the one
place the representation is decided and make both read it; never let pass B
infer the ABI from a signature pass A rewrote for a different reason. When a
type hint is ambiguous between a pointer and an integer, do not convert on the
hint: convert only on a proof (where the value came from, or that it is never
dereferenced), and test the fix with pointers through the same path — the
identity function, the closure capture, the `if` join.
