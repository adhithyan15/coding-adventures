# code-packager

Cross-platform binary packaging for compiled code.  Takes native machine
bytes from any backend and wraps them in the OS-specific container format
the target platform expects — ELF64 (Linux), Mach-O 64 (macOS), PE32+
(Windows), WASM, Intel HEX, or raw binary.

Cross-compilation is first-class: a Mac can produce a Windows `.exe`, a
Linux CI machine can produce a macOS Mach-O binary, and a Pi can produce a
WASM module.  All format writers are pure Python — no OS-specific syscalls.

## Where it fits in the stack

```
Source (.tetrad / .nib / …)
        │
        ▼
Lexer → Parser → Type Checker
        │
        ▼
   InterpreterIR
        │
    aot-core
    (infer → specialise → optimize → backend.compile → link)
        │
        ▼
   CodeArtifact  ←── code-packager picks up here
        │
   PackagerRegistry.pack(artifact)
        │
   ┌────┴────────────────┐
   ▼                     ▼
ELF64 / Mach-O64    PE32+ / WASM
(Linux / macOS)    (Windows / browser)
```

`code-packager` is the *only* layer that knows about OS binary formats.
`aot-core` and all backends remain format-agnostic.

## Quick start

```python
from code_packager import CodeArtifact, PackagerRegistry, Target

# Bytes compiled by a backend for Linux x86-64
code = b"\x48\x31\xc0\xc3"           # xor rax, rax; ret
artifact = CodeArtifact(
    native_bytes=code,
    entry_point=0,
    target=Target.linux_x64(),
)

registry = PackagerRegistry.default()
elf_bytes = registry.pack(artifact)   # valid ELF64 executable

# Cross-compile the same code for Windows (from any host OS)
win_artifact = CodeArtifact(
    native_bytes=code,
    entry_point=0,
    target=Target.windows_x64(),
)
exe_bytes = registry.pack(win_artifact)  # valid PE32+ .exe
```

## Supported targets

| Factory method | arch | os | binary_format | ext |
|---------------|------|----|---------------|-----|
| `Target.linux_x64()` | x86_64 | linux | elf64 | `.elf` |
| `Target.linux_arm64()` | arm64 | linux | elf64 | `.elf` |
| `Target.macos_x64()` | x86_64 | macos | macho64 | `.macho` |
| `Target.macos_arm64()` | arm64 | macos | macho64 | `.macho` |
| `Target.windows_x64()` | x86_64 | windows | pe | `.exe` |
| `Target.wasm()` | wasm32 | none | wasm | `.wasm` |
| `Target.intel_4004()` | i4004 | none | intel_hex | `.hex` |
| `Target.intel_8008()` | i8008 | none | intel_hex | `.hex` |
| `Target.raw(arch?)` | any | none | raw | `.bin` |

## API reference

### `Target`

Immutable, hashable triple describing the compilation target.  All three
fields are plain strings so new targets can be added without API changes.

```python
Target(arch="x86_64", os="linux", binary_format="elf64")
str(Target.linux_x64())  # "x86_64-linux-elf64"
```

### `CodeArtifact`

Handoff object between a backend and the packager.

```python
CodeArtifact(
    native_bytes=code,          # raw machine code
    entry_point=0,              # byte offset of entry function
    target=Target.linux_x64(),
    symbol_table={"main": 0},   # optional: name → byte offset
    metadata={"subsystem": 3},  # optional: packager-specific hints
)
```

### `PackagerRegistry`

```python
registry = PackagerRegistry.default()  # pre-populated with all built-ins
binary = registry.pack(artifact)       # auto-selects packager by target
packager = registry.get(target)        # get packager directly
registry.register(my_packager)         # add custom packager
```

### Metadata keys

| Key | Packager | Type | Default | Description |
|-----|----------|------|---------|-------------|
| `load_address` | ELF64, Mach-O64 | int | `0x400000` / `0x100000000` | Virtual load address |
| `subsystem` | PE | int | 3 | 2=GUI, 3=console |
| `image_base` | PE | int | `0x140000000` | Image base |
| `origin` | Intel HEX | int | 0 | ROM load address |
| `exports` | WASM | list[str] | `["main"]` | Export names |

### Exceptions

- `PackagerError` — base class
- `UnsupportedTargetError` — no packager handles `artifact.target`
- `ArtifactTooLargeError` — binary format's size limit exceeded
- `MissingMetadataError` — required metadata key absent

## Writing a custom packager

Any object with `supported_targets`, `pack()`, and `file_extension()`
satisfies `PackagerProtocol`:

```python
class SrecPackager:
    supported_targets = frozenset({Target.raw(arch="arm64")})

    def pack(self, artifact: CodeArtifact) -> bytes:
        # Produce Motorola SREC format
        ...

    def file_extension(self, target: Target) -> str:
        return ".srec"

registry = PackagerRegistry.default()
registry.register(SrecPackager())
```

## Installation

```bash
pip install coding-adventures-code-packager
```

Requires `coding-adventures-intel-4004-packager` and
`coding-adventures-wasm-module-encoder`.
