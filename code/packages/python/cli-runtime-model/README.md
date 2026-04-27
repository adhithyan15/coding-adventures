# CLI Runtime Model

`coding-adventures-cli-runtime-model` provides reusable state objects for CLR
execution packages. It sits below a future `clr-vm-simulator` and keeps runtime
concepts out of decoders, disassemblers, and assembly writers.

The package models:

- CLI types, method signatures, methods, fields, and metadata tokens
- typed CLI values, null references, heap references, and boxed values
- evaluation stacks, argument/local slots, frames, and thread state
- map-backed token resolution for methods, fields, strings, and types
- `call` / `callvirt` argument collection and null checking
- exception-handler lookup over decoded method regions

It does not execute instructions by itself. Execution engines compose these
objects with decoded CIL instructions and host bindings.
