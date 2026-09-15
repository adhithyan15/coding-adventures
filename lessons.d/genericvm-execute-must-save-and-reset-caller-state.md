---
category: Compiler / VM / language pipeline
---

# `GenericVM.execute` must save and reset caller state

(pc, stack, call_stack, halted, vars, locals) for function calls, then restore after extracting the return value.
