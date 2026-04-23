"""CodeViewEmitter — builds CodeView 4 debug sections for PE/Windows.

Produces two PE sections:

  .debug$S  — symbols + line numbers
             DEBUG_S_SYMBOLS (0xF1): S_GPROC32 records for each function
             DEBUG_S_FILECHKSMS (0xF4): file checksum entries
             DEBUG_S_LINES (0xF2): source line → code offset mapping

  .debug$T  — minimal type section (one LF_PROCEDURE entry) so WinDbg
              does not complain about missing type info.

These are embedded into a PE32+ binary by ``embed_in_pe()``.

WinDbg / Visual Studio / dumpbin read this format without a separate .pdb.

Usage::

    from native_debug_info import CodeViewEmitter
    from debug_sidecar import DebugSidecarReader

    reader = DebugSidecarReader(sidecar_bytes)
    emitter = CodeViewEmitter(
        reader=reader,
        image_base=0x140000000,
        symbol_table={"main": 0},   # fn_name → byte offset from code_rva
        code_rva=0x1000,            # RVA of .text section
    )
    pe_with_cv = emitter.embed_in_pe(pe_bytes)
"""

from __future__ import annotations

import struct

from debug_sidecar import DebugSidecarReader

# ---------------------------------------------------------------------------
# CodeView 4 constants
# ---------------------------------------------------------------------------

CV_SIGNATURE = 4

# Subsection types
DEBUG_S_SYMBOLS = 0xF1
DEBUG_S_LINES = 0xF2
DEBUG_S_STRINGTABLE = 0xF3
DEBUG_S_FILECHKSMS = 0xF4

# Symbol record types
S_GPROC32 = 0x1110
S_END = 0x0006

# PE section characteristics for debug sections (discardable, read-only, initialized data)
IMAGE_SCN_CNT_INITIALIZED_DATA = 0x00000040
IMAGE_SCN_MEM_DISCARDABLE = 0x02000000
IMAGE_SCN_MEM_READ = 0x40000000
DEBUG_SECTION_CHARACTERISTICS = (
    IMAGE_SCN_CNT_INITIALIZED_DATA | IMAGE_SCN_MEM_DISCARDABLE | IMAGE_SCN_MEM_READ
)


def _pad4(buf: bytearray) -> None:
    """Pad buf to 4-byte alignment with zero bytes."""
    while len(buf) % 4 != 0:
        buf.append(0)


class CodeViewEmitter:
    """Builds CodeView 4 sections from a debug sidecar and symbol table.

    Parameters
    ----------
    reader:
        Loaded ``DebugSidecarReader``.
    image_base:
        PE image base address (e.g. 0x140000000 for 64-bit executables).
    symbol_table:
        Maps function name → byte offset from ``code_rva``.
    code_rva:
        RVA of the .text section in the PE.
    code_section_index:
        1-based section index for .text (default 1).
    """

    def __init__(
        self,
        reader: DebugSidecarReader,
        image_base: int,
        symbol_table: dict[str, int],
        code_rva: int,
        code_section_index: int = 1,
    ) -> None:
        self._reader = reader
        self._image_base = image_base
        self._symbol_table = symbol_table
        self._code_rva = code_rva
        self._code_section_index = code_section_index

    # ------------------------------------------------------------------
    # Public API
    # ------------------------------------------------------------------

    def build(self) -> dict[str, bytes]:
        """Build both CodeView sections.

        Returns
        -------
        dict with keys ".debug$S" and ".debug$T"
        """
        return {
            ".debug$S": self._build_debug_s(),
            ".debug$T": self._build_debug_t(),
        }

    def embed_in_pe(self, pe_bytes: bytes) -> bytes:
        """Append .debug$S and .debug$T sections to a PE32+ file.

        Checks that there is space in the section header table for two new
        section headers.  Appends section data at the end of the file,
        aligned to FileAlignment.

        Parameters
        ----------
        pe_bytes:
            Valid PE32+ (64-bit) little-endian binary.

        Returns
        -------
        bytes
            PE32+ binary with .debug$S and .debug$T sections added.

        Raises
        ------
        ValueError
            If the input is not a valid PE32+ file or lacks header space.
        """
        data = bytearray(pe_bytes)

        # Parse DOS header
        if data[:2] != b"MZ":
            raise ValueError("not a valid PE file")
        pe_off = struct.unpack_from("<I", data, 60)[0]
        if data[pe_off : pe_off + 4] != b"PE\x00\x00":
            raise ValueError("not a valid PE file (no PE signature)")

        coff_off = pe_off + 4
        num_sections, opt_hdr_size = struct.unpack_from("<xHxxxIxxH", data, coff_off)[0], \
            struct.unpack_from("<xHxxxIxxH", data, coff_off)[2]
        # Re-parse cleanly
        (machine, num_sections, _, _, _, opt_hdr_size, _) = struct.unpack_from(
            "<HHIIIHH", data, coff_off
        )

        opt_off = coff_off + 20
        magic = struct.unpack_from("<H", data, opt_off)[0]
        if magic != 0x020B:
            raise ValueError("only PE32+ (64-bit) supported")

        file_alignment = struct.unpack_from("<I", data, opt_off + 36)[0]
        section_alignment = struct.unpack_from("<I", data, opt_off + 32)[0]
        size_of_headers = struct.unpack_from("<I", data, opt_off + 60)[0]

        section_table_off = opt_off + opt_hdr_size
        section_table_size = num_sections * 40

        # Check space for 2 new section headers
        available = size_of_headers - section_table_off - section_table_size
        if available < 2 * 40:
            raise ValueError(
                f"insufficient PE header space for 2 new debug sections "
                f"(need 80 bytes, have {available})"
            )

        # Find highest existing section's virtual end (for new section RVAs)
        last_rva = 0
        last_vsize = 0
        for i in range(num_sections):
            off = section_table_off + i * 40
            vsize, rva = struct.unpack_from("<II", data, off + 8)
            if rva >= last_rva:
                last_rva = rva
                last_vsize = vsize

        def align_to(x: int, n: int) -> int:
            return (x + n - 1) & ~(n - 1)

        # Build CodeView section data
        cv = self.build()
        debug_s = cv[".debug$S"]
        debug_t = cv[".debug$T"]

        # Assign RVAs and file offsets
        debug_s_rva = align_to(last_rva + last_vsize, section_alignment)
        debug_t_rva = align_to(debug_s_rva + len(debug_s), section_alignment)

        file_end = len(data)
        debug_s_raw_start = align_to(file_end, file_alignment)
        debug_s_raw_size = align_to(len(debug_s), file_alignment)
        debug_t_raw_start = debug_s_raw_start + debug_s_raw_size
        debug_t_raw_size = align_to(len(debug_t), file_alignment)

        new_size_of_image = align_to(debug_t_rva + len(debug_t), section_alignment)

        # Write two new section headers into the existing header area
        def make_section_header(name8: bytes, vsize: int, rva: int,
                                raw_size: int, raw_off: int) -> bytes:
            return (
                name8[:8].ljust(8, b"\x00")
                + struct.pack("<IIIIIIHHI", vsize, rva, raw_size, raw_off,
                              0, 0, 0, 0, DEBUG_SECTION_CHARACTERISTICS)
            )

        new_sh_s_off = section_table_off + num_sections * 40
        new_sh_t_off = new_sh_s_off + 40

        sh_s = make_section_header(b".debug$S", len(debug_s), debug_s_rva,
                                   debug_s_raw_size, debug_s_raw_start)
        sh_t = make_section_header(b".debug$T", len(debug_t), debug_t_rva,
                                   debug_t_raw_size, debug_t_raw_start)

        data[new_sh_s_off : new_sh_s_off + 40] = sh_s
        data[new_sh_t_off : new_sh_t_off + 40] = sh_t

        # Update COFF header: NumberOfSections += 2
        struct.pack_into("<H", data, coff_off + 2, num_sections + 2)

        # Update Optional header: SizeOfImage
        struct.pack_into("<I", data, opt_off + 56, new_size_of_image)

        # Append section data
        result = bytearray(data)
        while len(result) < debug_s_raw_start:
            result.append(0)
        result.extend(debug_s)
        while len(result) < debug_t_raw_start:
            result.append(0)
        result.extend(debug_t)

        return bytes(result)

    # ------------------------------------------------------------------
    # Section builders
    # ------------------------------------------------------------------

    def _build_debug_s(self) -> bytes:
        """Build the .debug$S section (symbols + file checksums + line numbers)."""
        buf = bytearray()
        buf += struct.pack("<I", CV_SIGNATURE)

        source_files = self._reader.source_files()

        # --- DEBUG_S_FILECHKSMS (0xF4): one entry per source file ---
        # Each entry: offset_in_name_table(u32) + size(u8) + kind(u8) + padding
        # We build a minimal name table: just the file paths concatenated.
        name_table = bytearray()
        name_offsets: list[int] = []
        for path in source_files:
            name_offsets.append(len(name_table))
            name_table.extend(path.encode("utf-8") + b"\x00")

        # Pad name_table to 4 bytes
        while len(name_table) % 4 != 0:
            name_table.append(0)

        # DEBUG_S_STRINGTABLE (0xF3): write the file path strings so that
        # DEBUG_S_FILECHKSMS offsets can reference them by index.
        buf += struct.pack("<II", DEBUG_S_STRINGTABLE, len(name_table))
        buf.extend(name_table)
        _pad4(buf)

        chksms_data = bytearray()
        for off in name_offsets:
            # offset(u32) + checksum_size(u8=0, no checksum) + kind(u8=0) + 2 bytes padding
            chksms_data += struct.pack("<IBBH", off, 0, 0, 0)

        # File checksum subsection
        buf += struct.pack("<II", DEBUG_S_FILECHKSMS, len(chksms_data))
        buf.extend(chksms_data)
        _pad4(buf)

        # --- DEBUG_S_SYMBOLS (0xF1): one S_GPROC32 + S_END per function ---
        sym_data = bytearray()
        for fn_name in self._reader.function_names():
            fn_range = self._reader.function_range(fn_name)
            fn_offset = self._symbol_table.get(fn_name, 0)
            fn_rva = self._code_rva + fn_offset
            fn_len = 0
            if fn_range is not None:
                start_instr, end_instr = fn_range
                fn_len = end_instr - start_instr

            name_bytes = fn_name.encode("utf-8") + b"\x00"
            # Pad name to 4-byte boundary (record must be 4-byte aligned)
            while (2 + len(name_bytes)) % 4 != 0:
                name_bytes += b"\x00"

            # S_GPROC32 fixed payload: parent(4)+end(4)+next(4)+len(4)+dbg_start(4)+
            #   dbg_end(4)+type_index(4)+offset(4)+segment(2)+flags(1) = 35 bytes
            # record_length = 2 (type field) + 35 + len(name_bytes)
            fixed = struct.pack(
                "<IIIIIIIIHb",
                0,          # parent
                0,          # end (filled by linker)
                0,          # next
                fn_len,     # proc_len
                0,          # dbg_start
                fn_len,     # dbg_end
                0,          # type_index
                fn_rva,     # offset (RVA)
                self._code_section_index,  # segment
                0,          # flags
            )
            record_len = 2 + len(fixed) + len(name_bytes)  # 2 = sizeof(type field)
            sym_data += struct.pack("<HH", record_len, S_GPROC32)
            sym_data += fixed
            sym_data += name_bytes

            # S_END record
            sym_data += struct.pack("<HH", 2, S_END)

        buf += struct.pack("<II", DEBUG_S_SYMBOLS, len(sym_data))
        buf.extend(sym_data)
        _pad4(buf)

        # --- DEBUG_S_LINES (0xF2): per-function line mapping ---
        for i, fn_name in enumerate(self._reader.function_names()):
            fn_range = self._reader.function_range(fn_name)
            fn_offset = self._symbol_table.get(fn_name, 0)
            fn_rva = self._code_rva + fn_offset
            fn_len = 0
            if fn_range is not None:
                start_instr, end_instr = fn_range
                fn_len = end_instr - start_instr

            raw_rows = self._reader._raw_line_table.get(fn_name, [])
            if not raw_rows:
                continue

            # Group rows by file_id
            by_file: dict[int, list[dict]] = {}
            for row in raw_rows:
                fid = row["file_id"]
                by_file.setdefault(fid, []).append(row)

            # Build lines data
            lines_data = bytearray()
            # Header: offset_of_contrib(u32) + section_index(u16) + flags(u16) + code_size(u32)
            lines_data += struct.pack("<IHHI", fn_rva, self._code_section_index, 0, fn_len)

            for file_id, rows in by_file.items():
                # file_checksum_offset: 6 bytes per entry in DEBUG_S_FILECHKSMS
                chksm_off = file_id * 8  # 8 bytes per entry (offset+size+kind+pad)
                num_lines = len(rows)
                block_size = 12 + num_lines * 8  # 12 = file_block header
                lines_data += struct.pack("<III", chksm_off, num_lines, block_size)
                for row in rows:
                    code_offset = row["instr_index"]  # byte offset within function
                    line = row["line"] & 0xFFFFFF     # bits 0-23
                    is_stmt = 1 << 31
                    lines_data += struct.pack("<II", code_offset, line | is_stmt)

            buf += struct.pack("<II", DEBUG_S_LINES, len(lines_data))
            buf.extend(lines_data)
            _pad4(buf)

        return bytes(buf)

    def _build_debug_t(self) -> bytes:
        """Build a minimal .debug$T type section.

        Emits CodeView signature + one LF_PROCEDURE record with void return
        type and no arguments.  Without this some tools emit warnings about
        missing type info; with it they are satisfied enough to show symbols.
        """
        buf = bytearray()
        buf += struct.pack("<I", CV_SIGNATURE)

        # LF_PROCEDURE (0x1008): return_type(u32) + calling_conv(u8) + attrs(u8) +
        #   param_count(u16) + arg_list(u32)
        # T_VOID = 0x0003, CV_CALL_NEARC = 0x00
        LF_PROCEDURE = 0x1008
        record_data = struct.pack("<IBBHi", 0x0003, 0x00, 0x00, 0, 0)
        record_len = 2 + len(record_data)  # 2 = sizeof(leaf_type)
        buf += struct.pack("<HH", record_len, LF_PROCEDURE)
        buf.extend(record_data)
        _pad4(buf)

        return bytes(buf)
