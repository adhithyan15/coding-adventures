"""MachO64Packager: produces a minimal Mach-O 64-bit executable for macOS.

Mach-O (Mach Object) is the binary format used by macOS and iOS.  This
packager produces the minimum structure the macOS kernel loader requires to
map and start a program:

Memory layout of the produced binary::

    ┌─────────────────────────────────────────────────────────────┐
    │ Mach-O header (32 bytes)                                    │
    │   magic:      0xFEEDFACF  (MH_MAGIC_64, little-endian)     │
    │   cputype:    0x01000007  (CPU_TYPE_X86_64)                 │
    │            or 0x0100000C  (CPU_TYPE_ARM64)                  │
    │   cpusubtype: 3           (CPU_SUBTYPE_ALL)                 │
    │   filetype:   2           (MH_EXECUTE)                      │
    │   ncmds:      2           (two load commands)               │
    │   sizeofcmds: computed    (total bytes of load commands)    │
    │   flags:      0x00000001  (MH_NOUNDEFS)                     │
    │   reserved:   0                                             │
    ├─────────────────────────────────────────────────────────────┤
    │ LC_SEGMENT_64 load command                                  │
    │   (maps entire file as __TEXT segment, execute+read)        │
    │   Includes one __text section header                        │
    ├─────────────────────────────────────────────────────────────┤
    │ LC_UNIXTHREAD load command                                  │
    │   (sets CPU state: RIP/PC to entry point virtual address)   │
    │   Used instead of LC_MAIN to avoid dyld dependency          │
    ├─────────────────────────────────────────────────────────────┤
    │ native_bytes (arbitrary length)                             │
    └─────────────────────────────────────────────────────────────┘

LC_UNIXTHREAD vs LC_MAIN
------------------------
``LC_MAIN`` (introduced macOS 10.8) requires the dynamic linker (dyld) to
set up the C runtime before jumping to main.  For standalone binaries with no
dynamic libraries — exactly what our backends produce — this adds an
unnecessary dependency.  ``LC_UNIXTHREAD`` directly sets the CPU register
state (RIP for x86-64, PC for ARM64) and works without dyld.

Load address convention
-----------------------
macOS ARM64 executables conventionally load at ``0x100000000`` (4 GiB).
This avoids the low 4 GiB which is reserved on Apple Silicon for the kernel.
x86-64 macOS executables also use ``0x100000000`` conventionally.

Supported targets: ``macos_x64()``, ``macos_arm64()``.
"""

from __future__ import annotations

import struct

from code_packager.artifact import CodeArtifact
from code_packager.errors import UnsupportedTargetError
from code_packager.target import Target

# Mach-O magic for 64-bit little-endian
MH_MAGIC_64: int = 0xFEEDFACF

# CPU type constants
CPU_TYPE_X86_64: int = 0x01000007
CPU_TYPE_ARM64: int = 0x0100000C
CPU_SUBTYPE_ALL: int = 3

# File type
MH_EXECUTE: int = 2

# Flags
MH_NOUNDEFS: int = 0x1

# Load command types
LC_SEGMENT_64: int = 0x19
LC_UNIXTHREAD: int = 0x5

# Segment / section protection flags
VM_PROT_READ: int = 0x1
VM_PROT_EXECUTE: int = 0x4

# Default virtual load address (Apple convention for 64-bit executables)
_DEFAULT_LOAD_ADDRESS: int = 0x100000000

# Mach-O header: 32 bytes
# magic(I) cputype(i) cpusubtype(i) filetype(I) ncmds(I) sizeofcmds(I) flags(I) reserved(I)
_MACHO_HEADER_FMT: str = "<IiiIIIII"
_MACHO_HEADER_SIZE: int = struct.calcsize(_MACHO_HEADER_FMT)  # 32

# LC_SEGMENT_64: 72 bytes for the command itself + 80 bytes per section
# segname(16s) vmaddr(Q) vmsize(Q) fileoff(Q) filesize(Q)
# maxprot(i) initprot(i) nsects(I) flags(I)
_SEG64_BODY_FMT: str = "<16sQQQQiiII"
_SEG64_CMD_SIZE: int = 8 + struct.calcsize(_SEG64_BODY_FMT)  # 72

# Section header for __text inside __TEXT: 80 bytes
# sectname(16s) segname(16s) addr(Q) size(Q) offset(I) align(I)
# reloff(I) nreloc(I) flags(I) reserved1(I) reserved2(I) reserved3(I)
_SECT64_FMT: str = "<16s16sQQIIIIIIII"
_SECT64_SIZE: int = struct.calcsize(_SECT64_FMT)  # 80

_SEGMENT_CMD_TOTAL: int = _SEG64_CMD_SIZE + _SECT64_SIZE  # 152

# LC_UNIXTHREAD for x86-64: cmd(I) cmdsize(I) flavor(I) count(I) + 168-byte thread state
# x86_THREAD_STATE64 flavor = 4, count = 42 (longs), 42 × 4 = 168 bytes
# We only set RIP (register index 16 in the state, zero-indexed).
_UNIXTHREAD_X86_64_FLAVOR: int = 4
_UNIXTHREAD_X86_64_COUNT: int = 42   # number of uint32_t words in the state
_UNIXTHREAD_X86_64_SIZE: int = 8 + 8 + _UNIXTHREAD_X86_64_COUNT * 4  # 184

# LC_UNIXTHREAD for ARM64: ARM_THREAD_STATE64 flavor = 6, count = 68
# 68 × 4 = 272 bytes of state
_UNIXTHREAD_ARM64_FLAVOR: int = 6
_UNIXTHREAD_ARM64_COUNT: int = 68
_UNIXTHREAD_ARM64_SIZE: int = 8 + 8 + _UNIXTHREAD_ARM64_COUNT * 4  # 288


class MachO64Packager:
    """Produce a minimal Mach-O 64-bit executable for macOS.

    Accepted targets: :func:`Target.macos_x64`, :func:`Target.macos_arm64`.

    Metadata keys
    -------------
    ``load_address`` (int)
        Override the virtual load address.  Default: ``0x100000000``.
    """

    supported_targets: frozenset[Target] = frozenset({
        Target.macos_x64(),
        Target.macos_arm64(),
    })

    def pack(self, artifact: CodeArtifact) -> bytes:
        if artifact.target not in self.supported_targets:
            raise UnsupportedTargetError(artifact.target)

        target = artifact.target
        code = artifact.native_bytes
        load_addr: int = int(
            artifact.metadata.get("load_address", _DEFAULT_LOAD_ADDRESS)
        )
        is_x86 = target.arch == "x86_64"
        cputype = CPU_TYPE_X86_64 if is_x86 else CPU_TYPE_ARM64

        thread_cmd = self._make_unixthread(
            is_x86=is_x86,
            entry_vaddr=load_addr + self._header_size(is_x86) + artifact.entry_point,
        )

        total_header = self._header_size(is_x86)
        file_size = total_header + len(code)
        sizeofcmds = _SEGMENT_CMD_TOTAL + len(thread_cmd)

        mach_header = struct.pack(
            _MACHO_HEADER_FMT,
            MH_MAGIC_64,
            cputype,
            CPU_SUBTYPE_ALL,
            MH_EXECUTE,
            2,                # ncmds: LC_SEGMENT_64 + LC_UNIXTHREAD
            sizeofcmds,
            MH_NOUNDEFS,
            0,                # reserved
        )

        segment_cmd = self._make_segment(load_addr, file_size, total_header, code)
        return mach_header + segment_cmd + thread_cmd + code

    def _header_size(self, is_x86: bool) -> int:
        thread_size = _UNIXTHREAD_X86_64_SIZE if is_x86 else _UNIXTHREAD_ARM64_SIZE
        return _MACHO_HEADER_SIZE + _SEGMENT_CMD_TOTAL + thread_size

    def _make_segment(
        self,
        load_addr: int,
        file_size: int,
        header_size: int,
        code: bytes,
    ) -> bytes:
        code_vaddr = load_addr + header_size
        prot = VM_PROT_READ | VM_PROT_EXECUTE

        seg_body = struct.pack(
            _SEG64_BODY_FMT,
            b"__TEXT\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",  # segname (16 bytes)
            load_addr,    # vmaddr
            file_size,    # vmsize
            0,            # fileoff
            file_size,    # filesize
            prot,         # maxprot
            prot,         # initprot
            1,            # nsects
            0,            # flags
        )

        sect = struct.pack(
            _SECT64_FMT,
            b"__text\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",  # sectname (16)
            b"__TEXT\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00",  # segname (16)
            code_vaddr,         # addr
            len(code),          # size
            header_size,        # offset (file offset to code)
            2,                  # align (2^2 = 4-byte aligned)
            0,                  # reloff
            0,                  # nreloc
            0x80000400,         # flags (S_ATTR_PURE_INSTRUCTIONS | S_ATTR_SOME_INSTRUCTIONS)
            0,                  # reserved1
            0,                  # reserved2
            0,                  # reserved3
        )

        cmd_header = struct.pack("<II", LC_SEGMENT_64, _SEGMENT_CMD_TOTAL)
        return cmd_header + seg_body + sect

    def _make_unixthread(self, is_x86: bool, entry_vaddr: int) -> bytes:
        if is_x86:
            # x86_THREAD_STATE64: 42 uint32_t values = 168 bytes
            # Registers: rax rcx rdx rbx rsp rbp rsi rdi  (8 × 64-bit = 16 × u32)
            #            r8 r9 r10 r11 r12 r13 r14 r15   (8 × 64-bit = 16 × u32)
            #            rip rflags cs fs gs               (5 × 64-bit = 10 × u32)
            # RIP is at word index 32 (little-endian 64-bit split into two 32-bit halves).
            # We build the state as raw bytes: 168 zero bytes, then write entry_vaddr at RIP.
            state = bytearray(168)
            # RIP offset: 16 GPRs before rip = 16×8 = 128 bytes into state
            struct.pack_into("<Q", state, 128, entry_vaddr)
            flavor, count, size = (
                _UNIXTHREAD_X86_64_FLAVOR,
                _UNIXTHREAD_X86_64_COUNT,
                _UNIXTHREAD_X86_64_SIZE,
            )
        else:
            # ARM_THREAD_STATE64: 68 uint32_t values = 272 bytes
            # Layout: x0..x28 (29 × 64-bit), fp, lr, sp, pc, cpsr+pad
            # pc is at offset 31×8 = 248 bytes
            state = bytearray(272)
            struct.pack_into("<Q", state, 248, entry_vaddr)  # pc
            flavor, count, size = (
                _UNIXTHREAD_ARM64_FLAVOR,
                _UNIXTHREAD_ARM64_COUNT,
                _UNIXTHREAD_ARM64_SIZE,
            )

        cmd_header = struct.pack("<IIII", LC_UNIXTHREAD, size, flavor, count)
        return cmd_header + bytes(state)

    def file_extension(self, target: Target) -> str:  # noqa: ARG002
        return ".macho"
