# Adding match arms to a hot recursive fn can overflow a calibrated deep-recursion guard (CLOC12.171)

When a new AST `Expression` variant forces new arms into a *recursive* dispatch fn
(`constant-fold`'s `fold_expression`), building the new node structs INLINE in the
match enlarges that fn's per-level stack frame in debug builds (each arm gets its own
slots). `constant-fold` folds a 20 000-deep binary chain on a fixed 64 MiB worker
(~3.2 KiB/frame budget); the enlarged frame overflowed it — a test green on origin/main
went red on the branch. FIX: delegate new arms to `#[inline(never)]` helpers (mirroring
the existing `fold_member`/`fold_call` delegation) so their locals live in the helper
frame, entered only when that node is actually hit — never on the hot binary path.
LESSON: after adding arms to a recursive dispatch fn, run the deep-nesting DoS-guard
tests; prefer delegating heavy arms out-of-line rather than inlining struct construction.
