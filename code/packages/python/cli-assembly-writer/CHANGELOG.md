# Changelog

## 0.4.0 — 2026-04-30 — System.Int32 TypeRef (closure box support)

Adds a third TypeRef row for `[System.Runtime]System.Int32` so
the CIL `box` opcode has a stable TypeDefOrRef token to reference
when boxing int32 closure returns into the polymorphic
`object`-returning `IClosure::Apply` contract (TW03 Phase 3
follow-up — closure-returning closures).

Always emitted (small cost; deterministic token assignment).
Token is `0x01000003` (TypeRef table tag 0x01, row 3).

## 0.3.0 — 2026-04-29 — CLR02 Phase 2b (multi-TypeDef metadata)

### Added — extra TypeDefs alongside the user's main type

Foundation for [CLR02 Phase 2 closures](../../../specs/CLR02-closure-lowering.md).
The writer previously hardcoded exactly two TypeDef rows
(`<Module>` + the user's main type).  It now accepts an arbitrary
list of additional types via `CILProgramArtifact.extra_types`.

Each extra type is a `CILTypeArtifact` carrying:

* `name`, `namespace` — fully qualified type identifier.
* `is_interface` — toggles between `Public + Class +
  BeforeFieldInit` (0x00100001) and `Public + Interface +
  Abstract` (0x000000A1) `TypeAttributes` flags.
* `extends` — `"System.Object"` (resolves to the existing
  TypeRef row 2), a same-module type's qualified name (resolves
  to its `TypeDef` row), or `None` for interfaces.
* `implements` — same-module interface qualified names; emits
  `InterfaceImpl` table rows (0x09).
* `fields` — list of `CILFieldArtifact` entries, each producing
  a row in the `Field` table (0x04).
* `methods` — list of `CILMethodArtifact` with new flags
  `is_instance` (sets `HASTHIS` in the MethodSig),
  `is_special_name` (used for `.ctor`), and `is_abstract`
  (RVA=0, no body).

### Validation

- `implements` references that point at types not declared in
  the same assembly are rejected.
- `extends` values other than `"System.Object"` or a known
  same-module TypeDef are rejected (CLR02 v1 doesn't yet wire up
  cross-assembly base classes other than `System.Object`).

### Tests

- 2 new real-`dotnet` tests prove that an extra `IClosure`
  interface, and an `IClosure` + a concrete `Closure_X`
  implementing it with one `int32` field, both load and the
  user's `Main` still returns 42.
- 4 new structural tests cover the typedef table, method-table
  growth, and the `extends` / `implements` validation paths.

### Backwards compatibility

`extra_types` defaults to `()`; callers that don't opt in get
byte-identical output to 0.2.0.  All 97 upstream tests
(`ir-to-cil-bytecode`, `twig-clr-compiler`) stay green without
modification.

## 0.2.0 — 2026-04-29

### CLR01 — ECMA-335 conformance fixes (LANDED end-to-end)

Real `dotnet 9.0.313` now executes our assemblies and returns the
expected exit code.  All simulator users (clr-vm-simulator,
brainfuck-clr, oct-clr, nib-clr) stay green.

The previously-flagged "CLR02 follow-up" turned out to be **one
bug**, not three: the pre-existing Assembly-table row encoding was
2 bytes too short (missing the ECMA-335 §II.22.2 `Culture` column).
Fixed in this same release.

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
- **Assembly row width fix (the actual root-cause)**: the format
  changed from `<IHHHHIHH>` (8 fields = 20 bytes, missing the
  Culture column) to `<IHHHHIHHH>` (9 fields = 22 bytes,
  correctly mapped) per ECMA-335 §II.22.2.  Without this, the
  AssemblyRef table that follows started 2 bytes early and
  `System.Reflection.Metadata.AssemblyRefTableReader` read past
  end-of-stream — that was the cryptic "BadImageFormatException:
  File is corrupt" we'd been chasing.

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
