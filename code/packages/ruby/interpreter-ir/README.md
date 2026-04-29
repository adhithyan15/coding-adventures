# coding_adventures_interpreter_ir

Ruby value objects for the LANG InterpreterIR chain. Language frontends lower
source programs into `IIRModule`, `IIRFunction`, and `IIRInstr` instances, then
share the same VM, JIT, and backend pipeline.
