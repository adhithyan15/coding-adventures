//! # compiler-source-map — Source-mapping sidecar for the AOT compiler pipeline.
//!
//! This crate provides the source-mapping sidecar that flows through every
//! stage of the AOT compiler pipeline. It allows any compiler error or
//! debugging event to be traced back to the original source position.
//!
//! ## Why a "chain" instead of a flat table?
//!
//! A flat table (machine-code offset → source position) works for the final
//! consumer — a debugger, profiler, or error reporter. But it doesn't help
//! when you're debugging the *compiler itself*:
//!
//! - "Why did the optimiser delete instruction #42?"
//!   → Look at the `IrToIr` segment for that pass.
//!
//! - "Which AST node produced this IR instruction?"
//!   → Look at `AstToIr`.
//!
//! - "The machine code for this instruction seems wrong — what IR produced it?"
//!   → Look at `IrToMachineCode` in reverse.
//!
//! The chain makes the compiler pipeline **transparent and debuggable at every
//! stage**. The flat composite mapping is just the composition of all segments.
//!
//! ## Segment overview
//!
//! ```text
//! Segment 1: SourceToAst      — source text position  → AST node ID
//! Segment 2: AstToIr          — AST node ID           → IR instruction IDs
//! Segment 3: IrToIr           — IR instruction ID     → optimised IR instruction IDs
//!                               (one segment per optimiser pass)
//! Segment 4: IrToMachineCode  — IR instruction ID     → machine code byte offset + length
//!
//! Composite: source position → machine code offset  (forward)
//!            machine code offset → source position   (reverse)
//! ```

use std::fmt;
use std::collections::HashSet;

// ===========================================================================
// SourcePosition — a span of characters in a source file
// ===========================================================================

/// A span of characters in a source file.
///
/// Think of this as a "highlighter pen" marking a region of source code.
/// The `(line, column)` pair marks the start; `length` tells you how many
/// characters are highlighted. For Brainfuck, every command is exactly
/// one character (`length = 1`). For BASIC, a keyword like `PRINT` would
/// have `length = 5`.
///
/// # Example
///
/// ```
/// use compiler_source_map::SourcePosition;
/// let pos = SourcePosition { file: "hello.bf".to_string(), line: 1, column: 3, length: 1 };
/// assert_eq!(pos.to_string(), "hello.bf:1:3 (len=1)");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourcePosition {
    /// Source file path (e.g., `"hello.bf"`).
    pub file: String,
    /// 1-based line number.
    pub line: usize,
    /// 1-based column number.
    pub column: usize,
    /// Character span in source.
    pub length: usize,
}

impl fmt::Display for SourcePosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}:{} (len={})", self.file, self.line, self.column, self.length)
    }
}

// ===========================================================================
// Segment 1: SourceToAst — source positions → AST node IDs
// ===========================================================================

/// One entry in the `SourceToAst` segment.
///
/// Maps a source position to the AST node that represents it. For example,
/// the `"+"` character at line 1, column 3 of `"hello.bf"` maps to
/// AST node #42 (a `command(INC)` node in the parse tree).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceToAstEntry {
    /// The source position (file, line, column, length).
    pub pos: SourcePosition,
    /// The AST node ID this source position maps to.
    pub ast_node_id: usize,
}

/// Segment 1: maps source text positions to AST node IDs.
///
/// Produced by the parser or the language-specific frontend (e.g.,
/// `brainfuck-ir-compiler`). Maps every meaningful source position to
/// the AST node that represents it.
#[derive(Debug, Clone, Default)]
pub struct SourceToAst {
    /// All source → AST mappings.
    pub entries: Vec<SourceToAstEntry>,
}

impl SourceToAst {
    /// Create an empty `SourceToAst` segment.
    pub fn new() -> Self {
        SourceToAst { entries: Vec::new() }
    }

    /// Record a mapping from a source position to an AST node ID.
    pub fn add(&mut self, pos: SourcePosition, ast_node_id: usize) {
        self.entries.push(SourceToAstEntry { pos, ast_node_id });
    }

    /// Return the source position for the given AST node ID, or `None` if
    /// not found. This is used for reverse lookups.
    pub fn lookup_by_node_id(&self, ast_node_id: usize) -> Option<&SourcePosition> {
        self.entries.iter()
            .find(|e| e.ast_node_id == ast_node_id)
            .map(|e| &e.pos)
    }
}

// ===========================================================================
// Segment 2: AstToIr — AST node IDs → IR instruction IDs
// ===========================================================================

/// One entry in the `AstToIr` segment.
///
/// A single AST node often produces multiple IR instructions. For example,
/// a Brainfuck `"+"` command produces four instructions:
/// `LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE`.
/// So the mapping is one-to-many: `ast_node_42 → [ir_7, ir_8, ir_9, ir_10]`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstToIrEntry {
    /// The AST node that produced these IR instructions.
    pub ast_node_id: usize,
    /// The IR instruction IDs this AST node produced.
    pub ir_ids: Vec<i64>,
}

/// Segment 2: maps AST node IDs to IR instruction IDs.
#[derive(Debug, Clone, Default)]
pub struct AstToIr {
    /// All AST → IR mappings.
    pub entries: Vec<AstToIrEntry>,
}

impl AstToIr {
    /// Create an empty `AstToIr` segment.
    pub fn new() -> Self {
        AstToIr { entries: Vec::new() }
    }

    /// Record that the given AST node produced the given IR instruction IDs.
    pub fn add(&mut self, ast_node_id: usize, ir_ids: Vec<i64>) {
        self.entries.push(AstToIrEntry { ast_node_id, ir_ids });
    }

    /// Return the IR instruction IDs for the given AST node, or `None` if
    /// not found.
    pub fn lookup_by_ast_node_id(&self, ast_node_id: usize) -> Option<&[i64]> {
        self.entries.iter()
            .find(|e| e.ast_node_id == ast_node_id)
            .map(|e| e.ir_ids.as_slice())
    }

    /// Return the AST node ID that produced the given IR instruction, or `None`
    /// if not found. Used for reverse lookups.
    pub fn lookup_by_ir_id(&self, ir_id: i64) -> Option<usize> {
        for entry in &self.entries {
            if entry.ir_ids.contains(&ir_id) {
                return Some(entry.ast_node_id);
            }
        }
        None
    }
}

// ===========================================================================
// Segment 3: IrToIr — original IR IDs → optimised IR IDs
// ===========================================================================

/// One entry in an `IrToIr` segment.
///
/// Three cases:
/// 1. **Preserved**: `original_id → [same_id]` (instruction unchanged)
/// 2. **Replaced**: `original_id → [new_id_1, ...]` (instruction split/transformed)
/// 3. **Deleted**: `original_id` is in `deleted` set (instruction optimised away)
///
/// # Example
///
/// A contraction pass folds three `ADD_IMM 1` instructions (IDs 7, 8, 9)
/// into one `ADD_IMM 3` (ID 100):
/// ```text
/// 7 → [100], 8 → [100], 9 → [100]
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrToIrEntry {
    /// The original IR instruction ID.
    pub original_id: i64,
    /// The new IR instruction IDs (empty if deleted).
    pub new_ids: Vec<i64>,
}

/// Segment 3: maps IR instruction IDs to optimised IR instruction IDs.
///
/// One segment is produced per optimiser pass. The `pass_name` field
/// identifies which pass produced this mapping (e.g., `"identity"`,
/// `"contraction"`, `"clear_loop"`, `"dead_store"`).
#[derive(Debug, Clone)]
pub struct IrToIr {
    /// All original → new ID mappings.
    pub entries: Vec<IrToIrEntry>,
    /// IDs that were optimised away.
    pub deleted: HashSet<i64>,
    /// Which optimiser pass produced this segment.
    pub pass_name: String,
}

impl IrToIr {
    /// Create a new `IrToIr` segment for the named pass.
    pub fn new(pass_name: &str) -> Self {
        IrToIr {
            entries: Vec::new(),
            deleted: HashSet::new(),
            pass_name: pass_name.to_string(),
        }
    }

    /// Record that the original instruction was replaced by the new ones.
    pub fn add_mapping(&mut self, original_id: i64, new_ids: Vec<i64>) {
        self.entries.push(IrToIrEntry { original_id, new_ids });
    }

    /// Record that the original instruction was deleted.
    pub fn add_deletion(&mut self, original_id: i64) {
        self.deleted.insert(original_id);
        self.entries.push(IrToIrEntry { original_id, new_ids: Vec::new() });
    }

    /// Return the new IDs for the given original ID, or `None` if deleted
    /// or not found.
    pub fn lookup_by_original_id(&self, original_id: i64) -> Option<&[i64]> {
        if self.deleted.contains(&original_id) {
            return None;
        }
        self.entries.iter()
            .find(|e| e.original_id == original_id)
            .map(|e| e.new_ids.as_slice())
    }

    /// Return the original ID that produced the given new ID, or `None` if
    /// not found. When multiple originals map to the same new ID (e.g.,
    /// contraction), this returns the first one found.
    pub fn lookup_by_new_id(&self, new_id: i64) -> Option<i64> {
        for entry in &self.entries {
            if entry.new_ids.contains(&new_id) {
                return Some(entry.original_id);
            }
        }
        None
    }
}

// ===========================================================================
// Segment 4: IrToMachineCode — IR IDs → machine code byte offsets
// ===========================================================================

/// One entry in the `IrToMachineCode` segment.
///
/// Each entry is a triple: `(ir_id, mc_offset, mc_length)`. For example,
/// a `LOAD_BYTE` IR instruction might produce 8 bytes of RISC-V machine
/// code starting at offset `0x14` in the `.text` section.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrToMachineCodeEntry {
    /// IR instruction ID.
    pub ir_id: i64,
    /// Byte offset in the `.text` section.
    pub mc_offset: usize,
    /// Number of bytes of machine code.
    pub mc_length: usize,
}

/// Segment 4: maps IR instruction IDs to machine code byte offsets.
///
/// Filled by the code generation backend (e.g., `codegen-riscv`).
/// Until the backend runs, this field is `None` in the `SourceMapChain`.
#[derive(Debug, Clone, Default)]
pub struct IrToMachineCode {
    /// All IR → machine code mappings.
    pub entries: Vec<IrToMachineCodeEntry>,
}

impl IrToMachineCode {
    /// Create an empty `IrToMachineCode` segment.
    pub fn new() -> Self {
        IrToMachineCode { entries: Vec::new() }
    }

    /// Record that the given IR instruction produced machine code at the
    /// given offset with the given length.
    pub fn add(&mut self, ir_id: i64, mc_offset: usize, mc_length: usize) {
        self.entries.push(IrToMachineCodeEntry { ir_id, mc_offset, mc_length });
    }

    /// Return the machine code `(offset, length)` for the given IR
    /// instruction ID, or `None` if not found.
    pub fn lookup_by_ir_id(&self, ir_id: i64) -> Option<(usize, usize)> {
        self.entries.iter()
            .find(|e| e.ir_id == ir_id)
            .map(|e| (e.mc_offset, e.mc_length))
    }

    /// Return the IR instruction ID whose machine code contains the given
    /// byte offset, or `None` if not found.
    ///
    /// A machine code offset "contains" an IR instruction if:
    /// `entry.mc_offset <= offset < entry.mc_offset + entry.mc_length`
    pub fn lookup_by_mc_offset(&self, offset: usize) -> Option<i64> {
        self.entries.iter()
            .find(|e| offset >= e.mc_offset && offset < e.mc_offset + e.mc_length)
            .map(|e| e.ir_id)
    }
}

// ===========================================================================
// SourceMapChain — the full pipeline sidecar
// ===========================================================================

/// The full source map chain: the central data structure that flows through
/// every stage of the compiler pipeline.
///
/// Each stage reads the existing segments and appends its own:
///
/// 1. **Frontend** (`brainfuck-ir-compiler`) → fills `source_to_ast` + `ast_to_ir`
/// 2. **Optimiser** (`compiler-ir-optimizer`) → appends `ir_to_ir` segments
/// 3. **Backend** (`codegen-riscv`) → fills `ir_to_machine_code`
///
/// ## Composite queries
///
/// The chain supports two end-to-end queries:
/// - `source_to_mc(pos)` — forward: source position → machine code offsets
/// - `mc_to_source(offset)` — reverse: machine code offset → source position
#[derive(Debug, Clone)]
pub struct SourceMapChain {
    /// Segment 1: source positions → AST node IDs.
    pub source_to_ast: SourceToAst,
    /// Segment 2: AST node IDs → IR instruction IDs.
    pub ast_to_ir: AstToIr,
    /// Segment 3: one entry per optimiser pass (IR IDs → optimised IR IDs).
    pub ir_to_ir: Vec<IrToIr>,
    /// Segment 4: IR IDs → machine code offsets (filled by backend).
    pub ir_to_machine_code: Option<IrToMachineCode>,
}

impl SourceMapChain {
    /// Create an empty source map chain ready for use.
    pub fn new() -> Self {
        SourceMapChain {
            source_to_ast: SourceToAst::new(),
            ast_to_ir: AstToIr::new(),
            ir_to_ir: Vec::new(),
            ir_to_machine_code: None,
        }
    }

    /// Append an `IrToIr` segment from an optimiser pass.
    pub fn add_optimizer_pass(&mut self, segment: IrToIr) {
        self.ir_to_ir.push(segment);
    }

    // ── Composite queries ──────────────────────────────────────────────────

    /// Compose all segments to look up the machine code offset(s) for a
    /// given source position.
    ///
    /// Returns `None` if the chain is incomplete or no mapping exists.
    ///
    /// ## Algorithm
    ///
    /// 1. `SourceToAst`: source position → AST node ID
    /// 2. `AstToIr`: AST node ID → IR instruction IDs
    /// 3. `IrToIr` (each pass): follow IR IDs through each optimiser pass
    /// 4. `IrToMachineCode`: final IR IDs → machine code offsets
    pub fn source_to_mc(&self, pos: &SourcePosition) -> Option<Vec<IrToMachineCodeEntry>> {
        let mc = self.ir_to_machine_code.as_ref()?;

        // Step 1: source position → AST node ID
        let ast_node_id = self.source_to_ast.entries.iter()
            .find(|e| e.pos.file == pos.file
                && e.pos.line == pos.line
                && e.pos.column == pos.column)
            .map(|e| e.ast_node_id)?;

        // Step 2: AST node ID → IR instruction IDs
        let ir_ids = self.ast_to_ir.lookup_by_ast_node_id(ast_node_id)?;
        let mut current_ids: Vec<i64> = ir_ids.to_vec();

        // Step 3: follow through optimiser passes
        for pass in &self.ir_to_ir {
            let mut next_ids = Vec::new();
            for &id in &current_ids {
                if pass.deleted.contains(&id) {
                    continue; // instruction was optimised away
                }
                if let Some(new_ids) = pass.lookup_by_original_id(id) {
                    next_ids.extend_from_slice(new_ids);
                }
            }
            current_ids = next_ids;
        }

        if current_ids.is_empty() {
            return None;
        }

        // Step 4: IR IDs → machine code
        let results: Vec<IrToMachineCodeEntry> = current_ids.iter()
            .filter_map(|&id| {
                mc.lookup_by_ir_id(id).map(|(offset, length)| {
                    IrToMachineCodeEntry { ir_id: id, mc_offset: offset, mc_length: length }
                })
            })
            .collect();

        if results.is_empty() { None } else { Some(results) }
    }

    /// Compose all segments in reverse to look up the source position for a
    /// given machine code offset.
    ///
    /// Returns `None` if the chain is incomplete or no mapping exists.
    ///
    /// ## Algorithm (reverse of `source_to_mc`)
    ///
    /// 1. `IrToMachineCode`: MC offset → IR instruction ID
    /// 2. `IrToIr` (each pass, in reverse): follow IR ID back through passes
    /// 3. `AstToIr`: IR ID → AST node ID
    /// 4. `SourceToAst`: AST node ID → source position
    pub fn mc_to_source(&self, mc_offset: usize) -> Option<&SourcePosition> {
        let mc = self.ir_to_machine_code.as_ref()?;

        // Step 1: MC offset → IR ID
        let mut current_id = mc.lookup_by_mc_offset(mc_offset)?;

        // Step 2: follow back through optimiser passes (in reverse order)
        for pass in self.ir_to_ir.iter().rev() {
            current_id = pass.lookup_by_new_id(current_id)?;
        }

        // Step 3: IR ID → AST node ID
        let ast_node_id = self.ast_to_ir.lookup_by_ir_id(current_id)?;

        // Step 4: AST node ID → source position
        self.source_to_ast.lookup_by_node_id(ast_node_id)
    }
}

impl Default for SourceMapChain {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // ── SourcePosition ─────────────────────────────────────────────────────

    #[test]
    fn test_source_position_display() {
        let pos = SourcePosition {
            file: "hello.bf".to_string(),
            line: 1,
            column: 3,
            length: 1,
        };
        assert_eq!(pos.to_string(), "hello.bf:1:3 (len=1)");
    }

    #[test]
    fn test_source_position_equality() {
        let pos1 = SourcePosition { file: "a.bf".to_string(), line: 1, column: 1, length: 1 };
        let pos2 = SourcePosition { file: "a.bf".to_string(), line: 1, column: 1, length: 1 };
        let pos3 = SourcePosition { file: "a.bf".to_string(), line: 2, column: 1, length: 1 };
        assert_eq!(pos1, pos2);
        assert_ne!(pos1, pos3);
    }

    // ── SourceToAst ────────────────────────────────────────────────────────

    #[test]
    fn test_source_to_ast_add_and_lookup() {
        let mut s2a = SourceToAst::new();
        let pos = SourcePosition { file: "hello.bf".to_string(), line: 1, column: 1, length: 1 };
        s2a.add(pos.clone(), 42);
        let found = s2a.lookup_by_node_id(42).unwrap();
        assert_eq!(found, &pos);
    }

    #[test]
    fn test_source_to_ast_not_found() {
        let s2a = SourceToAst::new();
        assert!(s2a.lookup_by_node_id(999).is_none());
    }

    #[test]
    fn test_source_to_ast_multiple_entries() {
        let mut s2a = SourceToAst::new();
        s2a.add(SourcePosition { file: "x.bf".to_string(), line: 1, column: 1, length: 1 }, 0);
        s2a.add(SourcePosition { file: "x.bf".to_string(), line: 1, column: 2, length: 1 }, 1);
        assert!(s2a.lookup_by_node_id(0).is_some());
        assert!(s2a.lookup_by_node_id(1).is_some());
        assert_eq!(s2a.lookup_by_node_id(0).unwrap().column, 1);
        assert_eq!(s2a.lookup_by_node_id(1).unwrap().column, 2);
    }

    // ── AstToIr ───────────────────────────────────────────────────────────

    #[test]
    fn test_ast_to_ir_add_and_lookup() {
        let mut a2i = AstToIr::new();
        a2i.add(42, vec![7, 8, 9, 10]);
        let ids = a2i.lookup_by_ast_node_id(42).unwrap();
        assert_eq!(ids, &[7, 8, 9, 10]);
    }

    #[test]
    fn test_ast_to_ir_lookup_by_ir_id() {
        let mut a2i = AstToIr::new();
        a2i.add(42, vec![7, 8, 9, 10]);
        assert_eq!(a2i.lookup_by_ir_id(8), Some(42));
        assert_eq!(a2i.lookup_by_ir_id(99), None);
    }

    #[test]
    fn test_ast_to_ir_not_found() {
        let a2i = AstToIr::new();
        assert!(a2i.lookup_by_ast_node_id(0).is_none());
        assert!(a2i.lookup_by_ir_id(0).is_none());
    }

    // ── IrToIr ────────────────────────────────────────────────────────────

    #[test]
    fn test_ir_to_ir_add_mapping() {
        let mut m = IrToIr::new("contraction");
        m.add_mapping(7, vec![100]);
        m.add_mapping(8, vec![100]);
        m.add_mapping(9, vec![100]);
        assert_eq!(m.lookup_by_original_id(7), Some([100].as_slice()));
        assert_eq!(m.lookup_by_new_id(100), Some(7)); // first one found
    }

    #[test]
    fn test_ir_to_ir_add_deletion() {
        let mut m = IrToIr::new("dead_store");
        m.add_deletion(5);
        assert!(m.deleted.contains(&5));
        assert!(m.lookup_by_original_id(5).is_none()); // deleted
    }

    #[test]
    fn test_ir_to_ir_not_found() {
        let m = IrToIr::new("identity");
        assert!(m.lookup_by_original_id(0).is_none());
        assert!(m.lookup_by_new_id(0).is_none());
    }

    #[test]
    fn test_ir_to_ir_pass_name() {
        let m = IrToIr::new("my_pass");
        assert_eq!(m.pass_name, "my_pass");
    }

    // ── IrToMachineCode ───────────────────────────────────────────────────

    #[test]
    fn test_ir_to_mc_add_and_lookup() {
        let mut mc = IrToMachineCode::new();
        mc.add(3, 0x14, 4);
        assert_eq!(mc.lookup_by_ir_id(3), Some((0x14, 4)));
    }

    #[test]
    fn test_ir_to_mc_lookup_by_offset() {
        let mut mc = IrToMachineCode::new();
        mc.add(3, 0x14, 4); // bytes 0x14..0x18
        assert_eq!(mc.lookup_by_mc_offset(0x14), Some(3));
        assert_eq!(mc.lookup_by_mc_offset(0x15), Some(3)); // inside the range
        assert_eq!(mc.lookup_by_mc_offset(0x17), Some(3)); // last byte
        assert_eq!(mc.lookup_by_mc_offset(0x18), None);    // past the end
    }

    #[test]
    fn test_ir_to_mc_not_found() {
        let mc = IrToMachineCode::new();
        assert!(mc.lookup_by_ir_id(0).is_none());
        assert!(mc.lookup_by_mc_offset(0).is_none());
    }

    // ── SourceMapChain ────────────────────────────────────────────────────

    #[test]
    fn test_chain_new() {
        let chain = SourceMapChain::new();
        assert!(chain.source_to_ast.entries.is_empty());
        assert!(chain.ast_to_ir.entries.is_empty());
        assert!(chain.ir_to_ir.is_empty());
        assert!(chain.ir_to_machine_code.is_none());
    }

    #[test]
    fn test_chain_source_to_mc_no_backend() {
        let chain = SourceMapChain::new();
        let pos = SourcePosition { file: "x.bf".to_string(), line: 1, column: 1, length: 1 };
        assert!(chain.source_to_mc(&pos).is_none());
    }

    #[test]
    fn test_chain_mc_to_source_no_backend() {
        let chain = SourceMapChain::new();
        assert!(chain.mc_to_source(0).is_none());
    }

    /// Full end-to-end roundtrip: source pos → AST → IR → MC → source pos.
    #[test]
    fn test_chain_full_roundtrip() {
        let mut chain = SourceMapChain::new();

        // Segment 1: "+" at line 1, column 1 → AST node 0
        let pos = SourcePosition { file: "test.bf".to_string(), line: 1, column: 1, length: 1 };
        chain.source_to_ast.add(pos.clone(), 0);

        // Segment 2: AST node 0 → IR IDs [7, 8, 9, 10]
        chain.ast_to_ir.add(0, vec![7, 8, 9, 10]);

        // No optimiser passes — IDs flow through unchanged.

        // Segment 4: IR ID 7 → MC bytes [0, 4)
        let mut mc = IrToMachineCode::new();
        mc.add(7, 0, 4);
        mc.add(8, 4, 4);
        mc.add(9, 8, 4);
        mc.add(10, 12, 4);
        chain.ir_to_machine_code = Some(mc);

        // Forward: source pos → MC offsets
        let results = chain.source_to_mc(&pos).unwrap();
        assert_eq!(results.len(), 4);
        assert_eq!(results[0].mc_offset, 0);

        // Reverse: MC offset 0 → source pos
        let found = chain.mc_to_source(0).unwrap();
        assert_eq!(found, &pos);
    }

    /// Test that optimiser passes are followed correctly in forward direction.
    #[test]
    fn test_chain_with_optimizer_pass_forward() {
        let mut chain = SourceMapChain::new();
        let pos = SourcePosition { file: "test.bf".to_string(), line: 1, column: 1, length: 1 };
        chain.source_to_ast.add(pos.clone(), 0);
        chain.ast_to_ir.add(0, vec![7, 8, 9]);

        // Optimiser pass: contracts IR IDs 7, 8, 9 → new ID 100
        let mut pass = IrToIr::new("contraction");
        pass.add_mapping(7, vec![100]);
        pass.add_mapping(8, vec![100]);
        pass.add_mapping(9, vec![100]);
        chain.add_optimizer_pass(pass);

        let mut mc = IrToMachineCode::new();
        mc.add(100, 0, 4);
        chain.ir_to_machine_code = Some(mc);

        let results = chain.source_to_mc(&pos).unwrap();
        // After dedup (all three map to 100), we get one result per unique entry
        // Actually we get 3 results (one per original IR ID tracing to 100),
        // but since 100 maps to (0, 4), we get (0, 4) three times.
        // Let's verify at least one entry is present
        assert!(!results.is_empty());
        assert_eq!(results[0].mc_offset, 0);
    }

    /// Test that deleted instructions are excluded in forward lookup.
    #[test]
    fn test_chain_optimizer_deletion_forward() {
        let mut chain = SourceMapChain::new();
        let pos = SourcePosition { file: "test.bf".to_string(), line: 1, column: 1, length: 1 };
        chain.source_to_ast.add(pos.clone(), 0);
        chain.ast_to_ir.add(0, vec![5]);

        // Optimiser pass deletes IR ID 5
        let mut pass = IrToIr::new("dead_store");
        pass.add_deletion(5);
        chain.add_optimizer_pass(pass);

        let mut mc = IrToMachineCode::new();
        mc.add(5, 0, 4);
        chain.ir_to_machine_code = Some(mc);

        // Deleted instruction → no MC entries
        assert!(chain.source_to_mc(&pos).is_none());
    }
}
