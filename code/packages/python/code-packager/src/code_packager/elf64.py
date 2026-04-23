"""Elf64Packager: produces a minimal ELF64 executable for Linux.

ELF (Executable and Linkable Format) is the binary format used by Linux,
FreeBSD, and most POSIX systems.  The structure produced here is the minimal
executable ELF that the Linux kernel will load and run:

Memory layout of the produced binary::

    ┌─────────────────────────────────────────────────────────────┐
    │ ELF header (64 bytes)                                       │
    │   e_ident[0..3]  = 0x7F 'E' 'L' 'F'  (magic)              │
    │   e_ident[4]     = 2   (ELFCLASS64)                         │
    │   e_ident[5]     = 1   (ELFDATA2LSB — little-endian)        │
    │   e_ident[6]     = 1   (EV_CURRENT — version)              │
    │   e_ident[7]     = 0   (ELFOSABI_NONE — SysV)              │
    │   e_ident[8..15] = 0   (padding)                           │
    │   e_type         = 2   (ET_EXEC — executable)               │
    │   e_machine      = 62  (EM_X86_64) or 183 (EM_AARCH64)     │
    │   e_version      = 1   (EV_CURRENT)                        │
    │   e_entry        = virtual address of entry point           │
    │   e_phoff        = 64  (program header immediately follows) │
    │   e_shoff        = 0   (no section header table)           │
    │   e_flags        = 0                                        │
    │   e_ehsize       = 64  (ELF header size)                   │
    │   e_phentsize    = 56  (program header entry size)         │
    │   e_phnum        = 1   (one PT_LOAD segment)               │
    │   e_shentsize    = 64  (section header size — unused)      │
    │   e_shnum        = 0                                        │
    │   e_shstrndx     = 0                                        │
    ├─────────────────────────────────────────────────────────────┤
    │ Program header (56 bytes)                                   │
    │   p_type    = 1  (PT_LOAD)                                  │
    │   p_flags   = 5  (PF_R | PF_X — read + execute)            │
    │   p_offset  = 0  (segment starts at file offset 0)         │
    │   p_vaddr   = load_address                                  │
    │   p_paddr   = load_address                                  │
    │   p_filesz  = header_size + len(native_bytes)              │
    │   p_memsz   = same                                          │
    │   p_align   = 0x200000 (2 MiB — huge-page friendly)        │
    ├─────────────────────────────────────────────────────────────┤
    │ native_bytes (arbitrary length)                             │
    └─────────────────────────────────────────────────────────────┘

Why only one PT_LOAD?
---------------------
Our backends emit pure code with no mutable global state — all constants are
immediates in the instruction stream.  One executable segment is sufficient.
A second writable segment (for a .data section) would be added if backends
start emitting global variables.

Why e_shoff = 0?
----------------
Linux only needs the program header table to load and execute a binary.
Section headers are for linkers and debuggers.  Omitting them saves space and
simplifies the writer.

Supported targets: ``linux_x64()``, ``linux_arm64()``.
"""

from __future__ import annotations

import struct

from code_packager.artifact import CodeArtifact
from code_packager.errors import UnsupportedTargetError
from code_packager.target import Target

# ELF machine-type constants (e_machine field)
EM_X86_64: int = 62
EM_AARCH64: int = 183

# Default virtual load address for Linux executables.
# The kernel's ASLR is disabled for static binaries at this address.
_DEFAULT_LOAD_ADDRESS: int = 0x400000

# ELF64 header: 64 bytes
# Fields: ident(16s) type(H) machine(H) version(I) entry(Q) phoff(Q)
#         shoff(Q) flags(I) ehsize(H) phentsize(H) phnum(H)
#         shentsize(H) shnum(H) shstrndx(H)
_ELF_HEADER_FMT: str = "<16sHHIQQQIHHHHHH"
_ELF_HEADER_SIZE: int = struct.calcsize(_ELF_HEADER_FMT)  # 64

# ELF64 program header: 56 bytes
# Fields: type(I) flags(I) offset(Q) vaddr(Q) paddr(Q) filesz(Q) memsz(Q) align(Q)
_PROG_HEADER_FMT: str = "<IIQQQQQQ"
_PROG_HEADER_SIZE: int = struct.calcsize(_PROG_HEADER_FMT)  # 56

_EI_NIDENT: int = 16
_PT_LOAD: int = 1
_PF_X: int = 1
_PF_R: int = 4
_ET_EXEC: int = 2
_EV_CURRENT: int = 1


class Elf64Packager:
    """Produce a minimal ELF64 executable for Linux x86-64 or AArch64.

    Accepted targets: :func:`Target.linux_x64`, :func:`Target.linux_arm64`.

    Metadata keys
    -------------
    ``load_address`` (int)
        Override the virtual load address.  Default: ``0x400000``.
    """

    supported_targets: frozenset[Target] = frozenset({
        Target.linux_x64(),
        Target.linux_arm64(),
    })

    def pack(self, artifact: CodeArtifact) -> bytes:
        if artifact.target not in self.supported_targets:
            raise UnsupportedTargetError(artifact.target)

        target = artifact.target
        code = artifact.native_bytes
        load_addr: int = int(artifact.metadata.get("load_address", _DEFAULT_LOAD_ADDRESS))

        e_machine = EM_X86_64 if target.arch == "x86_64" else EM_AARCH64
        header_size = _ELF_HEADER_SIZE + _PROG_HEADER_SIZE
        file_size = header_size + len(code)
        entry_vaddr = load_addr + header_size + artifact.entry_point

        e_ident = bytes([
            0x7F, ord('E'), ord('L'), ord('F'),  # magic
            2,   # ELFCLASS64
            1,   # ELFDATA2LSB
            1,   # EV_CURRENT
            0,   # ELFOSABI_NONE
        ]) + b"\x00" * (_EI_NIDENT - 8)

        elf_header = struct.pack(
            _ELF_HEADER_FMT,
            e_ident,
            _ET_EXEC,          # e_type
            e_machine,         # e_machine
            _EV_CURRENT,       # e_version
            entry_vaddr,       # e_entry
            _ELF_HEADER_SIZE,  # e_phoff (program header follows immediately)
            0,                 # e_shoff (no section headers)
            0,                 # e_flags
            _ELF_HEADER_SIZE,  # e_ehsize
            _PROG_HEADER_SIZE, # e_phentsize
            1,                 # e_phnum
            64,                # e_shentsize (unused but must be valid)
            0,                 # e_shnum
            0,                 # e_shstrndx
        )

        prog_header = struct.pack(
            _PROG_HEADER_FMT,
            _PT_LOAD,          # p_type
            _PF_R | _PF_X,    # p_flags
            0,                 # p_offset (segment starts at file beginning)
            load_addr,         # p_vaddr
            load_addr,         # p_paddr
            file_size,         # p_filesz
            file_size,         # p_memsz
            0x200000,          # p_align (2 MiB — huge-page boundary)
        )

        return elf_header + prog_header + code

    def file_extension(self, target: Target) -> str:  # noqa: ARG002
        return ".elf"
