---
category: Security boundaries
---

# A JIT handler that reports failure through a side error slot fails open unless the call path drains the slot

**What went wrong.** LANG83 PR1b taught `GenericCirJit` to compile `trap`
and the trapping `i64_div_floor` / `i64_mod_floor`. `Backend::run` returns a
plain `Value`, so a trap wrote `trap: <kind>` into the backend's error slot
and returned `Value::Null`. The vm-core JIT handler (`Fn(&[Value]) -> Value`)
handed that `Null` to `call`, which assigned it and moved on. Nothing on the
call path read the slot. An interpreted caller of a trapping JIT-compiled
function kept running, and `execute_with_jit` returned `Ok`. The security
review's repro returned `Ok(Some(Int(0)))` after running two `mark` builtins
past the trap. `div_i64` by zero had always failed open the same way. Every
test passed because every JIT test compiled `main` itself and checked the slot
after `execute` returned. Nobody tested the shape that fails: an interpreted
caller of a compiled callee. `set_error` keeping only the first message made
it worse: a slot that nobody cleared would hide a later failure behind an
earlier one.

**The fix.** Make the boundary fallible. The vm-core handler type is
`Fn(&[Value]) -> Result<Value, VMError>`, and `call` propagates `Err` with
`?`. `Backend::take_error()` (default `None`) drains the slot. `JITCore`
calls it after every `run`, in the registered handler and in
`JITCore::execute`, and returns `Err`. Draining (`take`, not `clone`) also
stops stale messages.

**Next time.** When an API cannot return an error and reports it on the
side, find the code that reads the side channel on every path that consumes
the return value. If no such reader exists, the error is ignored. Change the
signature rather than adding another reader. When you add a failing operation
to a tier, test it from a caller on a different tier: an interpreted caller,
a statement after the call that has an observable side effect, and an
assertion that the side effect did not happen.
