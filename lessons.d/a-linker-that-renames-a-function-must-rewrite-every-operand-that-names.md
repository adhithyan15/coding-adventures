---
category: Compiler / VM / language pipeline
---

# A linker that renames a function must rewrite every operand that names it, not just call callees

`iir-linker` renamed a colliding private function to `"<module>::name"` but
only rewrote `call` callees, and only for imported names. Twig numbers lambdas
from `__lambda_0` in every module, so the second module's
`alloc_closure(Str("__lambda_0"), …)` kept naming the first module's body:
same-arity closures silently computed the wrong value (1016 instead of 1006),
different-arity ones failed with an arity mismatch (#18421). A module's own
`call helper` was not redirected to its renamed `helper` either, and the
entry point was never mapped.

The fix keeps one table of function-name operand slots (`call`,
`alloc_closure`, `task_spawn` at `srcs[0]`; `group_spawn` at `srcs[1]`) and
applies a per-module rename map, built from the module's own renamed functions
plus its imports, to those slots and to `entry_point`. When you add an opcode
that names a function, add it to `function_name_slot` in
`iir-linker/src/merge.rs`. Never rewrite by operand *kind*: `const(Str(..))`
is a user string, and globals and `catch` kinds live in other namespaces.

A second trap came up while fixing it. A module can contain two functions with
the same name (Twig emits the user's `(define (main) …)` beside its
synthesized top-level `main`). The rename map has to bind a name to the
*first* such function, which is what `IIRModule::get_function` returns.
Letting the later duplicate overwrite the entry sent every self-compile test
to the empty synthesized `main`. Run the whole `twig-module-driver` suite
(it needs `code/packages/twig/` in a sparse checkout), not just the new
tests.
