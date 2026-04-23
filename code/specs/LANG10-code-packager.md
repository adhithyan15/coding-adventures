# LANG10 — code-packager: Cross-Platform Binary Packaging

## Overview

`code-packager` is the **final stage of the ahead-of-time compilation
pipeline**.  It takes a blob of native machine code produced by any backend
(aot-core, jit-core, or a standalone backend), wraps it in the appropriate
OS-specific binary format, and writes a file that the target operating system
can load and execute.

The key design principle is **host/target independence**: a Mac can produce a
Windows `.exe`, a Linux CI machine can produce a macOS Mach-O binary, and a
Pi can produce a WASM module.  The binary formats (ELF, Mach-O, PE/COFF, WASM)
are just data layouts — any machine can write them.

```
Source (.tetrad / .nib / …)
        │
        ▼
Lexer → Parser → Type Checker
        │
        ▼
   InterpreterIR (IIRModule)
        │
    ┌───┴──────────────────────────┐
    │        aot-core              │
    │  infer → specialise          │
    │  → optimize → backend.compile│
    │  → link                      │
    └───────────────┬──────────────┘
                    │  CodeArtifact
                    │  (native_bytes + symbol_table
                    │   + entry_point + target)
                    ▼
             code-packager
                    │
        ┌───────────┼───────────┐
        ▼           ▼           ▼
    ELF64       Mach-O64     PE/COFF
  (Linux)       (macOS)    (Windows)
                    │
                 WASM / Raw
```

`code-packager` is the **only** component in the stack that knows about OS
binary formats.  `aot-core`, `jit-core`, and the backends remain completely
format-agnostic — they deal only in `(native_bytes, symbol_table)`.

---

## Target triple

A **target** describes the machine the binary will run on — not the machine
running the compiler.

```python
@dataclass(frozen=True)
class Target:
    arch: str          # "x86_64" | "arm64" | "wasm32" | "i4004"
    os: str            # "linux" | "macos" | "windows" | "none"
    binary_format: str # "elf64" | "macho64" | "pe" | "wasm" | "raw" | "intel_hex"
```

Factory methods cover the common triples:

| Method | arch | os | binary_format |
|--------|------|----|---------------|
| `linux_x64()` | `x86_64` | `linux` | `elf64` |
| `linux_arm64()` | `arm64` | `linux` | `elf64` |
| `macos_x64()` | `x86_64` | `macos` | `macho64` |
| `macos_arm64()` | `arm64` | `macos` | `macho64` |
| `windows_x64()` | `x86_64` | `windows` | `pe` |
| `wasm()` | `wasm32` | `none` | `wasm` |
| `raw(arch)` | `arch` | `none` | `raw` |
| `intel_4004()` | `i4004` | `none` | `intel_hex` |

A `Target` is immutable and equality/hashable so it can be used as a dict key
(e.g. in a registry mapping target → packager).

### Why not use an autoconf-style triple string?

Autoconf strings like `x86_64-unknown-linux-gnu` are convenient as command-line
arguments but opaque to code.  Structured fields let callers branch on
`target.binary_format` without parsing strings, and add new fields (e.g.
`abi: str`, `float_abi: str`) without breaking the existing API.

---

## CodeArtifact

A `CodeArtifact` is the handoff object between a compilation backend and the
packager.

```python
@dataclass
class CodeArtifact:
    native_bytes: bytes
    entry_point: int
    target: Target
    symbol_table: dict[str, int] = field(default_factory=dict)
    metadata: dict[str, object] = field(default_factory=dict)
```

Fields:

- **`native_bytes`** — raw machine code bytes for the target ISA (or WASM binary
  expression bytes if the target is wasm32).  All functions are concatenated in
  link order.
- **`entry_point`** — byte offset within `native_bytes` where execution begins.
  Set to 0 for single-function binaries.  Produced by `aot_core.link.entry_point_offset`.
- **`target`** — the `Target` the bytes were compiled for.
- **`symbol_table`** — maps function name → byte offset within `native_bytes`.
  Enables the packager to populate the binary's symbol/export section.
- **`metadata`** — free-form key/value store for packager hints (e.g.
  `{"stack_size": 65536, "heap_size": 1048576, "wasm_imports": [...]}`).

### Why include `metadata`?

Different binary formats need different supplementary information:

- ELF needs a PT_NOTE segment with build-ID
- PE needs a subsystem flag (GUI vs CONSOLE)
- WASM needs an import declaration list
- Mach-O needs an LC_UUID load command

Rather than hard-coding all these in `CodeArtifact`, they go in `metadata` with
packager-specific keys.  Packagers document which keys they consume and ignore
unknown keys.

---

## PackagerProtocol

A packager is any object that satisfies the `PackagerProtocol`:

```python
from typing import Protocol, runtime_checkable

@runtime_checkable
class PackagerProtocol(Protocol):
    """Pack a CodeArtifact into a platform-specific binary."""

    supported_targets: frozenset[Target]

    def pack(self, artifact: CodeArtifact) -> bytes:
        """
        Wrap artifact.native_bytes in the appropriate binary container.

        Raises ValueError if artifact.target is not in supported_targets.
        """
        ...

    def file_extension(self, target: Target) -> str:
        """Return the conventional file extension for target (e.g. ".exe", ".elf")."""
        ...
```

All packagers are **pure Python** — no native extensions, no OS-specific system
calls.  They are byte-level format writers that can run on any host and produce
output for any target.

---

## Built-in packagers

### `RawPackager`

The simplest packager: writes `native_bytes` verbatim, with no container.
Useful for embedded targets where the firmware loader handles placement.

- **Supported targets**: any `Target(binary_format="raw")`
- **File extension**: `.bin`
- **Metadata keys**: none

### `IntelHexPackager`

Wraps `native_bytes` in the [Intel HEX](https://en.wikipedia.org/wiki/Intel_HEX)
format understood by EPROM programmers and most embedded simulators.

Under the hood delegates to `intel_4004_packager.encode_hex` (which is format-
agnostic despite its name — it produces standard Intel HEX for any binary).

- **Supported targets**: any `Target(binary_format="intel_hex")`
- **File extension**: `.hex`
- **Metadata keys**: `origin` (int, default 0) — load address

### `Elf64Packager`

Produces a minimal but valid **ELF64** executable for Linux x86-64 and ARM64.

ELF is the Executable and Linkable Format used by Linux, FreeBSD, and most
POSIX systems.  A minimal executable ELF has:

```
ELF header (64 bytes)
  e_ident:    magic + class=64-bit + data=little-endian
              + type=ET_EXEC + machine=EM_X86_64 or EM_AARCH64
  e_entry:    virtual address of entry point
  e_phoff:    offset of program header table
  e_phentsize / e_phnum: 56 / 1

Program header table (1 entry × 56 bytes = 56 bytes)
  PT_LOAD segment covers the entire file (header + code)
  Flags: PF_R | PF_X (read + execute)
  vaddr: 0x400000 (conventional Linux load address)
  align: 0x200000

Code section
  native_bytes verbatim
```

Why only one PT_LOAD segment?

A minimal statically-linked binary only needs one segment: executable code.
No data segment, no dynamic linking, no GOT/PLT.  This matches the output of
backends like `ir-to-intel-4004-compiler` that emit pure code with no writable
global state.

- **Supported targets**: `linux_x64()`, `linux_arm64()`
- **File extension**: `.elf`
- **Load address**: `0x400000` (x86-64) or `0x400000` (ARM64) — conventional
  kernel load address; overridable via `metadata["load_address"]`

### `MachO64Packager`

Produces a minimal **Mach-O 64-bit** executable for macOS x86-64 and ARM64.

Mach-O is the binary format used by macOS and iOS.  A minimal executable has:

```
Mach-O header (32 bytes)
  magic:      0xFEEDFACF (64-bit, little-endian)
  cputype:    CPU_TYPE_X86_64 (0x01000007) or CPU_TYPE_ARM64 (0x0100000C)
  cpusubtype: CPU_SUBTYPE_ALL (0x3)
  filetype:   MH_EXECUTE (0x2)
  ncmds / sizeofcmds: computed from load commands
  flags:      MH_NOUNDEFS (0x1)

Load commands
  LC_SEGMENT_64: maps __TEXT segment (header + code) into address space
    vmaddr:   0x100000000 (macOS ARM64 convention)
    vmsize:   aligned to page size
    sections: one __text section

  LC_UNIXTHREAD (x86-64) / LC_MAIN (ARM64):
    sets entry point register (RIP / PC) to entry point address
```

Note on LC_UNIXTHREAD vs LC_MAIN:

`LC_MAIN` (introduced in OS X 10.8 Mountain Lion) requires `libdyld.dylib` to
set up the runtime — unsuitable for standalone binaries with no dynamic
libraries.  `LC_UNIXTHREAD` directly sets the CPU state and works without a
dyld stub.  We use `LC_UNIXTHREAD` for cross-compiled binaries and
`LC_MAIN` when `metadata["use_lc_main"]` is True.

- **Supported targets**: `macos_x64()`, `macos_arm64()`
- **File extension**: `.macho`
- **Load address**: `0x100000000` (ARM64 convention), overridable via
  `metadata["load_address"]`

### `PePackager`

Produces a minimal **Portable Executable (PE32+)** for Windows x86-64.

PE is the binary format used by Windows for executables (`.exe`) and DLLs.
PE32+ is the 64-bit variant.  Structure:

```
DOS stub (64 bytes)
  MZ signature: 0x5A4D
  e_lfanew: offset to PE header (= 64)

PE header signature: "PE\x00\x00"

COFF header (20 bytes)
  Machine:          0x8664 (IMAGE_FILE_MACHINE_AMD64)
  NumberOfSections: 1
  SizeOfOptionalHeader: 240
  Characteristics:  IMAGE_FILE_EXECUTABLE_IMAGE | IMAGE_FILE_LARGE_ADDRESS_AWARE

Optional header PE32+ (240 bytes)
  Magic:            0x020B (PE32+)
  AddressOfEntryPoint: RVA of entry point
  ImageBase:        0x140000000 (ASLR-friendly default)
  SectionAlignment: 0x1000
  FileAlignment:    0x200
  Subsystem:        IMAGE_SUBSYSTEM_WINDOWS_CUI (3) = console app
  SizeOfImage / SizeOfHeaders: computed

Section table (1 entry × 40 bytes)
  .text section: code, execute + read

Section data
  Padding to FileAlignment + native_bytes
```

Note on the minimal DOS stub:

Windows requires the `MZ` header so the loader can find `e_lfanew → PE signature`.
A minimal DOS stub prints "This program cannot be run in DOS mode." if executed
in 16-bit DOS.  We emit the standard 64-byte stub verbatim.

- **Supported targets**: `windows_x64()`
- **File extension**: `.exe`
- **Metadata keys**:
  - `subsystem` (int, default 3 = `CUI`) — 2 = `GUI`, 3 = console
  - `image_base` (int, default `0x140000000`)

### `WasmPackager`

Delegates to `wasm_module_encoder` to wrap `native_bytes` in a valid WASM
module with a proper function section and export.

- **Supported targets**: `wasm()`
- **File extension**: `.wasm`
- **Metadata keys**:
  - `exports` (list[str], default `["main"]`) — function names to export
  - `imports` (list[dict], default `[]`) — WASM import declarations

### `PackagerRegistry`

```python
class PackagerRegistry:
    def register(self, packager: PackagerProtocol) -> None: ...
    def get(self, target: Target) -> PackagerProtocol: ...
    def pack(self, artifact: CodeArtifact) -> bytes: ...
    @classmethod
    def default(cls) -> "PackagerRegistry": ...
```

`default()` returns a registry pre-populated with all built-in packagers.

---

## Integration with aot-core

`aot-core.AOTCore` gains an optional `target` parameter:

```python
class AOTCore:
    def __init__(
        self,
        backend,
        optimization_level: int = 1,
        vm_runtime=None,
        target: Target | None = None,
        packager_registry: PackagerRegistry | None = None,
    ): ...
```

When `target` is provided, `compile()` returns a `CodeArtifact` rather than
raw bytes; the caller passes it to `packager_registry.pack(artifact)` to get
the final binary.

This separation keeps `aot-core` format-agnostic: it knows nothing about ELF
or PE, it just produces `CodeArtifact`.

---

## JIT profiling insights (LANG11 preview)

A closely related idea is having the **JIT compiler** report to the developer
*where* the program is paying for dynamic dispatch and what statically typing
those sites would save.  This is addressed in LANG11.

The data model for profiling insights will be:

```python
@dataclass
class TypeSite:
    function: str
    instruction: str
    observed_type: str     # e.g. "int" (seen 999×) or "any" (mixed)
    inferred_type: str     # what static inference would assign
    dispatch_cost: str     # "none" | "guard" | "generic_call"
    call_count: int
    savings: str           # human-readable: "would eliminate 3 guards per call"
```

`JITCore.profile_report()` returns a `list[TypeSite]` sorted by impact
(highest `call_count × guard_overhead` first).  This plugs into a formatter
that produces developer-readable advice:

```
Hot site: add in loop_body — 12,847 calls
  Observed type: int (100%)
  Current code: type_assert(r0, int) + add_int(r0, r1) [1 guard/call]
  Suggestion: annotate loop counter as Int → eliminate guard
  Estimated speedup: ~15% (1 branch removed from hot path)
```

See LANG11-jit-profiling-insights.md for the full design.

---

## Error handling

- `PackagerError` — base class for all packager errors
- `UnsupportedTargetError(PackagerError)` — raised when no packager handles
  `artifact.target`
- `ArtifactTooLargeError(PackagerError)` — raised when `native_bytes` exceeds
  the binary format's limits (e.g. PE's 4 GiB section limit)
- `MissingMetadataError(PackagerError)` — raised when a required metadata key
  is absent

All packager exceptions carry `target` and `artifact_size` attributes.

---

## Module layout

```
code-packager/
├── pyproject.toml
├── BUILD
├── README.md
├── CHANGELOG.md
└── src/
    └── code_packager/
        ├── __init__.py
        ├── target.py          # Target dataclass + factory methods
        ├── artifact.py        # CodeArtifact dataclass
        ├── protocol.py        # PackagerProtocol + PackagerRegistry
        ├── errors.py          # PackagerError hierarchy
        ├── raw.py             # RawPackager
        ├── intel_hex.py       # IntelHexPackager
        ├── elf64.py           # Elf64Packager
        ├── macho64.py         # MachO64Packager
        ├── pe.py              # PePackager
        └── wasm.py            # WasmPackager
```

---

## Testing strategy

Each packager has its own test module.  Every test module verifies:

1. **Round-trip**: pack a known 4-byte code blob → parse the output with a
   reference parser or struct.unpack, confirm the embedded code matches.
2. **Entry point**: verify the entry point address in the binary header equals
   the requested offset.
3. **Header fields**: spot-check magic bytes, machine type, subsystem, etc.
4. **Metadata overrides**: verify `load_address`, `subsystem`, etc.
5. **Error paths**: wrong target, empty bytes, metadata validation.

The ELF, Mach-O, and PE tests parse the produced binary with `struct.unpack`
— no external parser dependency.  This keeps tests self-contained and ensures
the format is correct by construction, not by accident.

Target: **100% line coverage** across all packager modules.

---

## Design decisions

### Pure-Python format writers

All binary format writers are pure Python `struct.pack` operations.  This
means:

- No platform-specific `ctypes` or OS APIs needed
- The packager runs on any Python 3.11+ interpreter regardless of OS
- Easy to test with deterministic outputs

The formats themselves (ELF, Mach-O, PE) are fully documented open standards;
we implement only the minimal subset needed to produce a runnable executable.

### Minimal section count

Each packager emits a single executable section (`.text` / `__text`).  No
`.data`, `.bss`, or `.rodata` sections are emitted.  This matches the output of
our backends, which embed all constants as immediate values in the code stream.
Future backends that need mutable globals will extend the packager with a data
section.

### No dynamic linking support

The packager produces fully statically-linked executables.  No import tables,
no PLT stubs, no dynamic relocations.  All symbols must be resolved by the
backend before the packager is invoked.  This constraint simplifies the format
writers enormously and is the right default for embedded and systems targets.
