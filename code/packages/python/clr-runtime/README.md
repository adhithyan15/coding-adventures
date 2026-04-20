# clr-runtime

Top-level orchestration for the modular CLR prototype.

## Composable Pipeline

`CLRRuntimePipeline` exposes each CLR boundary as a separate stage:

```text
assembly bytes
  -> decode_assembly
  -> select_method
  -> disassemble_selected_method
  -> execute_disassembled_method
```

`CLRRuntime` remains the backward-compatible default facade. Use
`run_entry_point(...)` when you want the whole path, or replace individual
stage callables when a compiler backend wants to validate generated assemblies
without taking ownership of decoding, disassembly, or simulation.
