"""Shared test fixtures for native-debug-info."""

import struct

import pytest

from debug_sidecar.reader import DebugSidecarReader
from debug_sidecar.writer import DebugSidecarWriter


def _fibonacci_reader() -> DebugSidecarReader:
    """Build a realistic DebugSidecarReader for the fibonacci function."""
    w = DebugSidecarWriter()
    fid = w.add_source_file("fibonacci.tetrad")
    w.begin_function("fibonacci", start_instr=0, param_count=1)
    w.declare_variable("fibonacci", reg_index=0, name="n", type_hint="any",
                       live_start=0, live_end=8)
    for idx, (line, col) in enumerate([
        (1, 1), (2, 5), (2, 5), (3, 9), (4, 5), (4, 5), (4, 5), (5, 1),
    ]):
        w.record("fibonacci", idx, file_id=fid, line=line, col=col)
    w.end_function("fibonacci", end_instr=8)
    return DebugSidecarReader(w.finish())


@pytest.fixture
def fibonacci_reader() -> DebugSidecarReader:
    return _fibonacci_reader()


@pytest.fixture
def minimal_elf64() -> bytes:
    """Minimal valid ELF64 LE: header + NULL shdr + .shstrtab shdr + shstrtab data."""
    shstrtab = b"\x00.shstrtab\x00"

    e_ident = (
        b"\x7fELF"
        b"\x02"       # ELFCLASS64
        b"\x01"       # ELFDATA2LSB
        b"\x01"       # EV_CURRENT
        b"\x00"       # ELFOSABI_NONE
        + b"\x00" * 8
    )

    # Section headers at offset 64 (right after ELF header)
    # shstrtab data at offset 64 + 2*64 = 192
    shstrtab_off = 192

    header = struct.pack(
        "<16sHHIQQQIHHHHHH",
        e_ident,
        2,           # e_type = ET_EXEC
        0x3E,        # e_machine = EM_X86_64
        1,           # e_version
        0x400000,    # e_entry
        0,           # e_phoff
        64,          # e_shoff (section headers right after ELF header)
        0,           # e_flags
        64,          # e_ehsize
        56,          # e_phentsize
        0,           # e_phnum
        64,          # e_shentsize
        2,           # e_shnum
        1,           # e_shstrndx
    )

    sh_null = struct.pack("<IIQQQQIIQQ", 0, 0, 0, 0, 0, 0, 0, 0, 0, 0)

    sh_shstrtab = struct.pack(
        "<IIQQQQIIQQ",
        1,               # sh_name (offset 1 = ".shstrtab")
        3,               # sh_type = SHT_STRTAB
        0, 0,            # sh_flags, sh_addr
        shstrtab_off,    # sh_offset
        len(shstrtab),   # sh_size
        0, 0,            # sh_link, sh_info
        1,               # sh_addralign
        0,               # sh_entsize
    )

    return header + sh_null + sh_shstrtab + shstrtab


@pytest.fixture
def minimal_macho64() -> bytes:
    """Minimal valid 64-bit LE Mach-O with no load commands."""
    MH_MAGIC_64 = 0xFEEDFACF
    return struct.pack(
        "<IiiiIIII",
        MH_MAGIC_64,
        0x01000007,  # CPU_TYPE_X86_64
        3,           # CPU_SUBTYPE_ALL
        2,           # MH_EXECUTE
        0,           # ncmds
        0,           # sizeofcmds
        0,           # flags
        0,           # reserved
    )


@pytest.fixture
def minimal_pe32plus() -> bytes:
    """Minimal valid PE32+ with SizeOfHeaders=512 and no sections."""
    e_lfanew = 64
    dos_header = b"MZ" + b"\x00" * 58 + struct.pack("<I", e_lfanew)

    coff = struct.pack("<HHIIIHH",
        0x8664,  # IMAGE_FILE_MACHINE_AMD64
        0,       # NumberOfSections
        0, 0, 0,
        240,     # SizeOfOptionalHeader
        0x0022,  # Characteristics
    )

    opt = bytearray(240)
    struct.pack_into("<H", opt, 0, 0x020B)   # Magic = PE32+
    struct.pack_into("<I", opt, 32, 4096)    # SectionAlignment
    struct.pack_into("<I", opt, 36, 512)     # FileAlignment
    struct.pack_into("<I", opt, 56, 4096)    # SizeOfImage
    struct.pack_into("<I", opt, 60, 512)     # SizeOfHeaders

    raw = dos_header + b"PE\x00\x00" + coff + bytes(opt)
    return raw + b"\x00" * (512 - len(raw))  # pad to SizeOfHeaders
