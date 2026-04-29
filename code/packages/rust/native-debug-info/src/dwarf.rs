//! [`DwarfEmitter`] — builds DWARF 4 debug sections from a [`DebugSidecarReader`].
//!
//! Produces the minimal DWARF subset that lets gdb/lldb set breakpoints by
//! source line and show function names in stack traces:
//!
//! - `.debug_abbrev` — abbreviation table (defines DIE structure)
//! - `.debug_info`   — compilation unit + `DW_TAG_subprogram` DIEs
//! - `.debug_line`   — line number program (instr address → file/line/col)
//! - `.debug_str`    — deduplicated string pool
//!
//! The same four sections work for both ELF (Linux) and Mach-O/macOS — only
//! the embedding step differs (ELF section names vs `__DWARF` segment).

use std::collections::HashMap;

use debug_sidecar::DebugSidecarReader;

use crate::leb128::{encode_sleb128, encode_uleb128};

// ---------------------------------------------------------------------------
// DWARF 4 constants
// ---------------------------------------------------------------------------

const DW_TAG_COMPILE_UNIT: u64 = 0x11;
const DW_TAG_SUBPROGRAM: u64 = 0x2E;

const DW_CHILDREN_YES: u8 = 0x01;
const DW_CHILDREN_NO: u8 = 0x00;

const DW_AT_PRODUCER: u64 = 0x25;
const DW_AT_LANGUAGE: u64 = 0x13;
const DW_AT_NAME: u64 = 0x03;
const DW_AT_COMP_DIR: u64 = 0x1B;
const DW_AT_LOW_PC: u64 = 0x11;
const DW_AT_HIGH_PC: u64 = 0x12;
const DW_AT_STMT_LIST: u64 = 0x10;
const DW_AT_DECL_FILE: u64 = 0x3A;
const DW_AT_DECL_LINE: u64 = 0x3B;
const DW_AT_EXTERNAL: u64 = 0x3F;

const DW_FORM_ADDR: u64 = 0x01;
const DW_FORM_DATA1: u64 = 0x08;
const DW_FORM_DATA2: u64 = 0x05;
const DW_FORM_DATA4: u64 = 0x06;
const DW_FORM_DATA8: u64 = 0x07;
const DW_FORM_STRP: u64 = 0x0E;
const DW_FORM_FLAG_PRESENT: u64 = 0x19;
const DW_FORM_SEC_OFFSET: u64 = 0x17;

const DW_LANG_C99: u16 = 0x0001;

const DW_LNS_COPY: u8 = 0x01;
const DW_LNS_ADVANCE_PC: u8 = 0x02;
const DW_LNS_ADVANCE_LINE: u8 = 0x03;
const DW_LNS_SET_FILE: u8 = 0x04;
const DW_LNE_SET_ADDRESS: u8 = 0x02;
const DW_LNE_END_SEQUENCE: u8 = 0x01;

const PRODUCER: &str = "coding-adventures aot-core";

// ---------------------------------------------------------------------------
// DwarfEmitter
// ---------------------------------------------------------------------------

/// Builds DWARF 4 sections from a debug sidecar and a symbol table.
///
/// # Parameters
///
/// - `reader` — a loaded [`DebugSidecarReader`].
/// - `load_address` — virtual address at which the code is loaded.
/// - `symbol_table` — maps function name → byte offset from `load_address`.
/// - `code_size` — total byte size of all code (for `DW_AT_high_pc`).
pub struct DwarfEmitter<'a> {
    reader: &'a DebugSidecarReader,
    load_address: u64,
    symbol_table: &'a HashMap<String, u64>,
    code_size: u64,
}

impl<'a> DwarfEmitter<'a> {
    /// Create a new emitter.
    pub fn new(
        reader: &'a DebugSidecarReader,
        load_address: u64,
        symbol_table: &'a HashMap<String, u64>,
        code_size: u64,
    ) -> Self {
        Self { reader, load_address, symbol_table, code_size }
    }

    // ------------------------------------------------------------------
    // Public API
    // ------------------------------------------------------------------

    /// Build all four DWARF sections.
    ///
    /// Returns a map with keys `.debug_abbrev`, `.debug_info`,
    /// `.debug_line`, `.debug_str`.
    pub fn build(&self) -> HashMap<String, Vec<u8>> {
        let (str_table, str_offsets) = self.build_str_table();
        let debug_abbrev = self.build_abbrev();
        let debug_line = self.build_line();
        let debug_info = self.build_info(&str_offsets);
        let mut sections = HashMap::new();
        sections.insert(".debug_abbrev".to_string(), debug_abbrev);
        sections.insert(".debug_info".to_string(), debug_info);
        sections.insert(".debug_line".to_string(), debug_line);
        sections.insert(".debug_str".to_string(), str_table);
        sections
    }

    /// Append DWARF sections to an ELF64 little-endian binary.
    ///
    /// Appends `.debug_abbrev`, `.debug_info`, `.debug_line`, `.debug_str`
    /// as new ELF sections.  Updates the section header table and string table.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the input is not a valid ELF64 little-endian binary.
    pub fn embed_in_elf(&self, elf_bytes: &[u8]) -> Result<Vec<u8>, String> {
        let mut data = elf_bytes.to_vec();

        if data.len() < 4 || &data[..4] != b"\x7fELF" {
            return Err("not a valid ELF file".into());
        }
        if data[4] != 2 {
            return Err("only ELF64 supported".into());
        }
        if data[5] != 1 {
            return Err("only little-endian ELF supported".into());
        }

        // The ELF64 header is 64 bytes; we read fields at fixed offsets within it.
        if data.len() < 64 {
            return Err("ELF header too short (need at least 64 bytes)".into());
        }
        let e_shoff = u64::from_le_bytes(data[40..48].try_into().unwrap());
        let e_shnum = u16::from_le_bytes(data[60..62].try_into().unwrap()) as usize;
        let e_shstrndx = u16::from_le_bytes(data[62..64].try_into().unwrap()) as usize;

        const SHDR_SIZE: usize = 64;
        const SHT_PROGBITS: u32 = 1;

        // Validate section header table bounds before indexing.
        let shdr_table_start = e_shoff as usize;
        let shdr_table_end = shdr_table_start
            .checked_add(e_shnum.checked_mul(SHDR_SIZE).ok_or("ELF e_shnum overflow")?)
            .ok_or("ELF section header table offset overflow")?;
        if shdr_table_end > data.len() {
            return Err(format!(
                "ELF section header table (offset {shdr_table_start}, {e_shnum} entries) \
                 extends past end of file ({} bytes)",
                data.len()
            ));
        }
        if e_shstrndx >= e_shnum {
            return Err(format!(
                "ELF e_shstrndx ({e_shstrndx}) out of range (e_shnum = {e_shnum})"
            ));
        }

        // Read existing section headers.
        let mut shdrs: Vec<Vec<u8>> = (0..e_shnum)
            .map(|i| {
                let off = shdr_table_start + i * SHDR_SIZE;
                data[off..off + SHDR_SIZE].to_vec()
            })
            .collect();

        // Read the existing section-name string table (.shstrtab).
        let shstrtab_off = u64::from_le_bytes(shdrs[e_shstrndx][24..32].try_into().unwrap()) as usize;
        let shstrtab_size = u64::from_le_bytes(shdrs[e_shstrndx][32..40].try_into().unwrap()) as usize;
        let shstrtab_end = shstrtab_off
            .checked_add(shstrtab_size)
            .ok_or("ELF shstrtab offset overflow")?;
        if shstrtab_end > data.len() {
            return Err(format!(
                "ELF shstrtab (offset {shstrtab_off}, size {shstrtab_size}) \
                 extends past end of file ({} bytes)",
                data.len()
            ));
        }
        let mut shstrtab = data[shstrtab_off..shstrtab_end].to_vec();

        // Build DWARF sections.
        let dwarf = self.build();
        let sec_order = [".debug_abbrev", ".debug_info", ".debug_line", ".debug_str"];

        // Extend .shstrtab with new section names.
        let mut name_offsets: HashMap<&str, u32> = HashMap::new();
        for name in &sec_order {
            name_offsets.insert(name, shstrtab.len() as u32);
            shstrtab.extend_from_slice(name.as_bytes());
            shstrtab.push(0);
        }

        fn align8(x: usize) -> usize { x + (8 - x % 8) % 8 }

        // Append debug section data after current EOF.
        let mut result = data.clone();
        let mut debug_file_offsets: HashMap<&str, usize> = HashMap::new();
        for name in &sec_order {
            let pos = align8(result.len());
            result.resize(pos, 0);
            debug_file_offsets.insert(name, result.len());
            result.extend_from_slice(&dwarf[*name]);
        }

        // Append extended .shstrtab.
        let pos = align8(result.len());
        result.resize(pos, 0);
        let new_shstrtab_off = result.len();
        result.extend_from_slice(&shstrtab);

        // Build new section header table.
        let pos = align8(result.len());
        result.resize(pos, 0);
        let new_shdr_table_off = result.len();

        // Copy existing section headers, updating .shstrtab entry.
        for (i, shdr) in shdrs.iter().enumerate() {
            let mut s = shdr.clone();
            if i == e_shstrndx {
                s[24..32].copy_from_slice(&(new_shstrtab_off as u64).to_le_bytes());
                s[32..40].copy_from_slice(&(shstrtab.len() as u64).to_le_bytes());
            }
            result.extend_from_slice(&s);
        }

        // Write four new debug section headers.
        for name in &sec_order {
            let sec_data = &dwarf[*name];
            let mut shdr = vec![0u8; SHDR_SIZE];
            // sh_name (4), sh_type (4), sh_flags (8), sh_addr (8), sh_offset (8),
            // sh_size (8), sh_link (4), sh_info (4), sh_addralign (8), sh_entsize (8)
            shdr[0..4].copy_from_slice(&name_offsets[name].to_le_bytes());
            shdr[4..8].copy_from_slice(&SHT_PROGBITS.to_le_bytes());
            // sh_flags = 0, sh_addr = 0
            shdr[24..32].copy_from_slice(&(debug_file_offsets[name] as u64).to_le_bytes());
            shdr[32..40].copy_from_slice(&(sec_data.len() as u64).to_le_bytes());
            // sh_addralign = 1
            shdr[56..64].copy_from_slice(&1u64.to_le_bytes());
            result.extend_from_slice(&shdr);
        }

        // Update ELF header.
        result[40..48].copy_from_slice(&(new_shdr_table_off as u64).to_le_bytes());
        result[60..62].copy_from_slice(&((e_shnum + sec_order.len()) as u16).to_le_bytes());

        Ok(result)
    }

    /// Append a `__DWARF` segment with DWARF sections to a Mach-O 64-bit binary.
    ///
    /// Inserts a new `LC_SEGMENT_64` load command.  All existing section file
    /// offsets are shifted by the size of the new load command.
    ///
    /// # Errors
    ///
    /// Returns `Err` if the input is not a valid 64-bit little-endian Mach-O.
    pub fn embed_in_macho(&self, macho_bytes: &[u8]) -> Result<Vec<u8>, String> {
        let mut data = macho_bytes.to_vec();
        const MH_MAGIC_64: u32 = 0xFEEDFACF;
        if data.len() < 4 || u32::from_le_bytes(data[0..4].try_into().unwrap()) != MH_MAGIC_64 {
            return Err("not a valid 64-bit little-endian Mach-O".into());
        }

        let ncmds = u32::from_le_bytes(data[16..20].try_into().unwrap());
        let sizeofcmds = u32::from_le_bytes(data[20..24].try_into().unwrap());

        let dwarf = self.build();
        let sec_order = [".debug_abbrev", ".debug_info", ".debug_line", ".debug_str"];

        const LC_SEGMENT_64: u32 = 0x19;
        let new_lc_size: u32 = 72 + 4 * 80; // 392 bytes

        // The Mach-O 64-bit header is 32 bytes; load commands start immediately after.
        if data.len() < 32 {
            return Err("Mach-O header too short (need at least 32 bytes)".into());
        }

        // Shift existing segment file offsets.
        let mut lc_off = 32usize;
        for _ in 0..ncmds {
            // Minimum load command size is 8 bytes (cmd + cmdsize).
            if lc_off + 8 > data.len() {
                return Err(format!(
                    "Mach-O load command at offset {lc_off} extends past end of file"
                ));
            }
            let cmd = u32::from_le_bytes(data[lc_off..lc_off + 4].try_into().unwrap());
            let cmdsize = u32::from_le_bytes(data[lc_off + 4..lc_off + 8].try_into().unwrap()) as usize;
            // A valid cmdsize must be at least 8 and must not take us past EOF.
            if cmdsize < 8 {
                return Err(format!(
                    "Mach-O load command at offset {lc_off} has invalid cmdsize {cmdsize} (< 8)"
                ));
            }
            if lc_off + cmdsize > data.len() {
                return Err(format!(
                    "Mach-O load command at offset {lc_off} (cmdsize {cmdsize}) extends past \
                     end of file ({} bytes)",
                    data.len()
                ));
            }
            if cmd == LC_SEGMENT_64 {
                // LC_SEGMENT_64 header needs 72 bytes minimum before section entries.
                if lc_off + 72 > data.len() {
                    return Err("Mach-O LC_SEGMENT_64 too short for segment header".into());
                }
                let seg_foff = u64::from_le_bytes(data[lc_off + 40..lc_off + 48].try_into().unwrap());
                if seg_foff > 0 {
                    let new_off = seg_foff + new_lc_size as u64;
                    data[lc_off + 40..lc_off + 48].copy_from_slice(&new_off.to_le_bytes());
                }
                let nsects = u32::from_le_bytes(data[lc_off + 64..lc_off + 68].try_into().unwrap()) as usize;
                // Validate: section entries start at lc_off+72, each 80 bytes.
                let sect_table_end = lc_off + 72 + nsects * 80;
                if sect_table_end > data.len() {
                    return Err(format!(
                        "Mach-O LC_SEGMENT_64 section table ({nsects} sections) extends past \
                         end of file ({} bytes)",
                        data.len()
                    ));
                }
                for s in 0..nsects {
                    let off_field = lc_off + 72 + s * 80 + 48;
                    let sec_foff = u32::from_le_bytes(data[off_field..off_field + 4].try_into().unwrap());
                    if sec_foff > 0 {
                        let new_foff = sec_foff.checked_add(new_lc_size)
                            .ok_or("Mach-O section file offset overflow")?;
                        data[off_field..off_field + 4].copy_from_slice(&new_foff.to_le_bytes());
                    }
                    let reloff_field = lc_off + 72 + s * 80 + 56;
                    let reloff = u32::from_le_bytes(data[reloff_field..reloff_field + 4].try_into().unwrap());
                    if reloff > 0 {
                        let new_reloff = reloff.checked_add(new_lc_size)
                            .ok_or("Mach-O section reloff overflow")?;
                        data[reloff_field..reloff_field + 4].copy_from_slice(&new_reloff.to_le_bytes());
                    }
                }
            }
            lc_off += cmdsize;
        }

        // Debug section data goes after all existing content + new_lc_size shift.
        let debug_data_start = data.len() + new_lc_size as usize;
        let mut debug_offsets: HashMap<&str, u32> = HashMap::new();
        let mut pos = debug_data_start;
        for name in &sec_order {
            debug_offsets.insert(name, pos as u32);
            pos += dwarf[*name].len();
            pos = (pos + 3) & !3;
        }

        let new_lc = self.build_dwarf_lc(&debug_offsets, &dwarf, &sec_order);
        assert_eq!(new_lc.len(), new_lc_size as usize);

        // Update Mach-O header: ncmds + 1, sizeofcmds + new_lc_size.
        // Use checked arithmetic to detect overflow on malformed inputs.
        let new_ncmds = ncmds
            .checked_add(1)
            .ok_or("Mach-O ncmds overflow")?
            .to_le_bytes();
        let new_sizeofcmds = sizeofcmds
            .checked_add(new_lc_size)
            .ok_or("Mach-O sizeofcmds overflow")?
            .to_le_bytes();
        data[16..20].copy_from_slice(&new_ncmds);
        data[20..24].copy_from_slice(&new_sizeofcmds);

        // Assemble: [header+existing LCs] + [new LC] + [existing body] + [debug]
        let existing_data_start = 32 + sizeofcmds as usize;
        let mut result = Vec::new();
        result.extend_from_slice(&data[..existing_data_start]);
        result.extend_from_slice(&new_lc);
        result.extend_from_slice(&data[existing_data_start..]);

        for name in &sec_order {
            result.extend_from_slice(&dwarf[*name]);
            while result.len() % 4 != 0 {
                result.push(0);
            }
        }

        Ok(result)
    }

    // ------------------------------------------------------------------
    // Section builders
    // ------------------------------------------------------------------

    fn build_str_table(&self) -> (Vec<u8>, HashMap<String, u32>) {
        let mut ordered: Vec<String> = vec![PRODUCER.to_string(), String::new()];
        let files = self.reader.source_files();
        if files.is_empty() {
            ordered.push("<unknown>".to_string());
        } else {
            ordered.extend_from_slice(files);
        }
        ordered.extend(self.reader.function_names().iter().map(|s| s.to_string()));

        let mut offsets: HashMap<String, u32> = HashMap::new();
        let mut buf: Vec<u8> = Vec::new();
        for s in ordered {
            if !offsets.contains_key(&s) {
                offsets.insert(s.clone(), buf.len() as u32);
                buf.extend_from_slice(s.as_bytes());
                buf.push(0);
            }
        }
        (buf, offsets)
    }

    fn build_abbrev(&self) -> Vec<u8> {
        let mut buf = Vec::new();

        // Abbrev 1: DW_TAG_compile_unit, has children
        buf.extend_from_slice(&encode_uleb128(1));
        buf.extend_from_slice(&encode_uleb128(DW_TAG_COMPILE_UNIT));
        buf.push(DW_CHILDREN_YES);
        for (at, form) in [
            (DW_AT_PRODUCER,  DW_FORM_STRP),
            (DW_AT_LANGUAGE,  DW_FORM_DATA2),
            (DW_AT_NAME,      DW_FORM_STRP),
            (DW_AT_COMP_DIR,  DW_FORM_STRP),
            (DW_AT_LOW_PC,    DW_FORM_ADDR),
            (DW_AT_HIGH_PC,   DW_FORM_DATA8),
            (DW_AT_STMT_LIST, DW_FORM_SEC_OFFSET),
        ] {
            buf.extend_from_slice(&encode_uleb128(at));
            buf.extend_from_slice(&encode_uleb128(form));
        }
        buf.extend_from_slice(&[0x00, 0x00]);

        // Abbrev 2: DW_TAG_subprogram, no children
        buf.extend_from_slice(&encode_uleb128(2));
        buf.extend_from_slice(&encode_uleb128(DW_TAG_SUBPROGRAM));
        buf.push(DW_CHILDREN_NO);
        for (at, form) in [
            (DW_AT_NAME,      DW_FORM_STRP),
            (DW_AT_DECL_FILE, DW_FORM_DATA1),
            (DW_AT_DECL_LINE, DW_FORM_DATA4),
            (DW_AT_LOW_PC,    DW_FORM_ADDR),
            (DW_AT_HIGH_PC,   DW_FORM_DATA8),
            (DW_AT_EXTERNAL,  DW_FORM_FLAG_PRESENT),
        ] {
            buf.extend_from_slice(&encode_uleb128(at));
            buf.extend_from_slice(&encode_uleb128(form));
        }
        buf.extend_from_slice(&[0x00, 0x00]);

        buf.push(0); // end of abbreviation table
        buf
    }

    fn build_line(&self) -> Vec<u8> {
        let source_files = self.reader.source_files();
        let std_opcode_lengths: &[u8] = &[0, 1, 1, 1, 1, 0, 0, 0, 1, 0, 0, 1];

        let mut header_body: Vec<u8> = Vec::new();
        header_body.push(1);    // minimum_instruction_length
        header_body.push(1);    // maximum_ops_per_instruction
        header_body.push(1);    // default_is_stmt
        header_body.push(0xFB); // line_base = -5 (two's complement)
        header_body.push(14);   // line_range
        header_body.push(13);   // opcode_base (13 standard opcodes)
        header_body.extend_from_slice(std_opcode_lengths);
        header_body.push(0);    // include_directories: empty

        for path in source_files {
            header_body.extend_from_slice(path.as_bytes());
            header_body.push(0);
            header_body.extend_from_slice(&encode_uleb128(0)); // dir_index
            header_body.extend_from_slice(&encode_uleb128(0)); // mtime
            header_body.extend_from_slice(&encode_uleb128(0)); // file size
        }
        header_body.push(0); // file names table terminator

        let mut program: Vec<u8> = Vec::new();

        for fn_name in self.reader.function_names() {
            let fn_range = match self.reader.function_range(fn_name) {
                Some(r) => r,
                None => continue,
            };
            let fn_offset = self.symbol_table.get(fn_name).copied().unwrap_or(0);
            let fn_address = self.load_address + fn_offset;

            let raw_rows = self.reader.raw_line_rows(fn_name);
            if raw_rows.is_empty() {
                continue;
            }

            // DW_LNE_set_address
            program.push(0x00);
            program.extend_from_slice(&encode_uleb128(1 + 8));
            program.push(DW_LNE_SET_ADDRESS);
            program.extend_from_slice(&fn_address.to_le_bytes());

            let mut cur_line: u32 = 1;
            let mut cur_file_idx: u64 = 1;

            for row in raw_rows {
                let row_file_idx = row.file_id as u64 + 1;

                let pc_delta = row.instr_index;
                if pc_delta > 0 {
                    program.push(DW_LNS_ADVANCE_PC);
                    program.extend_from_slice(&encode_uleb128(pc_delta as u64));
                }

                if row_file_idx != cur_file_idx {
                    program.push(DW_LNS_SET_FILE);
                    program.extend_from_slice(&encode_uleb128(row_file_idx));
                    cur_file_idx = row_file_idx;
                }

                let line_delta = row.line as i64 - cur_line as i64;
                if line_delta != 0 {
                    program.push(DW_LNS_ADVANCE_LINE);
                    program.extend_from_slice(&encode_sleb128(line_delta));
                    cur_line = row.line;
                }

                // Reset PC to function start for next row.
                if pc_delta > 0 {
                    program.push(DW_LNS_ADVANCE_PC);
                    let neg_delta = (-(pc_delta as i64)) as u64;
                    program.extend_from_slice(&encode_uleb128(neg_delta));
                }

                program.push(DW_LNS_COPY);
            }

            // DW_LNE_end_sequence
            program.push(0x00);
            program.extend_from_slice(&encode_uleb128(1));
            program.push(DW_LNE_END_SEQUENCE);

            let _ = fn_range; // used for context; fn_address was computed from it above
        }

        let header_length = header_body.len() as u32;
        let unit_length = 2 + 4 + header_length + program.len() as u32;

        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(&unit_length.to_le_bytes());
        buf.extend_from_slice(&4u16.to_le_bytes()); // version
        buf.extend_from_slice(&header_length.to_le_bytes());
        buf.extend_from_slice(&header_body);
        buf.extend_from_slice(&program);
        buf
    }

    fn build_info(&self, str_offsets: &HashMap<String, u32>) -> Vec<u8> {
        let source_files = self.reader.source_files();
        let primary_file = source_files.first().map(|s| s.as_str()).unwrap_or("<unknown>");

        let mut dies: Vec<u8> = Vec::new();

        // Compile unit DIE (abbrev 1).
        dies.extend_from_slice(&encode_uleb128(1));
        dies.extend_from_slice(&str_offsets.get(PRODUCER).copied().unwrap_or(0).to_le_bytes());
        dies.extend_from_slice(&DW_LANG_C99.to_le_bytes());
        dies.extend_from_slice(&str_offsets.get(primary_file).copied().unwrap_or(0).to_le_bytes());
        dies.extend_from_slice(&str_offsets.get("").copied().unwrap_or(0).to_le_bytes());
        dies.extend_from_slice(&self.load_address.to_le_bytes());
        dies.extend_from_slice(&self.code_size.to_le_bytes());
        dies.extend_from_slice(&0u32.to_le_bytes()); // DW_AT_stmt_list → 0

        // One DW_TAG_subprogram per function.
        for fn_name in self.reader.function_names() {
            let fn_range = self.reader.function_range(fn_name);
            let fn_offset = self.symbol_table.get(fn_name).copied().unwrap_or(0);
            let fn_address = self.load_address + fn_offset;

            let raw_rows = self.reader.raw_line_rows(fn_name);
            let decl_line = raw_rows.first().map(|r| r.line).unwrap_or(1);
            let decl_file_idx = raw_rows.first().map(|r| r.file_id as u8 + 1).unwrap_or(1);

            let fn_byte_len = fn_range.map(|(s, e)| (e - s) as u64).unwrap_or(0);

            dies.extend_from_slice(&encode_uleb128(2)); // abbrev 2
            dies.extend_from_slice(&str_offsets.get(fn_name).copied().unwrap_or(0).to_le_bytes());
            dies.push(decl_file_idx);
            dies.extend_from_slice(&decl_line.to_le_bytes());
            dies.extend_from_slice(&fn_address.to_le_bytes());
            dies.extend_from_slice(&fn_byte_len.to_le_bytes());
            // DW_AT_external with DW_FORM_flag_present occupies 0 bytes
        }

        dies.push(0); // end of children (compile unit)

        // Compile unit header: unit_length(4) + version(2) + abbrev_off(4) + addr_size(1)
        let unit_length = 2u32 + 4 + 1 + dies.len() as u32;
        let mut buf: Vec<u8> = Vec::new();
        buf.extend_from_slice(&unit_length.to_le_bytes());
        buf.extend_from_slice(&4u16.to_le_bytes()); // version
        buf.extend_from_slice(&0u32.to_le_bytes()); // abbrev offset
        buf.push(8); // address size = 8 bytes
        buf.extend_from_slice(&dies);
        buf
    }

    fn build_dwarf_lc(
        &self,
        debug_offsets: &HashMap<&str, u32>,
        dwarf: &HashMap<String, Vec<u8>>,
        sec_order: &[&str],
    ) -> Vec<u8> {
        const LC_SEGMENT_64: u32 = 0x19;
        let n_sects = sec_order.len();
        let cmdsize = (72 + n_sects * 80) as u32;
        let total_size: u32 = sec_order.iter().map(|n| dwarf[*n].len() as u32).sum();
        let seg_fileoff = debug_offsets[sec_order[0]];

        let mut buf: Vec<u8> = Vec::new();

        // LC_SEGMENT_64 header (72 bytes)
        buf.extend_from_slice(&LC_SEGMENT_64.to_le_bytes());
        buf.extend_from_slice(&cmdsize.to_le_bytes());
        // segname: "__DWARF\0\0\0\0\0\0\0\0\0" (16 bytes)
        let segname = b"__DWARF\x00\x00\x00\x00\x00\x00\x00\x00\x00";
        buf.extend_from_slice(segname);
        buf.extend_from_slice(&0u64.to_le_bytes());         // vmaddr
        buf.extend_from_slice(&(total_size as u64).to_le_bytes()); // vmsize
        buf.extend_from_slice(&(seg_fileoff as u64).to_le_bytes()); // fileoff
        buf.extend_from_slice(&(total_size as u64).to_le_bytes()); // filesize
        buf.extend_from_slice(&7u32.to_le_bytes());  // maxprot (RWX)
        buf.extend_from_slice(&5u32.to_le_bytes());  // initprot (RX)
        buf.extend_from_slice(&(n_sects as u32).to_le_bytes()); // nsects
        buf.extend_from_slice(&0u32.to_le_bytes());  // flags

        assert_eq!(buf.len(), 72);

        // section_64 entries (80 bytes each)
        let macho_name: HashMap<&str, &[u8]> = [
            (".debug_abbrev", b"__debug_abbrev\x00\x00" as &[u8]),
            (".debug_info",   b"__debug_info\x00\x00\x00\x00" as &[u8]),
            (".debug_line",   b"__debug_line\x00\x00\x00\x00" as &[u8]),
            (".debug_str",    b"__debug_str\x00\x00\x00\x00\x00" as &[u8]),
        ].into_iter().collect();

        for name in sec_order {
            let mname = macho_name[name];
            let sec_data = &dwarf[*name];
            buf.extend_from_slice(mname);
            buf.extend_from_slice(segname);
            buf.extend_from_slice(&0u64.to_le_bytes());  // addr
            buf.extend_from_slice(&(sec_data.len() as u64).to_le_bytes()); // size
            buf.extend_from_slice(&debug_offsets[name].to_le_bytes());     // offset
            buf.extend_from_slice(&0u32.to_le_bytes());  // align
            buf.extend_from_slice(&0u32.to_le_bytes());  // reloff
            buf.extend_from_slice(&0u32.to_le_bytes());  // nreloc
            buf.extend_from_slice(&0x02000000u32.to_le_bytes()); // flags: S_ATTR_DEBUG
            buf.extend_from_slice(&0u32.to_le_bytes());  // reserved1
            buf.extend_from_slice(&0u32.to_le_bytes());  // reserved2
            buf.extend_from_slice(&0u32.to_le_bytes());  // reserved3
        }

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
        let fid = w.add_source_file("fib.tetrad", b"");
        w.begin_function("fib", 0, 1);
        w.record("fib", 0, fid, 3, 1);
        w.record("fib", 5, fid, 5, 1);
        w.end_function("fib", 10);
        debug_sidecar::DebugSidecarReader::new(&w.finish()).unwrap()
    }

    fn empty_symtab() -> HashMap<String, u64> {
        let mut m = HashMap::new();
        m.insert("fib".to_string(), 0);
        m
    }

    #[test]
    fn build_returns_four_sections() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = DwarfEmitter::new(&reader, 0x400000, &sym, 256);
        let sections = emitter.build();
        assert!(sections.contains_key(".debug_abbrev"));
        assert!(sections.contains_key(".debug_info"));
        assert!(sections.contains_key(".debug_line"));
        assert!(sections.contains_key(".debug_str"));
    }

    #[test]
    fn debug_abbrev_non_empty() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = DwarfEmitter::new(&reader, 0x400000, &sym, 256);
        let sections = emitter.build();
        assert!(!sections[".debug_abbrev"].is_empty());
    }

    #[test]
    fn debug_info_non_empty() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = DwarfEmitter::new(&reader, 0x400000, &sym, 256);
        let sections = emitter.build();
        assert!(!sections[".debug_info"].is_empty());
    }

    #[test]
    fn debug_line_non_empty() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = DwarfEmitter::new(&reader, 0x400000, &sym, 256);
        let sections = emitter.build();
        assert!(!sections[".debug_line"].is_empty());
    }

    #[test]
    fn debug_str_contains_producer() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = DwarfEmitter::new(&reader, 0x400000, &sym, 256);
        let sections = emitter.build();
        let str_sec = &sections[".debug_str"];
        assert!(str_sec.windows(PRODUCER.len())
            .any(|w| w == PRODUCER.as_bytes()));
    }

    #[test]
    fn debug_str_contains_function_name() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = DwarfEmitter::new(&reader, 0x400000, &sym, 256);
        let sections = emitter.build();
        let str_sec = &sections[".debug_str"];
        assert!(str_sec.windows(3).any(|w| w == b"fib"));
    }

    #[test]
    fn embed_in_elf_rejects_non_elf() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = DwarfEmitter::new(&reader, 0x400000, &sym, 256);
        let result = emitter.embed_in_elf(b"not an elf");
        assert!(result.is_err());
    }

    #[test]
    fn embed_in_macho_rejects_non_macho() {
        let reader = make_reader();
        let sym = empty_symtab();
        let emitter = DwarfEmitter::new(&reader, 0x400000, &sym, 256);
        let result = emitter.embed_in_macho(b"not a macho");
        assert!(result.is_err());
    }

    #[test]
    fn empty_sidecar_build_succeeds() {
        let w = DebugSidecarWriter::new();
        let reader = debug_sidecar::DebugSidecarReader::new(&w.finish()).unwrap();
        let sym = HashMap::new();
        let emitter = DwarfEmitter::new(&reader, 0, &sym, 0);
        let sections = emitter.build();
        // All sections exist even for empty sidecars.
        assert_eq!(sections.len(), 4);
    }
}
