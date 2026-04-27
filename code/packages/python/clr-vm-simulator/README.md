# CLR VM Simulator

`coding-adventures-clr-vm-simulator` executes decoded CLR method bodies using
the shared `cli-runtime-model` package. It is the first execution layer for the
composable CLR pipeline:

```text
PE/CLI bytes -> clr-pe-file -> clr-bytecode-disassembler -> clr-vm-simulator
```

The VM supports the current compiler-backend MVP:

- int32 constants, locals, arguments, arithmetic, bitwise operations, and
  comparisons
- short and long conditional/unconditional branches
- internal `MethodDef` calls and external `MemberRef` host calls
- `callvirt` receiver/null checks through the runtime model
- host helpers emitted by `ir-to-cil-bytecode`, including `__ca_syscall`

It is deliberately not a full CLR implementation yet: object allocation,
exceptions, generics, verification, and full metadata binding remain future
work.
