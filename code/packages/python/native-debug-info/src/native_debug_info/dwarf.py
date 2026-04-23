"""DwarfEmitter — builds DWARF 4 debug sections from a DebugSidecarReader.

Produces the minimal DWARF subset that lets gdb/lldb set breakpoints by
source line and show function names in stack traces:

  .debug_abbrev  — abbreviation table (defines DIE structure)
  .debug_info    — compilation unit + DW_TAG_subprogram DIEs
  .debug_line    — line number program (instr address → file/line/col)
  .debug_str     — deduplicated string pool

The same four sections work for both ELF (Linux) and Mach-O/macOS — only
the embedding differs (ELF section names vs __DWARF segment).

Usage::

    from native_debug_info import DwarfEmitter
    from debug_sidecar import DebugSidecarReader

    reader = DebugSidecarReader(sidecar_bytes)
    emitter = DwarfEmitter(
        reader=reader,
        load_address=0x400000,
        symbol_table={"fibonacci": 0},   # fn_name → byte offset from load_address
        code_size=256,
    )
    sections = emitter.build()
    elf_with_dwarf = emitter.embed_in_elf(elf_bytes)
    macho_with_dwarf = emitter.embed_in_macho(macho_bytes)
"""

from __future__ import annotations

import struct

from debug_sidecar import DebugSidecarReader

from .leb128 import encode_sleb128, encode_uleb128

# ---------------------------------------------------------------------------
# DWARF 4 constants
# ---------------------------------------------------------------------------

# Tags
DW_TAG_COMPILE_UNIT = 0x11
DW_TAG_SUBPROGRAM = 0x2E

# Children
DW_CHILDREN_YES = 0x01
DW_CHILDREN_NO = 0x00

# Attributes
DW_AT_PRODUCER = 0x25
DW_AT_LANGUAGE = 0x13
DW_AT_NAME = 0x03
DW_AT_COMP_DIR = 0x1B
DW_AT_LOW_PC = 0x11
DW_AT_HIGH_PC = 0x12
DW_AT_STMT_LIST = 0x10
DW_AT_DECL_FILE = 0x3A
DW_AT_DECL_LINE = 0x3B
DW_AT_EXTERNAL = 0x3F

# Forms
DW_FORM_ADDR = 0x01
DW_FORM_DATA1 = 0x08
DW_FORM_DATA2 = 0x05
DW_FORM_DATA4 = 0x06
DW_FORM_DATA8 = 0x07
DW_FORM_STRP = 0x0E
DW_FORM_FLAG_PRESENT = 0x19
DW_FORM_SEC_OFFSET = 0x17  # 4-byte section offset in DWARF 4

# Language
DW_LANG_C99 = 0x0001  # generic placeholder

# Line number opcodes
DW_LNS_COPY = 0x01
DW_LNS_ADVANCE_PC = 0x02
DW_LNS_ADVANCE_LINE = 0x03
DW_LNS_SET_FILE = 0x04
DW_LNE_SET_ADDRESS = 0x02  # extended opcode
DW_LNE_END_SEQUENCE = 0x01  # extended opcode

PRODUCER = "coding-adventures aot-core"


class DwarfEmitter:
    """Builds DWARF 4 sections from a debug sidecar and symbol table.

    Parameters
    ----------
    reader:
        Loaded ``DebugSidecarReader``.
    load_address:
        Virtual address at which the code is loaded (e.g. 0x400000 for ELF).
    symbol_table:
        Maps function name → byte offset from ``load_address``.
        Functions not present default to offset 0.
    code_size:
        Total byte size of all code (used for DW_AT_high_pc on the
        compilation unit DIE).
    """

    def __init__(
        self,
        reader: DebugSidecarReader,
        load_address: int,
        symbol_table: dict[str, int],
        code_size: int,
    ) -> None:
        self._reader = reader
        self._load_address = load_address
        self._symbol_table = symbol_table
        self._code_size = code_size

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def build(self) -> dict[str, bytes]:
        """Build all four DWARF sections.

        Returns
        -------
        dict with keys ".debug_abbrev", ".debug_info", ".debug_line", ".debug_str"
        """
        str_table, str_offsets = self._build_str_table()
        debug_abbrev = self._build_abbrev()
        debug_line = self._build_line(str_offsets)
        debug_info = self._build_info(str_offsets)
        debug_str = str_table
        return {
            ".debug_abbrev": debug_abbrev,
            ".debug_info": debug_info,
            ".debug_line": debug_line,
            ".debug_str": debug_str,
        }

    def embed_in_elf(self, elf_bytes: bytes) -> bytes:
        """Append DWARF sections to an ELF64 file.

        Appends `.debug_abbrev`, `.debug_info`, `.debug_line`, `.debug_str`
        as new ELF sections.  Updates the section header table and string
        table in-place.

        Parameters
        ----------
        elf_bytes:
            Valid ELF64 little-endian binary.

        Returns
        -------
        bytes
            ELF64 binary with DWARF sections added.

        Raises
        ------
        ValueError
            If the input is not a valid 64-bit little-endian ELF.
        """
        data = bytearray(elf_bytes)

        if data[:4] != b"\x7fELF":
            raise ValueError("not a valid ELF file")
        if data[4] != 2:
            raise ValueError("only ELF64 supported")
        if data[5] != 1:
            raise ValueError("only little-endian ELF supported")

        # Parse key header fields
        e_shoff = struct.unpack_from("<Q", data, 40)[0]
        e_shnum = struct.unpack_from("<H", data, 60)[0]
        e_shstrndx = struct.unpack_from("<H", data, 62)[0]

        SHDR_SIZE = 64
        SHT_PROGBITS = 1
        SHT_STRTAB = 3

        # Read all existing section headers
        shdrs = []
        for i in range(e_shnum):
            off = e_shoff + i * SHDR_SIZE
            shdrs.append(bytearray(data[off : off + SHDR_SIZE]))

        # Read the existing section-name string table (.shstrtab)
        shstrtab_shdr = shdrs[e_shstrndx]
        shstrtab_off = struct.unpack_from("<Q", shstrtab_shdr, 24)[0]
        shstrtab_size = struct.unpack_from("<Q", shstrtab_shdr, 32)[0]
        shstrtab = bytearray(data[shstrtab_off : shstrtab_off + shstrtab_size])

        # Build DWARF sections
        dwarf = self.build()
        sec_order = [".debug_abbrev", ".debug_info", ".debug_line", ".debug_str"]

        # Extend .shstrtab with the new section names
        name_offsets: dict[str, int] = {}
        for name in sec_order:
            name_offsets[name] = len(shstrtab)
            shstrtab.extend(name.encode("ascii") + b"\x00")

        def align8(x: int) -> int:
            return x + (8 - x % 8) % 8

        # Append debug section data after the current end of file
        result = bytearray(data)
        debug_sec_file_offsets: dict[str, int] = {}
        for name in sec_order:
            pos = align8(len(result))
            result.extend(b"\x00" * (pos - len(result)))
            debug_sec_file_offsets[name] = len(result)
            result.extend(dwarf[name])

        # Append the extended .shstrtab
        pos = align8(len(result))
        result.extend(b"\x00" * (pos - len(result)))
        new_shstrtab_file_off = len(result)
        result.extend(shstrtab)

        # Build a new section header table (existing + 4 new debug sections)
        pos = align8(len(result))
        result.extend(b"\x00" * (pos - len(result)))
        new_shdr_table_off = len(result)

        # Copy existing section headers; update the .shstrtab entry
        for i, shdr in enumerate(shdrs):
            s = bytearray(shdr)
            if i == e_shstrndx:
                struct.pack_into("<Q", s, 24, new_shstrtab_file_off)  # sh_offset
                struct.pack_into("<Q", s, 32, len(shstrtab))          # sh_size
            result.extend(s)

        # Write four new debug section headers
        for name in sec_order:
            shdr = struct.pack(
                "<IIQQQQIIQQ",
                name_offsets[name],           # sh_name
                SHT_PROGBITS,                 # sh_type
                0,                            # sh_flags
                0,                            # sh_addr
                debug_sec_file_offsets[name], # sh_offset
                len(dwarf[name]),             # sh_size
                0,                            # sh_link
                0,                            # sh_info
                1,                            # sh_addralign
                0,                            # sh_entsize
            )
            result.extend(shdr)

        # Update ELF header: new section header table offset and count
        struct.pack_into("<Q", result, 40, new_shdr_table_off)
        struct.pack_into("<H", result, 60, e_shnum + len(sec_order))
        # e_shstrndx unchanged (same index in the new table)

        return bytes(result)

    def embed_in_macho(self, macho_bytes: bytes) -> bytes:
        """Append a __DWARF segment with DWARF sections to a Mach-O 64-bit file.

        Inserts a new LC_SEGMENT_64 load command for __DWARF.  All existing
        section file offsets are shifted by the size of the new load command.

        Parameters
        ----------
        macho_bytes:
            Valid 64-bit little-endian Mach-O binary.

        Returns
        -------
        bytes
            Mach-O binary with __DWARF segment added.

        Raises
        ------
        ValueError
            If the input is not a valid 64-bit little-endian Mach-O.
        """
        data = bytearray(macho_bytes)

        MH_MAGIC_64 = 0xFEEDFACF
        if struct.unpack_from("<I", data, 0)[0] != MH_MAGIC_64:
            raise ValueError("not a valid 64-bit little-endian Mach-O")

        ncmds, sizeofcmds = struct.unpack_from("<II", data, 16)

        dwarf = self.build()
        sec_order = [".debug_abbrev", ".debug_info", ".debug_line", ".debug_str"]

        # The new LC_SEGMENT_64: 72-byte header + 4 × 80-byte sections = 392 bytes
        LC_SEGMENT_64 = 0x19
        new_lc_size = 72 + 4 * 80  # 392

        # Shift all section file offsets in existing LC_SEGMENT_64 commands
        lc_off = 32
        for _ in range(ncmds):
            cmd, cmdsize = struct.unpack_from("<II", data, lc_off)
            if cmd == LC_SEGMENT_64:
                seg_foff = struct.unpack_from("<Q", data, lc_off + 40)[0]
                if seg_foff > 0:
                    struct.pack_into("<Q", data, lc_off + 40, seg_foff + new_lc_size)
                nsects = struct.unpack_from("<I", data, lc_off + 64)[0]
                for s in range(nsects):
                    off_field = lc_off + 72 + s * 80 + 48  # 'offset' in section_64
                    sec_foff = struct.unpack_from("<I", data, off_field)[0]
                    if sec_foff > 0:
                        struct.pack_into("<I", data, off_field, sec_foff + new_lc_size)
                    reloff_field = lc_off + 72 + s * 80 + 56  # reloff
                    reloff = struct.unpack_from("<I", data, reloff_field)[0]
                    if reloff > 0:
                        struct.pack_into("<I", data, reloff_field, reloff + new_lc_size)
            lc_off += cmdsize

        # Debug section data goes after all existing content (+ new_lc_size shift)
        debug_data_start = len(data) + new_lc_size
        debug_offsets: dict[str, int] = {}
        pos = debug_data_start
        for name in sec_order:
            debug_offsets[name] = pos
            pos += len(dwarf[name])
            pos = (pos + 3) & ~3  # 4-byte align between sections

        new_lc = self._build_dwarf_lc(debug_offsets, dwarf, sec_order)
        assert len(new_lc) == new_lc_size

        # Update Mach-O header: ncmds + 1, sizeofcmds + new_lc_size
        struct.pack_into("<II", data, 16, ncmds + 1, sizeofcmds + new_lc_size)

        # Assemble result: [header+LCs (updated)] + [new LC] + [existing data] + [debug]
        existing_data_start = 32 + sizeofcmds
        result = bytearray()
        result.extend(data[:existing_data_start])
        result.extend(new_lc)
        result.extend(data[existing_data_start:])

        for name in sec_order:
            result.extend(dwarf[name])
            while len(result) % 4 != 0:
                result.append(0)

        return bytes(result)

    # ------------------------------------------------------------------
    # Section builders
    # ------------------------------------------------------------------

    def _build_str_table(self) -> tuple[bytes, dict[str, int]]:
        """Build the .debug_str string pool and return (bytes, {string: offset})."""
        ordered = [PRODUCER, ""]  # producer, comp_dir
        files = self._reader.source_files()
        ordered.extend(files if files else ["<unknown>"])
        ordered.extend(self._reader.function_names())

        offsets: dict[str, int] = {}
        buf = bytearray()
        for s in ordered:
            if s not in offsets:
                offsets[s] = len(buf)
                buf.extend(s.encode("utf-8") + b"\x00")
        return bytes(buf), offsets

    def _build_abbrev(self) -> bytes:
        """Build the fixed .debug_abbrev section.

        Defines two abbreviations:
          1 — DW_TAG_compile_unit (has children)
          2 — DW_TAG_subprogram   (no children)
        """
        buf = bytearray()

        # Abbrev 1: DW_TAG_compile_unit, has children
        buf += encode_uleb128(1)
        buf += encode_uleb128(DW_TAG_COMPILE_UNIT)
        buf.append(DW_CHILDREN_YES)
        for at, form in [
            (DW_AT_PRODUCER,   DW_FORM_STRP),
            (DW_AT_LANGUAGE,   DW_FORM_DATA2),
            (DW_AT_NAME,       DW_FORM_STRP),
            (DW_AT_COMP_DIR,   DW_FORM_STRP),
            (DW_AT_LOW_PC,     DW_FORM_ADDR),
            (DW_AT_HIGH_PC,    DW_FORM_DATA8),
            (DW_AT_STMT_LIST,  DW_FORM_SEC_OFFSET),
        ]:
            buf += encode_uleb128(at)
            buf += encode_uleb128(form)
        buf += b"\x00\x00"  # attribute list terminator

        # Abbrev 2: DW_TAG_subprogram, no children
        buf += encode_uleb128(2)
        buf += encode_uleb128(DW_TAG_SUBPROGRAM)
        buf.append(DW_CHILDREN_NO)
        for at, form in [
            (DW_AT_NAME,      DW_FORM_STRP),
            (DW_AT_DECL_FILE, DW_FORM_DATA1),
            (DW_AT_DECL_LINE, DW_FORM_DATA4),
            (DW_AT_LOW_PC,    DW_FORM_ADDR),
            (DW_AT_HIGH_PC,   DW_FORM_DATA8),
            (DW_AT_EXTERNAL,  DW_FORM_FLAG_PRESENT),
        ]:
            buf += encode_uleb128(at)
            buf += encode_uleb128(form)
        buf += b"\x00\x00"  # attribute list terminator

        buf.append(0)  # end of abbreviation table
        return bytes(buf)

    def _build_line(self, str_offsets: dict[str, int]) -> bytes:
        """Build the .debug_line section (line number program).

        Uses the simplest encoding: one DW_LNS_copy per recorded row,
        DW_LNE_set_address to set absolute addresses for each function,
        DW_LNE_end_sequence to close each function's block.
        """
        source_files = self._reader.source_files()

        # --- Header (fixed portion, minus unit_length and header_length) ---
        std_opcode_lengths = bytes([0, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 1])

        header_body = bytearray()
        header_body.append(1)   # minimum_instruction_length
        header_body.append(1)   # maximum_ops_per_instruction
        header_body.append(1)   # default_is_stmt
        header_body.append(0xFB)  # line_base = -5 (signed byte, two's complement)
        header_body.append(14)  # line_range
        header_body.append(13)  # opcode_base (13 standard opcodes)
        header_body.extend(std_opcode_lengths)
        header_body.append(0)   # include_directories: empty list (single null byte)

        # File names table (1-based in DWARF; index 0 is unused)
        for path in source_files:
            # DWARF uses just the base name in the file_names table;
            # directories go in the include_directories list.  For simplicity
            # we store the full path as the file name with dir_index=0.
            header_body.extend(path.encode("utf-8") + b"\x00")
            header_body += encode_uleb128(0)  # dir_index (0 = no directory entry)
            header_body += encode_uleb128(0)  # mtime
            header_body += encode_uleb128(0)  # file size
        header_body.append(0)   # file names table terminator

        # --- Line number program body ---
        program = bytearray()

        for fn_name in self._reader.function_names():
            fn_range = self._reader.function_range(fn_name)
            if fn_range is None:
                continue
            start_instr, end_instr = fn_range
            fn_offset = self._symbol_table.get(fn_name, 0)
            fn_address = self._load_address + fn_offset

            # Get all line table rows for this function via the private table.
            # We access _raw_line_table directly since native_debug_info is a
            # privileged companion package.  A future reader.rows(fn) API would
            # be cleaner.
            raw_rows = self._reader._raw_line_table.get(fn_name, [])
            if not raw_rows:
                continue

            # DW_LNE_set_address: set absolute address for this function
            addr_bytes = struct.pack("<Q", fn_address)
            program.append(0x00)  # extended opcode marker
            program += encode_uleb128(1 + 8)  # length: opcode(1) + addr(8)
            program.append(DW_LNE_SET_ADDRESS)
            program.extend(addr_bytes)

            cur_line = 1
            cur_file_idx = 1  # 1-based

            for row in raw_rows:
                instr_idx = row["instr_index"]
                row_line = row["line"]
                row_file_id = row["file_id"]
                row_file_idx = row_file_id + 1  # DWARF is 1-based

                # Advance PC to instr_idx (treating each IIR instruction as 1 byte)
                pc_delta = instr_idx  # from function start
                if pc_delta > 0:
                    program.append(DW_LNS_ADVANCE_PC)
                    program += encode_uleb128(pc_delta)

                # Set file if changed
                if row_file_idx != cur_file_idx:
                    program.append(DW_LNS_SET_FILE)
                    program += encode_uleb128(row_file_idx)
                    cur_file_idx = row_file_idx

                # Advance line if changed
                line_delta = row_line - cur_line
                if line_delta != 0:
                    program.append(DW_LNS_ADVANCE_LINE)
                    program += encode_sleb128(line_delta)
                    cur_line = row_line

                # Reset PC for next row (each row is relative to function start)
                if pc_delta > 0:
                    program.append(DW_LNS_ADVANCE_PC)
                    program += encode_uleb128(-pc_delta & 0xFFFFFFFFFFFFFFFF)

                program.append(DW_LNS_COPY)

            # DW_LNE_end_sequence
            program.append(0x00)
            program += encode_uleb128(1)
            program.append(DW_LNE_END_SEQUENCE)

        # --- Assemble with length fields ---
        # header_length: from after the header_length field to start of program
        header_length = len(header_body)
        # unit_length: from after unit_length field to end of section
        # = 2 (version) + 4 (header_length) + header_length + len(program)
        unit_length = 2 + 4 + header_length + len(program)

        buf = bytearray()
        buf += struct.pack("<IHI", unit_length, 4, header_length)
        buf.extend(header_body)
        buf.extend(program)
        return bytes(buf)

    def _build_info(self, str_offsets: dict[str, int]) -> bytes:
        """Build the .debug_info section (compilation unit DIE + subprogram DIEs)."""
        source_files = self._reader.source_files()
        primary_file = source_files[0] if source_files else "<unknown>"

        # Build DIE body (everything after the compile unit header)
        dies = bytearray()

        # Compile unit DIE (abbrev 1)
        dies += encode_uleb128(1)
        dies += struct.pack("<I", str_offsets.get(PRODUCER, 0))        # DW_AT_producer  (strp)
        dies += struct.pack("<H", DW_LANG_C99)                          # DW_AT_language  (data2)
        dies += struct.pack("<I", str_offsets.get(primary_file, 0))     # DW_AT_name      (strp)
        dies += struct.pack("<I", str_offsets.get("", 0))               # DW_AT_comp_dir  (strp)
        dies += struct.pack("<Q", self._load_address)                    # DW_AT_low_pc    (addr)
        dies += struct.pack("<Q", self._code_size)                       # DW_AT_high_pc   (data8)
        dies += struct.pack("<I", 0)                                     # DW_AT_stmt_list (sec_offset → 0)

        # One DW_TAG_subprogram per function
        for fn_name in self._reader.function_names():
            fn_range = self._reader.function_range(fn_name)
            fn_offset = self._symbol_table.get(fn_name, 0)
            fn_address = self._load_address + fn_offset

            # Find the first source line for this function
            raw_rows = self._reader._raw_line_table.get(fn_name, [])
            decl_line = raw_rows[0]["line"] if raw_rows else 1
            decl_file_id = raw_rows[0]["file_id"] if raw_rows else 0
            decl_file_idx = decl_file_id + 1  # DWARF 1-based

            # Byte length of the function
            if fn_range is not None:
                start_instr, end_instr = fn_range
                fn_byte_len = end_instr - start_instr
            else:
                fn_byte_len = 0

            dies += encode_uleb128(2)                                        # abbrev 2
            dies += struct.pack("<I", str_offsets.get(fn_name, 0))          # DW_AT_name    (strp)
            dies.append(decl_file_idx & 0xFF)                                # DW_AT_decl_file (data1)
            dies += struct.pack("<I", decl_line)                             # DW_AT_decl_line (data4)
            dies += struct.pack("<Q", fn_address)                            # DW_AT_low_pc  (addr)
            dies += struct.pack("<Q", fn_byte_len)                           # DW_AT_high_pc (data8)
            # DW_AT_external with DW_FORM_flag_present occupies 0 bytes

        dies.append(0)  # end of children (compile unit)

        # Compile unit header: unit_length, version, abbrev offset, address size
        # unit_length = size from after unit_length field to end
        # = 2 (version) + 4 (abbrev_offset) + 1 (addr_size) + len(dies)
        unit_length = 2 + 4 + 1 + len(dies)
        header = struct.pack("<IHIB", unit_length, 4, 0, 8)  # version=4, abbrev_off=0, addr_size=8

        return header + bytes(dies)

    # ------------------------------------------------------------------
    # Mach-O helper
    # ------------------------------------------------------------------

    def _build_dwarf_lc(
        self,
        debug_offsets: dict[str, int],
        dwarf: dict[str, bytes],
        sec_order: list[str],
    ) -> bytes:
        """Build a LC_SEGMENT_64 load command for the __DWARF segment."""
        LC_SEGMENT_64 = 0x19
        n_sects = len(sec_order)
        cmdsize = 72 + n_sects * 80
        total_size = sum(len(dwarf[n]) for n in sec_order)
        seg_fileoff = debug_offsets[sec_order[0]]

        buf = bytearray()

        # LC_SEGMENT_64 header (72 bytes)
        buf += struct.pack("<II", LC_SEGMENT_64, cmdsize)
        buf += b"__DWARF\x00\x00\x00\x00\x00\x00\x00\x00\x00"  # segname (16 bytes)
        buf += struct.pack(
            "<QQQQiiII",
            0,           # vmaddr (0 — debug segments are not mapped)
            total_size,  # vmsize
            seg_fileoff, # fileoff
            total_size,  # filesize
            7,           # maxprot  (RWX)
            5,           # initprot (RX)
            n_sects,     # nsects
            0,           # flags
        )

        # section_64 entries (80 bytes each)
        macho_name = {
            ".debug_abbrev": "__debug_abbrev",
            ".debug_info":   "__debug_info",
            ".debug_line":   "__debug_line",
            ".debug_str":    "__debug_str",
        }
        for name in sec_order:
            mname = macho_name[name].encode("ascii").ljust(16, b"\x00")
            segname = b"__DWARF\x00\x00\x00\x00\x00\x00\x00\x00\x00"
            buf += mname                # sectname (16)
            buf += segname              # segname  (16)
            buf += struct.pack(
                "<QQIIIIIIII",
                0,                    # addr
                len(dwarf[name]),     # size
                debug_offsets[name],  # offset (file offset, u32 range)
                0,                    # align (2^0 = 1)
                0,                    # reloff
                0,                    # nreloc
                0x02000000,           # flags: S_ATTR_DEBUG
                0,                    # reserved1
                0,                    # reserved2
                0,                    # reserved3
            )

        return bytes(buf)
