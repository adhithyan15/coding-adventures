# LANG45 — `code-packager`: ELF64 & PE/COFF Object Emitters

**Status:** Draft — 2026-05-14

> Third of four specs in the x86-64 port (LANG44 encoder → LANG43
> backend → **LANG45** packager → LANG46 twig-aot driver).  This one
> teaches `code-packager` to emit *relocatable object files* (`.o`
> for Linux, `.obj` for Windows) so the system linker on each OS
> can produce a runnable executable.

## Motivation

The AArch64 / macOS pipeline (LANG41) delegates the final link step
to Apple's `ld` because the kernel's provenance check kills binaries
written by anyone else.  That works because `code-packager` has a
**Mach-O object-file writer** (`macho_object.rs`) that emits
`MH_OBJECT` with external relocations.

Linux and Windows do not have provenance checks, but they have their
own reasons to prefer the "emit an object file, hand it to the
system linker" pattern:

1. **Linker handles libc.**  `__twig_print_i64` calls `printf`, which
   on Linux lives in `libc.so.6` and on Windows in
   `ucrtbase.dll` / `msvcrt.dll`.  Generating a runnable executable
   that finds and calls these dynamically is hundreds of lines of
   ELF dynamic-section / PE import-directory glue per OS.  The
   system linker does it for free.
2. **Static analysis tools cope.**  `readelf`, `objdump`, `dumpbin`
   all understand standard object files.  Our hand-rolled
   `MH_EXECUTE` / `elf64.rs` minimal-executable writers don't
   produce something a sanitizer or stripper expects.
3. **Future debug info.**  DWARF (Linux/macOS) and CodeView (Windows)
   are easier to attach to an object file that the linker then
   merges, vs. a hand-built executable.

This spec defines two new modules in `code-packager`:

- `code/packages/rust/code-packager/src/elf_object.rs` — ELF64
  `ET_REL` writer with `R_X86_64_*` relocations.
- `code/packages/rust/code-packager/src/pe_object.rs` — PE/COFF
  `IMAGE_FILE_HEADER` (no MZ stub — that's executable-only) writer
  with `IMAGE_REL_AMD64_*` relocations.

They mirror `macho_object.rs`'s API surface but target their
respective formats.

## Non-goals

- **Executable formats** are already done (`elf64.rs`, `macho64.rs`,
  `pe.rs`).  This spec is only about *relocatable object* files.
- **Static libraries** (`.a` / `.lib`) — out of scope; we always
  hand off a single `.o` / `.obj` to the linker, which handles
  archive search.
- **Cross-OS object reloc compatibility** — an ELF object only goes
  to a Linux linker, a COFF object only goes to a Windows linker.
  No magic to make a single object file work on both.
- **DWARF / CodeView debug info** — explicitly deferred.
- **Section attributes for `.eh_frame` / `.pdata` / `.xdata`** —
  LANG43 V1 doesn't emit unwind data, so we don't need these
  sections.
- **TLS, COMDAT, weak symbols** — none of the V1 use cases need
  them.

## API surface

Mirror of `macho_object::pack_object_with_globals_and_externals`:

```rust
// elf_object.rs
pub fn pack_elf64_object_with_globals_and_externals(
    text: &[u8],
    entry_off: usize,
    n_global_slots: u32,
    global_relocs: &[GlobalByteReloc],
    extern_relocs: &[ExternBranchReloc],
    target: &Target,
) -> Result<Vec<u8>, PackagerError>;

// pe_object.rs
pub fn pack_pe_object_with_globals_and_externals(
    text: &[u8],
    entry_off: usize,
    n_global_slots: u32,
    global_relocs: &[GlobalByteReloc],
    extern_relocs: &[ExternBranchReloc],
    target: &Target,
) -> Result<Vec<u8>, PackagerError>;
```

Both consume the same `GlobalByteReloc` / `ExternBranchReloc`
records `macho_object` uses today — no churn for the backend layer.

### `Target` matching

`twig-aot` / the registry dispatch on `Target` fields:

| `target.arch` | `target.os` | `target.binary_format` | Module |
|---|---|---|---|
| `x86_64` | `linux`  | `elf_object` | `elf_object` |
| `x86_64` | `windows` | `pe_object`  | `pe_object`  |
| `aarch64` | `macos`  | `macho_object` | `macho_object` (existing) |
| `aarch64` | `linux`  | `elf_object` | `elf_object` (free with this spec) |

The third row drops out for free: once `elf_object.rs` exists, the
existing ARM64 backend's Mach-O object can be re-pointed at ELF for
Linux ARM64 with no new lowering code — handy for CI.

## ELF64 object layout

```text
Offset             │ Size │ Content
───────────────────┼──────┼───────────────────────────────────────────────
              0    │  64  │ ELF64 header (e_type=ET_REL, e_machine=EM_X86_64)
             64    │  N₁  │ .text section bytes (= text)
       64 + N₁     │  N₂  │ .data section bytes (= 8 * n_global_slots zeros)
                   │      │ Section alignment to 8
       (aligned)   │  N₃  │ .rela.text relocation table (8 × num_text_relocs entries)
       (aligned)   │  N₄  │ .rela.data relocation table (8 × num_data_relocs entries)
       (aligned)   │  N₅  │ .symtab (24 × num_symbols entries)
       (aligned)   │  N₆  │ .strtab string table
       (aligned)   │  N₇  │ .shstrtab section name string table
       (aligned)   │  N₈  │ Section header table (64 × num_sections entries)
```

### Section table

Required sections (all 8-byte aligned in file):

| Index | Name | Type | Flags | Notes |
|---|---|---|---|---|
| 0 | (null) | `SHT_NULL` | 0 | ELF mandatory |
| 1 | `.text` | `SHT_PROGBITS` | `SHF_ALLOC \| SHF_EXECINSTR` | The code |
| 2 | `.data` | `SHT_PROGBITS` | `SHF_ALLOC \| SHF_WRITE` | Globals zero-initialised; could be `.bss` but we use `.data` for parity with macho_object |
| 3 | `.rela.text` | `SHT_RELA` | `SHF_INFO_LINK` | `sh_link = symtab`, `sh_info = .text` |
| 4 | `.rela.data` | `SHT_RELA` | `SHF_INFO_LINK` | Same |
| 5 | `.symtab` | `SHT_SYMTAB` | 0 | `sh_link = .strtab` |
| 6 | `.strtab` | `SHT_STRTAB` | 0 | Symbol names |
| 7 | `.shstrtab` | `SHT_STRTAB` | 0 | Section names |

### Relocation type IDs (`R_X86_64_*`)

| Abstract (LANG44) | ELF type | Hex |
|---|---|---|
| `PltRel32` | `R_X86_64_PLT32` | 4 |
| `PcRel32` | `R_X86_64_PC32` | 2 |
| `GotPcRel32` | `R_X86_64_REX_GOTPCRELX` | 42 |
| (data section absolute) | `R_X86_64_64` | 1 |

Encoded in the `Elf64_Rela` record:

```c
struct Elf64_Rela {
    uint64_t r_offset;       // section-relative byte offset of the field
    uint64_t r_info;         // (symbol_index << 32) | type_id
    int64_t  r_addend;       // -4 for CALL rel32 / Jcc rel32; -4 also for LEA RIP-rel
};
```

The `-4` addend is mandatory: x86-64 PC-relative displacements are
relative to the *end* of the instruction (which is 4 bytes past the
slot the encoder writes).

### Symbol table

Standard `Elf64_Sym`.  V1 emits:

- One `STT_FUNC` symbol per Twig function (binding = `STB_GLOBAL`).
- One `STT_OBJECT` symbol named `_twig_globals` for the data section
  (binding = `STB_GLOBAL`).
- One `STT_NOTYPE` undefined external symbol per distinct
  `extern_relocs` symbol name (binding = `STB_GLOBAL`, `st_shndx =
  SHN_UNDEF`).

`STT_FILE` and section symbols are optional and skipped in V1.

## PE/COFF object layout

```text
Offset             │ Size │ Content
───────────────────┼──────┼───────────────────────────────────────────────
              0    │  20  │ IMAGE_FILE_HEADER
             20    │  40  │ IMAGE_SECTION_HEADER for .text
             60    │  40  │ IMAGE_SECTION_HEADER for .data
            100    │  N₁  │ .text raw bytes (= text)
       100 + N₁    │  N₂  │ .data raw bytes (= 8 * n_global_slots zeros)
       (offset)    │ 10×R₁│ IMAGE_RELOCATION entries for .text
       (offset)    │ 10×R₂│ IMAGE_RELOCATION entries for .data
       (offset)    │ 18×S │ IMAGE_SYMBOL entries
       (offset)    │  ?   │ COFF string table
```

PE object files have **no MZ/DOS stub** (that's executables only).

### `IMAGE_FILE_HEADER` (20 bytes)

```text
Machine               │ 0x8664 = IMAGE_FILE_MACHINE_AMD64
NumberOfSections      │ 2 (.text + .data)
TimeDateStamp         │ 0 (reproducible builds)
PointerToSymbolTable  │ offset to symbol table
NumberOfSymbols       │ count
SizeOfOptionalHeader  │ 0 (object files have no optional header)
Characteristics       │ 0
```

### Section header (40 bytes each)

```text
Name                 │ ".text\0\0\0" / ".data\0\0\0" (8-byte NUL-padded)
VirtualSize          │ 0 (objects use raw size only)
VirtualAddress       │ 0
SizeOfRawData        │ N₁ / N₂
PointerToRawData     │ file offset of section data
PointerToRelocations │ file offset of reloc records
PointerToLinenumbers │ 0
NumberOfRelocations  │ R₁ / R₂
NumberOfLinenumbers  │ 0
Characteristics      │ .text: IMAGE_SCN_CNT_CODE | IMAGE_SCN_MEM_EXECUTE | IMAGE_SCN_MEM_READ | IMAGE_SCN_ALIGN_16BYTES
                     │ .data: IMAGE_SCN_CNT_INITIALIZED_DATA | IMAGE_SCN_MEM_READ | IMAGE_SCN_MEM_WRITE | IMAGE_SCN_ALIGN_8BYTES
```

### Relocation type IDs (`IMAGE_REL_AMD64_*`)

| Abstract (LANG44) | PE type | Hex |
|---|---|---|
| `PltRel32` | `IMAGE_REL_AMD64_REL32` | 0x04 |
| `PcRel32` | `IMAGE_REL_AMD64_REL32` | 0x04 |
| `GotPcRel32` | (collapses to) `IMAGE_REL_AMD64_REL32` | 0x04 |
| (data section absolute) | `IMAGE_REL_AMD64_ADDR64` | 0x01 |

Encoded in `IMAGE_RELOCATION` (10 bytes per record, unpadded):

```c
struct IMAGE_RELOCATION {
    uint32_t VirtualAddress;       // section-relative byte offset
    uint32_t SymbolTableIndex;     // 0-based index into symbol table
    uint16_t Type;
};
```

Note: PE/COFF `REL32` has the addend stored **inside the
instruction's displacement bytes**, not in a separate field.  The
encoder already writes the value with the `-4` adjustment baked in
(see LANG44 `call_rel32` worked example), so the packager just
emits the reloc record pointing at the slot.

### Symbol table

`IMAGE_SYMBOL` (18 bytes per entry):

```text
ShortName / Name    │ 8 bytes: literal if ≤ 8 chars; else 4 zeros + 4-byte string table offset
Value               │ for static symbols: byte offset within section
SectionNumber       │ 1-based section index; 0 = undefined external
Type                │ 0x20 = function, 0 = not function
StorageClass        │ 2 = external (global)
NumberOfAuxSymbols  │ 0
```

V1 emits the same logical set as ELF: one external function symbol
per Twig function, one external object symbol for `_twig_globals`,
one undefined external per `extern_relocs` symbol name.

Note: Windows expects symbol names to have a **leading underscore**
in some toolchains but not in MSVC link.exe for 64-bit (where MS
already drops the underscore convention).  V1 emits names verbatim —
`__twig_print_i64` stays `__twig_print_i64`.

## Linker invocation reference

`code-packager` does not invoke the linker — that's `twig-aot`'s job
(LANG46).  This section documents the expected invocation for
reference.

### Linux (System V AMD64 ABI)

```
cc -o <out> <input>.o <runtime>.a -lc -lm
```

`cc` is `gcc` or `clang`, whichever is on PATH.  Falls back to `ld`
directly if `cc` is missing:

```
ld -dynamic-linker /lib64/ld-linux-x86-64.so.2 \
   /usr/lib/x86_64-linux-gnu/Scrt1.o \
   /usr/lib/x86_64-linux-gnu/crti.o \
   <input>.o <runtime>.a -lc \
   /usr/lib/x86_64-linux-gnu/crtn.o \
   -o <out>
```

Going through `cc` is dramatically simpler — preferred.

### Windows (Microsoft x64 ABI)

MSVC `link.exe`:

```
link /OUT:<out>.exe /ENTRY:main /SUBSYSTEM:CONSOLE \
     <input>.obj <runtime>.lib \
     libcmt.lib  ;  legacy_stdio_definitions.lib
```

LLVM's `lld-link.exe`:

```
lld-link /OUT:<out>.exe /ENTRY:main /SUBSYSTEM:CONSOLE \
         <input>.obj <runtime>.lib \
         libcmt.lib  legacy_stdio_definitions.lib
```

MinGW / GCC on Windows (alternative):

```
gcc -o <out>.exe <input>.o <runtime>.a
```

LANG46 picks among these at runtime.

## Test plan

Coverage target ≥ 95%.

### ELF tests

1. **Magic bytes** — output starts with `7F 45 4C 46` (`\x7fELF`).
2. **`ET_REL`** — `e_type = 1` (relocatable file, not executable).
3. **`EM_X86_64`** — `e_machine = 62`.
4. **Section table well-formed** — `readelf -SW <out>.o` lists
   `.text`, `.data`, `.symtab`, `.strtab`, `.shstrtab`, `.rela.text`.
5. **Reloc records decode** — for a `CALL __twig_print_i64`, the
   `.rela.text` entry has `r_offset = call site +1`, `r_info` type
   `R_X86_64_PLT32`, `r_addend = -4`.
6. **End-to-end link** — write a tiny `factorial` object, link with
   `cc factorial.o runtime.a -o factorial`, exec, assert exit
   code = `factorial(5) & 0xFF` = 120.
7. **`objdump -d`** disassembles the `.text` section back to the
   expected mnemonic sequence.

### PE/COFF tests

1. **Machine = 0x8664** — first two bytes of the header.
2. **No MZ stub** — third byte is `NumberOfSections` low, not `M`.
3. **Section count = 2** — `.text` + `.data`.
4. **Reloc records decode** — for `CALL __twig_print_i64`, the
   `IMAGE_RELOCATION` has correct `VirtualAddress`, symbol index,
   type `IMAGE_REL_AMD64_REL32`.
5. **`dumpbin /HEADERS factorial.obj`** lists the sections, symbols,
   and relocs we expect.
6. **End-to-end link** — link with `lld-link`, run on Windows
   (CI matrix has `windows-latest`), assert exit code = 120.
7. **String table layout** — symbol names > 8 chars use the
   `0 + offset` encoding; ≤ 8 chars use the literal form.

### Cross-format sanity

8. **Same Twig program → both formats** — compile `factorial.twig`
   once via the AArch64-on-Linux path (ELF object), once via the
   x86_64-on-Windows path (PE object).  Assert both produce a
   working binary on their respective CI runners.

## Out of scope (deferred follow-ups)

- macOS x86-64 Mach-O object format — `macho_object.rs` already
  exists for ARM64; teaching it to emit `CPU_TYPE_X86_64` headers
  is a follow-up (small change; gated on demand from Intel-Mac
  contributors).
- Symbol visibility / weak symbols.
- DWARF (Linux/macOS) and CodeView (Windows) debug info.
- `.eh_frame` (Linux unwind) and `.pdata` / `.xdata` (Windows
  unwind).
- COMDAT / one-definition-rule sections.
- Linker scripts.
- LTO bitcode.

## Risk register

| Risk | Mitigation |
|---|---|
| ELF reloc addend semantics confused with PE in-instruction addend | Encoder bakes the `-4` into the displacement field for both targets; ELF re-asserts via `r_addend = -4`; PE packager omits the field entirely. Test 5 in each suite locks the contract. |
| `.symtab` ordering (local symbols must precede globals; `sh_info` = first non-local index) | Lint pass in the packager validates ordering; one negative test confirms an unsorted symtab is rejected before write-out. |
| Windows symbol name underscoring | V1 emits names verbatim; documented in this spec. Adopt prefix policy if/when MinGW users hit it. |
| Section alignment / offset math drift between header and actual data | Single `aligned_offset()` helper used everywhere; assertion at end of packing confirms each `PointerToRawData` matches the section's actual file position. |
