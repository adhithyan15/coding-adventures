# Changelog

## 0.2.0 — 2026-04-28

### CLR01 Phase 1 — ECMA-335 conformance fixes

Brings the writer closer to real-`dotnet` loadability while keeping
every existing simulator user (clr-vm-simulator, brainfuck-clr,
oct-clr, nib-clr) green.  Real `dotnet 9.0.313` still rejects the
output with `BadImageFormatException` — see `code/specs/CLR01...md`
"Status (2026-04-28)" for what landed and what remains (CLR02).

Concrete changes in `writer.py`:

- **MS-DOS stub** at file offset 0x40 (canonical 64-byte sequence
  printing "This program cannot be run in DOS mode.").
- **`<Module>` pseudo-TypeDef** as TypeDef row 1, per
  ECMA-335 §II.22.37.  User TypeDef shifts to row 2.
- **AssemblyRef table** (0x23) with one row pointing at
  System.Runtime (net9.0 version, ECMA public-key token
  `b03f5f7f11d50a3a`).
- **System.Object TypeRef** as TypeRef row 2; user TypeDef's
  `Extends` column now points at it.
- **ResolutionScope** on every existing TypeRef rewired from 0
  (dangling) to AssemblyRef row 1.
- **`#GUID` metadata stream** + Module Mvid pointing at a
  deterministic GUID derived from the assembly name.
- **Stream order** changed to match real C#: #~, #Strings, #US,
  #GUID, #Blob.
- **COFF Characteristics** changed from `0x102`
  (legacy 32-bit-machine) to `0x22` (executable + large-address-
  aware) to match real-C# AnyCPU output.
- **Empty tables dropped from the Valid mask**: previously the
  MemberRef bit was set even with zero rows; real .NET rejects
  that as corrupt.

Test surface:

- `tests/test_writer.py::test_write_ir_lowered_fat_method_with_locals_and_internal_call`
  updated to expect `<Module>` at TypeDef row 0 and the user
  TypeDef at row 1 (the ECMA-335-correct layout).
- `tests/test_real_dotnet.py` import paths fixed
  (`CILBytecodeBuilder` from `cil_bytecode_builder`,
  `CILMethodArtifact` / `CILProgramArtifact` from
  `ir_to_cil_bytecode`, plus `SequentialCILTokenProvider` for the
  required `token_provider` field).

## 0.1.0

- Add a composable CLI assembly writer for wrapping CIL method artifacts in a
  minimal managed PE container.
- Emit metadata roots, `#~`, `#Strings`, `#Blob`, and `#US` streams with
  Module, TypeRef, TypeDef, MethodDef, MemberRef, StandAloneSig, and Assembly
  rows.
- Preserve `ir-to-cil-bytecode` method and helper token conventions so emitted
  bytecode can be decoded by downstream CLR tooling.
