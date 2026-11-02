---
category: Compiler / VM / language pipeline
---

# A JIT builtin callback that cannot return an error must still be able to fail the run

`GenericCirJit`'s builtin callbacks have the type `Fn(&[Value]) -> Value`, so
they cannot return an `Err`. `lang-aot`'s McCarthy embedding registered the
same function on the VM and on the JIT. Because the JIT shape cannot fail,
its `car` / `cdr` mapped "not a cons cell" to nil: `(CAR 5)` returned a value
on the JIT and failed on every other column (#18417). The comment called this
"graceful", and the test suite only exercised real pairs.

**Fix:** the compiled callback captures the JIT's `error_handle()` and records
the refusal there. `GenericCirJit` now stops right after a `CALL_BUILTIN`
whose callback recorded an error, rather than running on with the
placeholder result. The VM registration returns a real
`VMError::TypeError`. Both tiers report the same text.

**Do differently:** when one function is registered on two engines with
different error channels, check what happens to the error on *each*, and
test a failing input on each tier. Here, with the compiled callback's report
disabled, only the `LAMBDA` form (the compiled tier) showed the gap. A
top-level `(CAR 5)` ran on the interpreter tier and looked fine.
