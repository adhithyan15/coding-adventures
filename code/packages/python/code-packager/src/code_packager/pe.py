"""PePackager: produces a minimal PE32+ executable for Windows x86-64.

PE (Portable Executable) is the binary format used by Windows for executables
(.exe), DLLs (.dll), and drivers (.sys).  PE32+ is the 64-bit variant.

The structure produced here is the minimum needed for the Windows loader to
map and execute a standalone, statically-linked program.

Memory layout of the produced binary::

    ┌─────────────────────────────────────────────────────────────┐
    │ DOS stub (64 bytes)                                         │
    │   Bytes 0..1: 0x4D 0x5A  ("MZ" — DOS magic)               │
    │   Bytes 60..63: 0x40 0x00 0x00 0x00  (e_lfanew = 64)       │
    │   The remaining bytes print "This program cannot be run in  │
    │   DOS mode." if executed under 16-bit DOS.                 │
    ├─────────────────────────────────────────────────────────────┤
    │ PE signature (4 bytes): 0x50 0x45 0x00 0x00  ("PE\\0\\0")  │
    ├─────────────────────────────────────────────────────────────┤
    │ COFF header (20 bytes)                                      │
    │   Machine:           0x8664  (IMAGE_FILE_MACHINE_AMD64)     │
    │   NumberOfSections:  1                                      │
    │   TimeDateStamp:     0  (reproducible builds)              │
    │   PointerToSymbolTable: 0                                   │
    │   NumberOfSymbols:   0                                      │
    │   SizeOfOptionalHeader: 240                                 │
    │   Characteristics:  0x0022                                  │
    │     IMAGE_FILE_EXECUTABLE_IMAGE (0x0002)                    │
    │     IMAGE_FILE_LARGE_ADDRESS_AWARE (0x0020)                 │
    ├─────────────────────────────────────────────────────────────┤
    │ Optional header PE32+ (240 bytes)                           │
    │   Magic:             0x020B  (PE32+)                        │
    │   AddressOfEntryPoint: RVA of entry point                   │
    │   ImageBase:         0x140000000  (default ASLR-friendly)   │
    │   SectionAlignment:  0x1000  (4 KiB page)                  │
    │   FileAlignment:     0x200   (512-byte sector)              │
    │   Subsystem:         3  (IMAGE_SUBSYSTEM_WINDOWS_CUI)       │
    │   SizeOfImage / SizeOfHeaders: computed                     │
    ├─────────────────────────────────────────────────────────────┤
    │ Section table (1 entry × 40 bytes)                          │
    │   .text section: code, execute + read                       │
    ├─────────────────────────────────────────────────────────────┤
    │ Padding to FileAlignment (512 bytes)                        │
    ├─────────────────────────────────────────────────────────────┤
    │ native_bytes                                                │
    └─────────────────────────────────────────────────────────────┘

Alignment
---------
PE distinguishes between *file alignment* (blocks on disk, typically 512 bytes)
and *section alignment* (pages in virtual memory, typically 4096 bytes).
Headers are padded to file_alignment, and the .text section's virtual address
is rounded up to section_alignment.

TimeDateStamp = 0
-----------------
Setting the timestamp to zero produces reproducible builds — the same source
always produces byte-identical output regardless of when it is compiled.

Supported targets: ``windows_x64()``.
"""

from __future__ import annotations

import struct

from code_packager.artifact import CodeArtifact
from code_packager.errors import UnsupportedTargetError
from code_packager.target import Target

# The canonical 64-byte DOS stub that Windows PE loaders expect.
# Taken from the PE/COFF specification section A.1.
# Bytes 60-63 hold e_lfanew (little-endian uint32) = 0x40 = 64.
_DOS_STUB: bytes = (
    b"\x4d\x5a\x90\x00\x03\x00\x00\x00\x04\x00\x00\x00\xff\xff\x00\x00"
    b"\xb8\x00\x00\x00\x00\x00\x00\x00\x40\x00\x00\x00\x00\x00\x00\x00"
    b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"
    b"\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x40\x00\x00\x00"
)

_PE_SIGNATURE: bytes = b"PE\x00\x00"

# COFF header constants
_MACHINE_AMD64: int = 0x8664
_CHAR_EXECUTABLE: int = 0x0002
_CHAR_LARGE_ADDRESS: int = 0x0020

# Optional header constants
_PE32PLUS_MAGIC: int = 0x020B
_SUBSYSTEM_CUI: int = 3    # Console application
_SUBSYSTEM_GUI: int = 2    # Windows GUI application

# Default image base for 64-bit Windows (ASLR-friendly, above 2 GiB)
_DEFAULT_IMAGE_BASE: int = 0x140000000

# Alignment constants
_SECTION_ALIGNMENT: int = 0x1000   # 4 KiB — virtual memory page
_FILE_ALIGNMENT: int = 0x200       # 512 bytes — disk sector

# Section characteristics
_SCN_CNT_CODE: int = 0x00000020
_SCN_MEM_EXECUTE: int = 0x20000000
_SCN_MEM_READ: int = 0x40000000

# Struct formats
_COFF_FMT: str = "<HHIIIHH"          # 20 bytes
# PE32+ optional header core (112 bytes).  29 fields, matching the COFF/PE spec exactly:
# Magic(H) MajLinker(B) MinLinker(B)
# SizeOfCode(I) SizeOfInitData(I) SizeOfUninitData(I) AddrOfEntryPoint(I) BaseOfCode(I)
# ImageBase(Q) SectionAlignment(I) FileAlignment(I)
# MajOSVer(H) MinOSVer(H) MajImgVer(H) MinImgVer(H) MajSubVer(H) MinSubVer(H)
# Win32VersionValue(I) SizeOfImage(I) SizeOfHeaders(I) CheckSum(I)
# Subsystem(H) DllCharacteristics(H)
# SizeOfStackReserve(Q) SizeOfStackCommit(Q) SizeOfHeapReserve(Q) SizeOfHeapCommit(Q)
# LoaderFlags(I) NumberOfRvaAndSizes(I)
_OPT_HDR_FMT: str = (
    "<H"        # Magic
    "BB"        # MajorLinkerVersion, MinorLinkerVersion
    "IIIII"     # SizeOfCode, InitData, UninitData, AddressOfEntryPoint, BaseOfCode
    "Q"         # ImageBase (QWORD)
    "II"        # SectionAlignment, FileAlignment
    "HHHHHH"    # MajOS MinOS MajImg MinImg MajSub MinSub
    "IIII"      # Win32VersionValue, SizeOfImage, SizeOfHeaders, CheckSum
    "HH"        # Subsystem, DllCharacteristics
    "QQQQ"      # SizeOfStackReserve, SizeOfStackCommit, SizeOfHeapReserve, SizeOfHeapCommit
    "II"        # LoaderFlags, NumberOfRvaAndSizes
)
_SECTION_FMT: str = "<8sIIIIIIHHI"   # 40 bytes


def _align_up(value: int, alignment: int) -> int:
    return (value + alignment - 1) & ~(alignment - 1)


class PePackager:
    """Produce a minimal PE32+ executable (.exe) for Windows x86-64.

    Accepted targets: :func:`Target.windows_x64`.

    Metadata keys
    -------------
    ``subsystem`` (int)
        PE subsystem.  2 = GUI (``IMAGE_SUBSYSTEM_WINDOWS_GUI``),
        3 = console (``IMAGE_SUBSYSTEM_WINDOWS_CUI``).  Default: 3.
    ``image_base`` (int)
        Virtual base address.  Default: ``0x140000000``.
    """

    supported_targets: frozenset[Target] = frozenset({
        Target.windows_x64(),
    })

    def pack(self, artifact: CodeArtifact) -> bytes:
        if artifact.target not in self.supported_targets:
            raise UnsupportedTargetError(artifact.target)

        code = artifact.native_bytes
        subsystem: int = int(artifact.metadata.get("subsystem", _SUBSYSTEM_CUI))
        image_base: int = int(artifact.metadata.get("image_base", _DEFAULT_IMAGE_BASE))

        # Header sizes (before code)
        # DOS stub (64) + PE sig (4) + COFF (20) + optional (240) + section table (40)
        raw_header_size = len(_DOS_STUB) + 4 + 20 + 240 + 40
        headers_size = _align_up(raw_header_size, _FILE_ALIGNMENT)  # padded to file align

        # Section RVA must be aligned to section_alignment
        text_rva = _align_up(headers_size, _SECTION_ALIGNMENT)
        # File offset of code is headers_size (already file-aligned)
        text_file_offset = headers_size

        raw_code_size = len(code)
        raw_code_padded = _align_up(raw_code_size, _FILE_ALIGNMENT)
        virt_code_size = _align_up(raw_code_size, _SECTION_ALIGNMENT)

        size_of_image = _align_up(text_rva + virt_code_size, _SECTION_ALIGNMENT)

        entry_rva = text_rva + artifact.entry_point

        # ── COFF header ────────────────────────────────────────────────────
        coff = struct.pack(
            _COFF_FMT,
            _MACHINE_AMD64,                         # Machine
            1,                                      # NumberOfSections
            0,                                      # TimeDateStamp (reproducible)
            0,                                      # PointerToSymbolTable
            0,                                      # NumberOfSymbols
            240,                                    # SizeOfOptionalHeader
            _CHAR_EXECUTABLE | _CHAR_LARGE_ADDRESS, # Characteristics
        )

        # ── Optional header PE32+ (240 bytes) ─────────────────────────────
        # Format: Magic(H) MajorLinkerVersion(B) MinorLinkerVersion(B)
        #   SizeOfCode(I) SizeOfInitializedData(I) SizeOfUninitializedData(I)
        #   AddressOfEntryPoint(I) BaseOfCode(I) ImageBase(Q)
        #   SectionAlignment(I) FileAlignment(I)
        #   MajorOSVersion(H) MinorOSVersion(H) MajorImageVersion(H) MinorImageVersion(H)
        #   MajorSubsystemVersion(H) MinorSubsystemVersion(H) Win32VersionValue(I)
        #   SizeOfImage(I) SizeOfHeaders(I) CheckSum(I)
        #   Subsystem(H) DllCharacteristics(H)
        #   SizeOfStackReserve(Q) SizeOfStackCommit(Q)
        #   SizeOfHeapReserve(Q) SizeOfHeapCommit(Q)
        #   LoaderFlags(I) NumberOfRvaAndSizes(I)
        # — then 16 data directory entries, each 8 bytes = 128 bytes
        # Total so far: 112 bytes + 128 = 240 bytes.
        opt_hdr_core = struct.pack(
            _OPT_HDR_FMT,
            _PE32PLUS_MAGIC,      # Magic
            14, 0,                # Linker version (14.0 = MSVC 2022)
            raw_code_padded,      # SizeOfCode
            0,                    # SizeOfInitializedData
            0,                    # SizeOfUninitializedData
            entry_rva,            # AddressOfEntryPoint
            text_rva,             # BaseOfCode
            image_base,           # ImageBase
            _SECTION_ALIGNMENT,   # SectionAlignment
            _FILE_ALIGNMENT,      # FileAlignment
            6, 0,                 # MajorOSVersion, MinorOSVersion (Windows 6.0+)
            0, 0,                 # MajorImageVersion, MinorImageVersion
            6, 0,                 # MajorSubsystemVersion, MinorSubsystemVersion
            0,                    # Win32VersionValue (must be 0)
            size_of_image,        # SizeOfImage
            headers_size,         # SizeOfHeaders
            0,                    # CheckSum (0 = not verified)
            subsystem,            # Subsystem
            0x8160,               # DllCharacteristics (NX compatible, no SEH, ASLR, TS aware)
            0x100000,             # SizeOfStackReserve (1 MiB)
            0x1000,               # SizeOfStackCommit (4 KiB)
            0x100000,             # SizeOfHeapReserve (1 MiB)
            0x1000,               # SizeOfHeapCommit (4 KiB)
            0,                    # LoaderFlags
            16,                   # NumberOfRvaAndSizes (always 16)
        )
        # 16 empty data directory entries (no imports, no exports, etc.)
        data_dirs = b"\x00" * (16 * 8)
        opt_hdr = opt_hdr_core + data_dirs

        # ── Section table ──────────────────────────────────────────────────
        sect_chars = _SCN_CNT_CODE | _SCN_MEM_EXECUTE | _SCN_MEM_READ
        section = struct.pack(
            _SECTION_FMT,
            b".text\x00\x00\x00",  # Name (8 bytes, zero-padded)
            virt_code_size,        # VirtualSize
            text_rva,              # VirtualAddress (RVA)
            raw_code_padded,       # SizeOfRawData
            text_file_offset,      # PointerToRawData
            0,                     # PointerToRelocations
            0,                     # PointerToLinenumbers
            0,                     # NumberOfRelocations
            0,                     # NumberOfLinenumbers
            sect_chars,            # Characteristics
        )

        # ── Assemble ───────────────────────────────────────────────────────
        header_bytes = _DOS_STUB + _PE_SIGNATURE + coff + opt_hdr + section
        padding_needed = headers_size - len(header_bytes)
        header_bytes += b"\x00" * padding_needed

        code_padded = code + b"\x00" * (raw_code_padded - raw_code_size)
        return header_bytes + code_padded

    def file_extension(self, target: Target) -> str:  # noqa: ARG002
        return ".exe"
