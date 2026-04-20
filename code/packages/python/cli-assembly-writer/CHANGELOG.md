# Changelog

## 0.1.0

- Add a composable CLI assembly writer for wrapping CIL method artifacts in a
  minimal managed PE container.
- Emit metadata roots, `#~`, `#Strings`, `#Blob`, and `#US` streams with
  Module, TypeRef, TypeDef, MethodDef, MemberRef, StandAloneSig, and Assembly
  rows.
- Preserve `ir-to-cil-bytecode` method and helper token conventions so emitted
  bytecode can be decoded by downstream CLR tooling.
