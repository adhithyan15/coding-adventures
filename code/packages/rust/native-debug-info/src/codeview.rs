//! [`CodeViewEmitter`] — builds CodeView 4 debug sections for PE/Windows.
//!
//! Produces two PE sections:
//!
//! - `.debug$S` — symbols + line numbers
//!   - `DEBUG_S_SYMBOLS` (`0xF1`): `S_GPROC32` records for each function
//!   - `DEBUG_S_FILECHKSMS` (`0xF4`): file checksum entries
//!   - `DEBUG_S_LINES` (`0xF2`): source line → code offset mapping
//!
//! - `.debug$T` — minimal type section (one `LF_PROCEDURE` entry) so WinDbg
//!   does not complain about missing type info.
//!
//! WinDbg / Visual Studio / dumpbin read this format without a separate `.pdb`.

use std::collections::HashMap;

use debug_sidecar::DebugSidecarReader;

// ---------------------------------------------------------------------------
// CodeView 4 constants
// ---------------------------------------------------------------------------

const CV_SIGNATURE: u32 = 4;

// Subsection types
const DEBUG_S_SYMBOLS: u32 = 0xF1;
const DEBUG_S_LINES: u32 = 0xF2;
const DEBUG_S_STRINGTABLE: u32 = 0xF3;
const DEBUG_S_FILECHKSMS: u32 = 0xF4;

// Symbol record types
const S_GPROC32: u16 = 0x1110;
const S_END: u16 = 0x0006;

// PE section characteristics
const IMAGE_SCN_CNT_INITIALIZED_DATA: u32 = 0x00000040;
const IMAGE_SCN_MEM_DISCARDABLE: u32 = 0x02000000;
const IMAGE_SCN_MEM_READ: u32 = 0x40000000;
const DEBUG_SECTION_CHARACTERISTICS: u32 =
    IMAGE_SCN_CNT_INITIALIZED_DATA | IMAGE_SCN_MEM_DISCARDABLE | IMAGE_SCN_MEM_READ;

fn pad4(buf: &mut Vec<u8>) {
    while buf.len() % 4 != 0 {
        buf.push(0);
    }
}

// ---------------------------------------------------------------------------
// CodeViewEmitter
// ---------------------------------------------------------------------------

/// Builds CodeView 4 sections from a debug sidecar and symbol table.
///
/// # Parameters
///
/// - `reader` — loaded [`DebugSidecarReader`].
/// - `image_base` — PE image base address.
/// - `symbol_table` — maps function name → byte offset from `code_rva`.
/// - `code_rva` — RVA of the `.text` section in the PE.
/// - `code_section_index` — 1-based section index for `.text` (default 1).
pub struct CodeViewEmitter<'a> {
    reader: &'a DebugSidecarReader,
    image_base: u64,
    symbol_table: &'a HashMap<String, u32>,
    code_rva: u32,
    code_section_index: u16,
}

impl<'a> CodeViewEmitter<'a> {
    /// Create a new emitter.
    pub fn new(
        reader: &'a DebugSidecarReader,
        image_base: u64,
        symbol_table: &'a HashMap<String, u32>,
        code_rva: u32,
        code_section_index: u16,
    ) -> Self {
        Self {
            reader,
            image_base,
            symbol_table,
            code_rva,
            code_section_index,
        }
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Build both CodeView sections.
    ///
    /// Returns a map with keys `.debug$S` and `.debug$T`.
    pub fn build(&self) -> HashMap<String, Vec<u8>> {
        let mut sections = HashMap::new();
        sections.insert(".debug$S".to_string(), self.build_debug_s());
        sections.insert(".debug$T".to_string(), self.build_debug_t());
        sections
    }

    /// Append `.debug$S` and `.debug$T` sections to a PE32+ file.
    ///
    /// Checks that there is space in the section header table for two new
    /// section headers.  Appends section data at the end of the file,
    /// aligned to `FileAlignment`.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the input is not a valid PE32+ binary or if there
    /// is insufficient header space.
    pub fn embed_in_pe(&self, pe_bytes: &[u8]) -> Result<Vec<u8>, String> {
        let mut data = pe_bytes.to_vec();

        if data.len() < 2 || &data[..2] != b"MZ" {
            return Err("not a valid PE file".into());
        }
        // The DOS stub stores the PE header offset at bytes 60-63.
        if data.len() < 64 {
            return Err("PE file too short to contain PE header offset".into());
        }
        let pe_off = u32::from_le_bytes(data[60..64].try_into().unwrap()) as usize;
        if data.len() < pe_off + 4 || &data[pe_off..pe_off + 4] != b"PE\x00\x00" {
            return Err("not a valid PE file (no PE signature)".into());
        }

        // COFF header starts 4 bytes past the PE signature (after "PE\x00\x00").
        let coff_off = pe_off + 4;
        // COFF header is 20 bytes; optional header starts at coff_off + 20.
        if data.len() < coff_off + 20 {
            return Err("PE file too short for COFF header".into());
        }
        let num_sections = u16::from_le_bytes(data[coff_off + 2..coff_off + 4].try_into().unwrap()) as usize;
        let opt_hdr_size = u16::from_le_bytes(data[coff_off + 16..coff_off + 18].try_into().unwrap()) as usize;

        let opt_off = coff_off + 20;
        // PE32+ optional header needs at least 64 bytes before SizeOfHeaders.
        if data.len() < opt_off + 64 {
            return Err("PE file too short for optional header fields".into());
        }
        let magic = u16::from_le_bytes(data[opt_off..opt_off + 2].try_into().unwrap());
        if magic != 0x020B {
            return Err("only PE32+ (64-bit) supported".into());
        }

        let file_alignment = u32::from_le_bytes(data[opt_off + 36..opt_off + 40].try_into().unwrap());
        let section_alignment = u32::from_le_bytes(data[opt_off + 32..opt_off + 36].try_into().unwrap());
        let size_of_headers = u32::from_le_bytes(data[opt_off + 60..opt_off + 64].try_into().unwrap()) as usize;

        let section_table_off = opt_off + opt_hdr_size;
        let section_table_size = num_sections
            .checked_mul(40)
            .ok_or("PE section table size overflow")?;
        let section_table_end = section_table_off
            .checked_add(section_table_size)
            .ok_or("PE section table end overflow")?;
        if section_table_end > data.len() {
            return Err(format!(
                "PE section table ({num_sections} sections) extends past end of file ({} bytes)",
                data.len()
            ));
        }

        // Guard against size_of_headers being smaller than what we already computed.
        let available = size_of_headers.saturating_sub(section_table_off + section_table_size);
        if available < 2 * 40 {
            return Err(format!(
                "insufficient PE header space for 2 new debug sections (need 80 bytes, have {available})"
            ));
        }

        // Find highest existing section for RVA calculation.
        let mut last_rva: u32 = 0;
        let mut last_vsize: u32 = 0;
        for i in 0..num_sections {
            let off = section_table_off + i * 40;
            // Each 40-byte section header entry was already bounds-checked above.
            let vsize = u32::from_le_bytes(data[off + 8..off + 12].try_into().unwrap());
            let rva = u32::from_le_bytes(data[off + 12..off + 16].try_into().unwrap());
            if rva >= last_rva {
                last_rva = rva;
                last_vsize = vsize;
            }
        }

        fn align_to(x: u32, n: u32) -> u32 { (x + n - 1) & !(n - 1) }

        let cv = self.build();
        let debug_s = &cv[".debug$S"];
        let debug_t = &cv[".debug$T"];

        let debug_s_rva = align_to(last_rva + last_vsize, section_alignment);
        let debug_t_rva = align_to(debug_s_rva + debug_s.len() as u32, section_alignment);

        let file_end = data.len() as u32;
        let debug_s_raw_start = align_to(file_end, file_alignment) as usize;
        let debug_s_raw_size = align_to(debug_s.len() as u32, file_alignment);
        let debug_t_raw_start = debug_s_raw_start + debug_s_raw_size as usize;
        let debug_t_raw_size = align_to(debug_t.len() as u32, file_alignment);

        let new_size_of_image = align_to(debug_t_rva + debug_t.len() as u32, section_alignment);

        fn make_section_header(
            name: &[u8],
            vsize: u32,
            rva: u32,
            raw_size: u32,
            raw_off: u32,
        ) -> Vec<u8> {
            let mut h = vec![0u8; 40];
            let name8 = &name[..8.min(name.len())];
            h[..name8.len()].copy_from_slice(name8);
            h[8..12].copy_from_slice(&vsize.to_le_bytes());
            h[12..16].copy_from_slice(&rva.to_le_bytes());
            h[16..20].copy_from_slice(&raw_size.to_le_bytes());
            h[20..24].copy_from_slice(&raw_off.to_le_bytes());
            h[36..40].copy_from_slice(&DEBUG_SECTION_CHARACTERISTICS.to_le_bytes());
            h
        }

        let new_sh_s_off = section_table_off + num_sections * 40;
        let new_sh_t_off = new_sh_s_off + 40;

        let sh_s = make_section_header(b".debug$S", debug_s.len() as u32, debug_s_rva, debug_s_raw_size, debug_s_raw_start as u32);
        let sh_t = make_section_header(b".debug$T", debug_t.len() as u32, debug_t_rva, debug_t_raw_size, debug_t_raw_start as u32);

        data[new_sh_s_off..new_sh_s_off + 40].copy_from_slice(&sh_s);
        data[new_sh_t_off..new_sh_t_off + 40].copy_from_slice(&sh_t);

        // Update COFF header: NumberOfSections += 2
        data[coff_off + 2..coff_off + 4].copy_from_slice(&((num_sections + 2) as u16).to_le_bytes());

        // Update Optional header: SizeOfImage
        data[opt_off + 56..opt_off + 60].copy_from_slice(&new_size_of_image.to_le_bytes());

        let mut result = data.clone();
        result.resize(debug_s_raw_start, 0);
        result.extend_from_slice(debug_s);
        result.resize(debug_t_raw_start, 0);
        result.extend_from_slice(debug_t);

        Ok(result)
    }

    // ------------------------------------------------------------------
    // Section builders
    // ------------------------------------------------------------------

    fn build_debug_s(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(&CV_SIGNATURE.to_le_bytes());

        let source_files = self.reader.source_files();

        // Build string name table.
        let mut name_table: Vec<u8> = Vec::new();
        let mut name_offsets: Vec<u32> = Vec::new();
        for path in source_files {
            name_offsets.push(name_table.len() as u32);
            name_table.extend_from_slice(path.as_bytes());
            name_table.push(0);
        }
        while name_table.len() % 4 != 0 { name_table.push(0); }

        // DEBUG_S_STRINGTABLE
        buf.extend_from_slice(&DEBUG_S_STRINGTABLE.to_le_bytes());
        buf.extend_from_slice(&(name_table.len() as u32).to_le_bytes());
        buf.extend_from_slice(&name_table);
        pad4(&mut buf);

        // DEBUG_S_FILECHKSMS
        let mut chksms_data: Vec<u8> = Vec::new();
        for off in &name_offsets {
            chksms_data.extend_from_slice(&off.to_le_bytes());
            chksms_data.push(0); // checksum_size = 0
            chksms_data.push(0); // kind = 0
            chksms_data.extend_from_slice(&0u16.to_le_bytes()); // padding
        }
        buf.extend_from_slice(&DEBUG_S_FILECHKSMS.to_le_bytes());
        buf.extend_from_slice(&(chksms_data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&chksms_data);
        pad4(&mut buf);

        // DEBUG_S_SYMBOLS: one S_GPROC32 + S_END per function.
        let mut sym_data: Vec<u8> = Vec::new();
        for fn_name in self.reader.function_names() {
            let fn_range = self.reader.function_range(fn_name);
            let fn_offset = self.symbol_table.get(fn_name).copied().unwrap_or(0);
            let fn_rva = self.code_rva + fn_offset;
            let fn_len = fn_range.map(|(s, e)| (e - s) as u32).unwrap_or(0);

            let mut name_bytes = fn_name.as_bytes().to_vec();
            name_bytes.push(0);
            while (2 + name_bytes.len()) % 4 != 0 { name_bytes.push(0); }

            // S_GPROC32 fixed payload (35 bytes):
            // parent(4)+end(4)+next(4)+len(4)+dbg_start(4)+dbg_end(4)+type_idx(4)+offset(4)+segment(2)+flags(1)
            let mut fixed: Vec<u8> = Vec::new();
            fixed.extend_from_slice(&0u32.to_le_bytes()); // parent
            fixed.extend_from_slice(&0u32.to_le_bytes()); // end
            fixed.extend_from_slice(&0u32.to_le_bytes()); // next
            fixed.extend_from_slice(&fn_len.to_le_bytes()); // proc_len
            fixed.extend_from_slice(&0u32.to_le_bytes()); // dbg_start
            fixed.extend_from_slice(&fn_len.to_le_bytes()); // dbg_end
            fixed.extend_from_slice(&0u32.to_le_bytes()); // type_index
            fixed.extend_from_slice(&fn_rva.to_le_bytes()); // offset (RVA)
            fixed.extend_from_slice(&self.code_section_index.to_le_bytes()); // segment
            fixed.push(0); // flags

            let record_len = (2 + fixed.len() + name_bytes.len()) as u16;
            sym_data.extend_from_slice(&record_len.to_le_bytes());
            sym_data.extend_from_slice(&S_GPROC32.to_le_bytes());
            sym_data.extend_from_slice(&fixed);
            sym_data.extend_from_slice(&name_bytes);

            // S_END record
            sym_data.extend_from_slice(&2u16.to_le_bytes());
            sym_data.extend_from_slice(&S_END.to_le_bytes());
        }
        buf.extend_from_slice(&DEBUG_S_SYMBOLS.to_le_bytes());
        buf.extend_from_slice(&(sym_data.len() as u32).to_le_bytes());
        buf.extend_from_slice(&sym_data);
        pad4(&mut buf);

        // DEBUG_S_LINES: per-function line mapping.
        for fn_name in self.reader.function_names() {
            let fn_range = self.reader.function_range(fn_name);
            let fn_offset = self.symbol_table.get(fn_name).copied().unwrap_or(0);
            let fn_rva = self.code_rva + fn_offset;
            let fn_len = fn_range.map(|(s, e)| (e - s) as u32).unwrap_or(0);

            let raw_rows = self.reader.raw_line_rows(fn_name);
            if raw_rows.is_empty() { continue; }

            // Group rows by file_id.
            let mut by_file: HashMap<usize, Vec<_>> = HashMap::new();
            for row in raw_rows {
                by_file.entry(row.file_id).or_default().push(row);
            }

            let mut lines_data: Vec<u8> = Vec::new();
            // Header: rva(4) + section_index(2) + flags(2) + code_size(4)
            lines_data.extend_from_slice(&fn_rva.to_le_bytes());
            lines_data.extend_from_slice(&self.code_section_index.to_le_bytes());
            lines_data.extend_from_slice(&0u16.to_le_bytes()); // flags
            lines_data.extend_from_slice(&fn_len.to_le_bytes());

            for (file_id, rows) in &by_file {
                let chksm_off = (*file_id as u32) * 8; // 8 bytes per entry
                let num_lines = rows.len() as u32;
                let block_size = 12 + num_lines * 8;
                lines_data.extend_from_slice(&chksm_off.to_le_bytes());
                lines_data.extend_from_slice(&num_lines.to_le_bytes());
                lines_data.extend_from_slice(&block_size.to_le_bytes());
                for row in rows {
                    let code_offset = row.instr_index as u32;
                    let line = (row.line & 0xFFFFFF) | (1 << 31); // is_stmt bit
                    lines_data.extend_from_slice(&code_offset.to_le_bytes());
                    lines_data.extend_from_slice(&line.to_le_bytes());
                }
            }

            buf.extend_from_slice(&DEBUG_S_LINES.to_le_bytes());
            buf.extend_from_slice(&(lines_data.len() as u32).to_le_bytes());
            buf.extend_from_slice(&lines_data);
            pad4(&mut buf);
        }

        buf
    }

    fn build_debug_t(&self) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(&CV_SIGNATURE.to_le_bytes());

        // LF_PROCEDURE (0x1008): return_type(u32=T_VOID=0x0003) + calling_conv(u8=0)
        //   + attrs(u8=0) + param_count(u16=0) + arg_list(u32=0)
        const LF_PROCEDURE: u16 = 0x1008;
        // record_data: return_type(4) + calling_conv(1) + attrs(1) + param_count(2) + arg_list(4) = 12 bytes
        let mut record_data: Vec<u8> = Vec::new();
        record_data.extend_from_slice(&0x0003u32.to_le_bytes()); // T_VOID
        record_data.push(0x00); // CV_CALL_NEARC
        record_data.push(0x00); // attrs
        record_data.extend_from_slice(&0u16.to_le_bytes()); // param_count
        record_data.extend_from_slice(&0u32.to_le_bytes()); // arg_list

        let record_len = (2 + record_data.len()) as u16;
        buf.extend_from_slice(&record_len.to_le_bytes());
        buf.extend_from_slice(&LF_PROCEDURE.to_le_bytes());
        buf.extend_from_slice(&record_data);
        pad4(&mut buf);

        buf
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use debug_sidecar::DebugSidecarWriter;

    fn make_reader() -> debug_sidecar::DebugSidecarReader {
        let mut w = DebugSidecarWriter::new();
        let fid = w.add_source_file("main.tetrad", b"");
        w.begin_function("main", 0, 0);
        w.record("main", 0, fid, 1, 1);
        w.record("main", 5, fid, 3, 1);
        w.end_function("main", 10);
        debug_sidecar::DebugSidecarReader::new(&w.finish()).unwrap()
    }

    fn empty_symtab() -> HashMap<String, u32> {
        let mut m = HashMap::new();
        m.insert("main".to_string(), 0u32);
        m
    }

    #[test]
    fn build_returns_two_sections() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = CodeViewEmitter::new(&reader, 0x140000000, &sym, 0x1000, 1);
        let sections = emitter.build();
        assert!(sections.contains_key(".debug$S"));
        assert!(sections.contains_key(".debug$T"));
    }

    #[test]
    fn debug_s_starts_with_cv_signature() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = CodeViewEmitter::new(&reader, 0x140000000, &sym, 0x1000, 1);
        let sections = emitter.build();
        let s = &sections[".debug$S"];
        assert_eq!(&s[..4], &CV_SIGNATURE.to_le_bytes());
    }

    #[test]
    fn debug_t_starts_with_cv_signature() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = CodeViewEmitter::new(&reader, 0x140000000, &sym, 0x1000, 1);
        let sections = emitter.build();
        let t = &sections[".debug$T"];
        assert_eq!(&t[..4], &CV_SIGNATURE.to_le_bytes());
    }

    #[test]
    fn debug_s_non_empty() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = CodeViewEmitter::new(&reader, 0x140000000, &sym, 0x1000, 1);
        let sections = emitter.build();
        assert!(sections[".debug$S"].len() > 4);
    }

    #[test]
    fn debug_t_non_empty() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = CodeViewEmitter::new(&reader, 0x140000000, &sym, 0x1000, 1);
        let sections = emitter.build();
        assert!(sections[".debug$T"].len() > 4);
    }

    #[test]
    fn embed_in_pe_rejects_non_pe() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = CodeViewEmitter::new(&reader, 0x140000000, &sym, 0x1000, 1);
        let result = emitter.embed_in_pe(b"not a PE");
        assert!(result.is_err());
    }

    #[test]
    fn empty_sidecar_build_succeeds() {
        let w = DebugSidecarWriter::new();
        let reader = debug_sidecar::DebugSidecarReader::new(&w.finish()).unwrap();
        let sym = HashMap::new();
        let emitter = CodeViewEmitter::new(&reader, 0, &sym, 0, 1);
        let sections = emitter.build();
        assert_eq!(sections.len(), 2);
    }
}
