# CLI Assembly Writer

`coding-adventures-cli-assembly-writer` wraps CIL method-body artifacts in a
minimal PE/CLI assembly. It is the next composable CLR backend stage after
`ir-to-cil-bytecode`.

```text
compiler_ir -> ir-to-cil-bytecode -> cli-assembly-writer -> CLR PE bytes
```

The writer emits enough metadata for repository tooling to decode and
disassemble generated assemblies:

- PE32 shell with one `.text` section
- CLI header
- metadata root and stream directory
- Module, TypeRef, TypeDef, MethodDef, MemberRef, StandAloneSig, and Assembly
  metadata rows
- tiny and fat method-body headers

It does not yet emit runtime helper implementations, resources, strong names,
debug records, or native import stubs.
