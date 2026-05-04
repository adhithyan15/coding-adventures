# CLR01 — Real-`dotnet` conformance for `cli-assembly-writer`

## Status (2026-04-29) — LANDED

CLR01 is **complete**.  Real `dotnet 9.0.313` now runs our
assemblies end-to-end.  The previously-flagged "CLR02 follow-up"
turned out to be **one bug**, not three: the pre-existing
Assembly-table row encoding was 2 bytes too short (missing the
ECMA-335 §II.22.2 `Culture` column).  Fixed in the same Phase 1
PR.  Concrete shipped fixes:

1. ✅ 64-byte canonical MS-DOS stub at file offset 0x40.
2. ✅ `<Module>` pseudo-TypeDef as TypeDef row 1 (ECMA-335 §II.22.37).
3. ✅ AssemblyRef table for System.Runtime + System.Object
   TypeRef + ResolutionScope wired on every TypeRef + user
   TypeDef's `Extends` column → System.Object.
4. ✅ `#GUID` metadata stream + Module `Mvid` pointing at it.
5. ✅ COFF Characteristics flag set to `0x22` (matches real-C#
   AnyCPU output) instead of legacy `IMAGE_FILE_32BIT_MACHINE`.
6. ✅ Empty tables (MemberRef, StandAloneSig) no longer flagged in
   the `Valid` mask — real .NET rejects empty-but-marked tables.
7. ✅ **The actual root-cause fix**: Assembly row format extended
   from `<IHHHHIHH>` (8 fields = 20 bytes, missing Culture) to
   `<IHHHHIHHH>` (9 fields = 22 bytes, correctly mapped).  Without
   this the AssemblyRef table that follows starts 2 bytes early
   and `System.Reflection.Metadata.AssemblyRefTableReader..ctor`
   reads past end-of-stream — producing the cryptic
   "BadImageFormatException: File is corrupt" we hit before.

Diagnostic that found it: a tiny C# program loading our assembly
via `PEReader.GetMetadataReader` and printing the precise
out-of-bounds stack trace.  Real `dotnet` itself only ever says
"File is corrupt"; the framework `System.Reflection.Metadata`
APIs give actually-actionable errors.

End-to-end test:

```
$ python3 build_x42.py        # writes a "return 42" assembly
$ dotnet X42.exe; echo $?
42
```

`tests/test_real_dotnet.py::test_return_42_runs_on_real_dotnet`
and `test_return_zero_runs_on_real_dotnet` now pass.

## Why this spec exists

Every CLR backend in the repo today
(`brainfuck-clr-compiler`, `oct-clr-compiler`, `nib-clr-compiler`, the
prototype `twig-clr-compiler`) emits PE/CLI assemblies that run **only
on the in-house `clr-vm-simulator`**.  Real `dotnet` (tested on 9.0.313)
rejects every one of them with:

```
System.BadImageFormatException: File is corrupt. (0x8013110E)
```

The simulator is permissive in ways real .NET isn't, and no existing
test in the repo runs CLR output through real `dotnet`.  This spec
brings `cli-assembly-writer` up to ECMA-335 conformance so the same
output runs both on the simulator AND on real `dotnet`.  The fix is
**repo-wide** — every existing CLR backend inherits conformance for
free.

This is the *first* of two linked "real-runtime correctness" tracks:

| Spec   | Backend | Gap                                                  |
|--------|---------|------------------------------------------------------|
| CLR01  | CLR     | `cli-assembly-writer` lacks ECMA-335 essentials      |
| JVM01* | JVM     | `ir-to-jvm-class-file` uses a class-level static reg |
|        |         | array → recursion is broken across method calls      |

*JVM01 is implied by the ``test_recursion_factorial_small`` xfail in
the new ``twig-jvm-compiler``.  It's a separate spec/PR; CLR01
focuses only on the CLR side.

## Sister sample

I built a known-good reference assembly with C# / `dotnet build` that
returns 42, and compared it byte-by-byte against what
`cli-assembly-writer` produces for the same logical program.  The
gap surface is small but every item has to land for `dotnet` to
load the file:

| Field                    | Real .NET / C#       | cli-assembly-writer today |
|--------------------------|----------------------|---------------------------|
| File size (`return 42`)  | 4608 bytes           | 1536 bytes                |
| MS-DOS stub @ 0x40       | "This program cannot be run in DOS mode." | zeros |
| TypeDef rows             | `<Module>`, Program  | Program only              |
| AssemblyRef rows         | 1 (System.Runtime)   | **0**                     |
| TypeRef resolution scope | `(AssemblyRef × 1)`  | 0 (no parent)             |
| Manifest                 | TargetFramework attr | absent                    |

The **single most important gap is AssemblyRef.**  Without it, every
`TypeRef` row's resolution scope points at nothing, so real .NET
can't load any external type — not even the primitive types every
assembly transitively depends on.

## Detailed byte-level plan

The fix lands in three layered chunks.  Each chunk is independently
testable: after Chunk 1 the existing simulator tests still pass and
the conformance fixture's error message changes (no longer "File is
corrupt"); after Chunk 2 the file loads but may fail later in
verification; after Chunk 3 a `return 42` assembly exits 42.

### Chunk 1 — DOS stub and ECMA-335 cosmetic header

**Files touched:** `cli-assembly-writer/src/cli_assembly_writer/writer.py`

**ECMA-335 reference:** §II.25.2.1 (PE file header) + §II.25.2.2
(MS-DOS header).

The current writer leaves bytes 0x00–0x80 as zeros except `e_lfanew`
at offset 0x3C.  Real .NET's `PEReader` validates that the MS-DOS
stub matches the canonical value before continuing.  Add the
standard 80-byte stub at file offset 0x40:

```
0e 1f ba 0e 00 b4 09 cd 21 b8 01 4c cd 21 54 68
69 73 20 70 72 6f 67 72 61 6d 20 63 61 6e 6e 6f
74 20 62 65 20 72 75 6e 20 69 6e 20 44 4f 53 20
6d 6f 64 65 2e 0d 0d 0a 24 00 00 00 00 00 00 00
```

(That's a tiny x86 prologue followed by the printable string "This
program cannot be run in DOS mode.\r\r\n$".)

**`e_lfanew` at offset 0x3C** must point at the PE header start
(0x80 today — keep it).

### Chunk 2 — `<Module>` pseudo-TypeDef and metadata heap fixes

**Files touched:** same writer.py.

**ECMA-335 reference:** §II.22.37 (TypeDef row 1 special).

The TypeDef table must have `<Module>` as the **first row**.  It owns
module-level fields and global functions; user types start at row 2.
Real .NET's metadata loader hard-rejects any TypeDef table that
doesn't begin with this row.

Schema: `Flags=0, Name="<Module>", Namespace="", Extends=0,
FieldList=1, MethodList=1`.

```python
_TYPE_DEF_TABLE: [
    # NEW: row 1 — the <Module> pseudo-type.
    struct.pack(
        "<IHHHHH",
        0,                                      # flags
        strings.add("<Module>"),                # name
        strings.add(""),                        # namespace (empty)
        0,                                      # extends (no base type)
        1,                                      # FieldList (1-based; no fields)
        1,                                      # MethodList (1-based; first method)
    ),
    # row 2: existing user type, but its FieldList / MethodList
    # indices need to skip past <Module>'s zero-length lists.
    struct.pack(
        "<IHHHHH",
        0x00100001,
        type_name_index,
        type_namespace_index,
        ???,                                    # see below
        1,
        1,
    ),
],
```

After this chunk the *first* meaningful real-.NET error message
should change.  If we still see "File is corrupt", look at the
`Extends` column on row 2 — Chunk 3 wires that to a real System.Object.

### Chunk 3 — AssemblyRef table + ResolutionScope wiring

**Files touched:** same writer.py.

**ECMA-335 reference:**
- §II.22.5 (AssemblyRef table).
- §II.24.2.6 (ResolutionScope coded index).

#### 3a. Emit the AssemblyRef table (0x23)

System.Runtime for net9.0 looks like (from a reference C# build):

| Column          | Value                            |
|-----------------|----------------------------------|
| MajorVersion    | 9                                |
| MinorVersion    | 0                                |
| BuildNumber     | 0                                |
| RevisionNumber  | 0                                |
| Flags           | 0                                |
| PublicKeyOrToken | 0xb03f5f7f11d50a3a (ECMA token) |
| Name            | "System.Runtime"                 |
| Culture         | "" (empty)                       |
| HashValue       | 0                                |

Encoded row:

```python
_ASSEMBLY_REF_TABLE = 0x23

_ASSEMBLY_REF_TOKEN_PREFIX = 0x23000000

# AssemblyRef row layout per §II.22.5:
#   USHORT MajorVersion, MinorVersion, BuildNumber, RevisionNumber
#   ULONG  Flags
#   BLOB   PublicKeyOrToken
#   STRING Name
#   STRING Culture
#   BLOB   HashValue
ecma_token = bytes.fromhex("b03f5f7f11d50a3a")
ecma_token_blob = blobs.add(ecma_token)
system_runtime_name = strings.add("System.Runtime")
empty_culture = strings.add("")
empty_hash_blob = blobs.add(b"")

_ASSEMBLY_REF_TABLE: [
    struct.pack(
        "<HHHHIHHHH",
        9, 0, 0, 0,                  # 9.0.0.0
        0,                           # Flags
        ecma_token_blob,             # PublicKeyOrToken
        system_runtime_name,         # Name
        empty_culture,               # Culture
        empty_hash_blob,             # HashValue
    ),
],
```

#### 3b. ResolutionScope on existing TypeRef rows

ResolutionScope is a coded index packing the table tag in the low 2
bits.  AssemblyRef = tag 2.  With one AssemblyRef row at index 1,
the encoded value is `(1 << 2) | 2 = 6`.

```python
_TYPE_REF_TABLE: [
    struct.pack(
        "<HHH",
        6,  # ResolutionScope = AssemblyRef row 1 (was 0 = dangling)
        helper_name_index,
        helper_namespace_index,
    ),
],
```

#### 3c. Update the user TypeDef's `Extends` column

In the table above, `Extends` is a TypeDefOrRef coded index pointing
to `System.Object`.  We need to *also* add a TypeRef row for
`System.Object` (in System.Runtime), and reference it from the
user TypeDef's `Extends` column.

Add a TypeRef row 2:

```python
_TYPE_REF_TABLE: [
    # row 1: existing helper TypeRef (rewired in 3b)
    ...,
    # row 2: System.Object in System.Runtime
    struct.pack(
        "<HHH",
        6,                                  # AssemblyRef row 1
        strings.add("Object"),
        strings.add("System"),
    ),
],
```

`Extends` for the user TypeDef is then a TypeDefOrRef coded index;
TypeRef = tag 1.  Pointing at TypeRef row 2: `(2 << 2) | 1 = 9`.

### Valid-mask update

The `valid_mask` (the U64 in the tables-stream header at offset 8)
needs the AssemblyRef bit set: `valid_mask |= 1 << 0x23`.

```python
valid_mask = 0
for table in tables:
    valid_mask |= 1 << table   # table 0x23 is now in the dict
```

Already handled by the existing loop — the dict-driven valid mask
"just works" as long as the AssemblyRef table is in `tables`.

## Test fixture (already in this branch)

`code/packages/python/cli-assembly-writer/tests/test_real_dotnet.py`
is the success criterion for this spec.  It:

1. Builds a minimal "return 42" program via the writer.
2. Drops it next to a `runtimeconfig.json` targeting `net9.0`.
3. Runs `dotnet <name>.exe` as a subprocess.
4. Asserts the exit code is 42.

Currently fails with `BadImageFormatException` (exit 134); each
chunk above moves the failure past one more loader stage.  Chunk 3
should make it pass.

The fixture is gated behind a `_has_dotnet()` probe so CI without
the SDK skips cleanly — same pattern the repo uses for
git/curl/java-dependent tests.

## Out of scope for this spec

- **Full ECMA-335 conformance.**  This spec targets the minimum to
  load and execute a return-only program.  Generics, custom
  attributes, embedded resources, debug symbols (PDB), strong
  names, and PEHeader2 fields are explicitly future work.
- **netfx / mono targets.**  Reference is net9.0.  The same
  AssemblyRef + Module fixes apply to `mscorlib` for netfx but
  we don't test against netfx here.
- **JVM01.**  The recursion limitation in `ir-to-jvm-class-file`
  is its own track (xfail-marked test in `twig-jvm-compiler`).
  Same class of "real-runtime correctness" problem; different
  fix in a different package.

## Risk register

- **Pseudo-token mismatch.**  The simulator may have its own
  conventions about which token values mean what; if the
  AssemblyRef token (0x23000001) collides with a simulator-side
  reserved value, simulator tests may regress.  Mitigation: run
  *all* existing CLR tests after each chunk, not just the new
  fixture.
- **Method body verification.**  Real .NET runs IL verification
  on method bodies.  Even with metadata fixed, bytecodes the
  simulator currently emits may fail verification (e.g. wrong
  stack delta).  Diagnostic: when chunks 1–3 land and dotnet
  still errors, the next message should be method-body specific
  ("Common Language Runtime detected an invalid program") and we
  add chunks for verifier conformance.
- **net9.0 assumption.**  Some CI environments may have older
  dotnet.  The `runtimeconfig.json` targets `net9.0`; for older
  runtimes we'd need a different value or a probe-and-pick.
