---
category: Compiler / VM / language pipeline
---

# A JIT column driven by execute_with_jit interprets every function that prints, so assert is_compiled

**What went wrong.** The LANG83 conformance suite asserted
`JITCore::is_compiled("main")` after `execute_with_jit` on a one-line program
(`print_i64 42`). The assertion failed. `execute_with_jit` eagerly compiles only
`FullyTyped` functions, and `IIRFunction::infer_type_status` does not count
`"void"` as a concrete type. Any function containing `call_builtin … : void`, or
a `jmp`/`label`, is therefore `PartiallyTyped` and runs on the vm-core
interpreter tier. It produced the right output, so nothing looked wrong. The
`lang_matrix` JIT column drives every printing program this way, so its green
cells prove the interpreter, not the JIT.

**Fix (in the conformance suite).** Compile `main` explicitly with
`JITCore::compile` and assert `Ok(true)` plus `is_compiled`, then run it with
`JITCore::execute`. Read the JIT-side error slot (`GenericCirJit::error`) before
moving the backend into `JITCore`, because `Backend::run` cannot return an
error. The matrix runner and the type-status rule are a separate follow-up
(backlog VM-078d).

**Next time.** A column with a silent fallback tier has to prove which tier ran.
Assert the compiled state, not just the output. When a JIT "passes" a new
feature with no JIT code change, suspect the fallback first.
