## Unreleased - 2026-10-27

### Fixed

- Add 82 Twig matrix rows pinning how values cross a user-function call on the
  tagged-word columns (iir-builtin-lowering 0.43.0), each checked against the
  VM (or, where the VM refuses the program, against the known result) on
  NativeAot, LLVM and every other column that runs it. On main NativeAot/LLVM
  gave a wrong answer or crashed on many of them — `(define (h y) (+ y 1))
  (h 1)` exited 9 (VM 2), `(h 1 2)` with `(+ y z)` exited 24, `(h (h 1))`
  exited 73, a recursive `(loop 3)` overflowed the stack, and an identity
  function returning a cell through a copy crashed (SIGSEGV). The rest are
  guards from eight security reviews and four differential fuzz generators:
  reassigned parameters, `mov` joins, closure captures and pointers through the
  closure dispatcher, mixed-use parameters, records, strings and symbols
  through identity functions, values stored into and read from cells (which
  keep main's representation), and the reviewers' guarded crash programs. CLR
  and BEAM were not available to verify.
