// Package compilersourcemap provides the source-mapping sidecar that flows
// through every stage of the AOT compiler pipeline.
//
// ──────────────────────────────────────────────────────────────────────────────
// Why a "chain" instead of a flat table?
// ──────────────────────────────────────────────────────────────────────────────
//
// A flat table (machine-code offset → source position) works for the final
// consumer — a debugger, profiler, or error reporter. But it doesn't help
// when you're debugging the *compiler itself*:
//
//   - "Why did the optimiser delete instruction #42?"
//     → Look at the IrToIr segment for that pass.
//
//   - "Which AST node produced this IR instruction?"
//     → Look at AstToIr.
//
//   - "The machine code for this instruction seems wrong — what IR produced it?"
//     → Look at IrToMachineCode in reverse.
//
// The chain makes the compiler pipeline **transparent and debuggable at every
// stage**. The flat composite mapping is just the composition of all segments.
//
// ──────────────────────────────────────────────────────────────────────────────
// Segment overview
// ──────────────────────────────────────────────────────────────────────────────
//
//	Segment 1: SourceToAst   — source text position  → AST node ID
//	Segment 2: AstToIr       — AST node ID           → IR instruction IDs
//	Segment 3: IrToIr        — IR instruction ID      → optimised IR instruction IDs
//	                           (one segment per optimiser pass)
//	Segment 4: IrToMachineCode — IR instruction ID    → machine code byte offset + length
//
//	Composite:  source position → machine code offset  (forward)
//	            machine code offset → source position   (reverse)
//
package compilersourcemap

import "fmt"

// ──────────────────────────────────────────────────────────────────────────────
// SourcePosition — a span of characters in a source file
//
// Think of this as a "highlighter pen" marking a region of source code.
// The (Line, Column) pair marks the start; Length tells you how many
// characters are highlighted. For Brainfuck, every command is exactly
// one character (Length = 1). For BASIC, a keyword like "PRINT" would
// have Length = 5.
// ──────────────────────────────────────────────────────────────────────────────

type SourcePosition struct {
	File   string // source file path (e.g., "hello.bf")
	Line   int    // 1-based line number
	Column int    // 1-based column number
	Length int    // character span in source
}

// String returns a human-readable representation like "hello.bf:1:3 (len=1)".
func (sp SourcePosition) String() string {
	return fmt.Sprintf("%s:%d:%d (len=%d)", sp.File, sp.Line, sp.Column, sp.Length)
}

// ──────────────────────────────────────────────────────────────────────────────
// SourceToAstEntry — one mapping from a source position to an AST node
//
// Example: The "+" character at line 1, column 3 of "hello.bf" maps
// to AST node #42 (which is a command(INC) node in the parse tree).
// ──────────────────────────────────────────────────────────────────────────────

type SourceToAstEntry struct {
	Pos       SourcePosition
	AstNodeID int
}

// ──────────────────────────────────────────────────────────────────────────────
// SourceToAst — Segment 1: source text positions → AST node IDs
//
// This segment is produced by the parser or by the language-specific
// frontend (e.g., brainfuck-ir-compiler). It maps every meaningful
// source position to the AST node that represents it.
// ──────────────────────────────────────────────────────────────────────────────

type SourceToAst struct {
	Entries []SourceToAstEntry
}

// Add records a mapping from a source position to an AST node ID.
func (s *SourceToAst) Add(pos SourcePosition, astNodeID int) {
	s.Entries = append(s.Entries, SourceToAstEntry{Pos: pos, AstNodeID: astNodeID})
}

// LookupByNodeID returns the source position for the given AST node ID,
// or nil if not found. This is used for reverse lookups.
func (s *SourceToAst) LookupByNodeID(astNodeID int) *SourcePosition {
	for i := range s.Entries {
		if s.Entries[i].AstNodeID == astNodeID {
			pos := s.Entries[i].Pos
			return &pos
		}
	}
	return nil
}

// ──────────────────────────────────────────────────────────────────────────────
// AstToIrEntry — one mapping from an AST node to the IR instructions it produced
//
// A single AST node often produces multiple IR instructions. For example,
// a Brainfuck "+" command produces four instructions:
//   LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE
// So the mapping is one-to-many: ast_node_42 → [ir_7, ir_8, ir_9, ir_10].
// ──────────────────────────────────────────────────────────────────────────────

type AstToIrEntry struct {
	AstNodeID int
	IrIDs     []int // the IR instruction IDs this AST node produced
}

// ──────────────────────────────────────────────────────────────────────────────
// AstToIr — Segment 2: AST node IDs → IR instruction IDs
// ──────────────────────────────────────────────────────────────────────────────

type AstToIr struct {
	Entries []AstToIrEntry
}

// Add records that the given AST node produced the given IR instruction IDs.
func (a *AstToIr) Add(astNodeID int, irIDs []int) {
	a.Entries = append(a.Entries, AstToIrEntry{AstNodeID: astNodeID, IrIDs: irIDs})
}

// LookupByAstNodeID returns the IR instruction IDs for the given AST node,
// or nil if not found.
func (a *AstToIr) LookupByAstNodeID(astNodeID int) []int {
	for i := range a.Entries {
		if a.Entries[i].AstNodeID == astNodeID {
			return a.Entries[i].IrIDs
		}
	}
	return nil
}

// LookupByIrID returns the AST node ID that produced the given IR instruction,
// or -1 if not found. This is used for reverse lookups.
func (a *AstToIr) LookupByIrID(irID int) int {
	for i := range a.Entries {
		for _, id := range a.Entries[i].IrIDs {
			if id == irID {
				return a.Entries[i].AstNodeID
			}
		}
	}
	return -1
}

// ──────────────────────────────────────────────────────────────────────────────
// IrToIrEntry — one mapping from an original IR instruction to its
// replacement(s) after an optimiser pass
//
// Three cases:
//   1. Preserved:  original_id → [same_id]        (instruction unchanged)
//   2. Replaced:   original_id → [new_id_1, ...]  (instruction split/transformed)
//   3. Deleted:    original_id is in Deleted set   (instruction optimised away)
//
// Example: A contraction pass folds three ADD_IMM 1 instructions
// (IDs 7, 8, 9) into one ADD_IMM 3 (ID 100):
//   7 → [100], 8 → [100], 9 → [100]
// ──────────────────────────────────────────────────────────────────────────────

type IrToIrEntry struct {
	OriginalID int
	NewIDs     []int
}

// ──────────────────────────────────────────────────────────────────────────────
// IrToIr — Segment 3: IR instruction IDs → optimised IR instruction IDs
//
// One segment is produced per optimiser pass. The PassName field identifies
// which pass produced this mapping (e.g., "identity", "contraction",
// "clear_loop", "dead_store").
// ──────────────────────────────────────────────────────────────────────────────

type IrToIr struct {
	Entries  []IrToIrEntry
	Deleted  map[int]bool // IDs that were optimised away (empty NewIDs)
	PassName string       // which optimiser pass produced this segment
}

// NewIrToIr creates an IrToIr segment for the named pass.
func NewIrToIr(passName string) *IrToIr {
	return &IrToIr{
		Deleted:  make(map[int]bool),
		PassName: passName,
	}
}

// AddMapping records that the original instruction was replaced by the new ones.
func (m *IrToIr) AddMapping(originalID int, newIDs []int) {
	m.Entries = append(m.Entries, IrToIrEntry{OriginalID: originalID, NewIDs: newIDs})
}

// AddDeletion records that the original instruction was deleted.
func (m *IrToIr) AddDeletion(originalID int) {
	m.Deleted[originalID] = true
	m.Entries = append(m.Entries, IrToIrEntry{OriginalID: originalID, NewIDs: nil})
}

// LookupByOriginalID returns the new IDs for the given original ID,
// or nil if deleted or not found.
func (m *IrToIr) LookupByOriginalID(originalID int) []int {
	if m.Deleted[originalID] {
		return nil
	}
	for i := range m.Entries {
		if m.Entries[i].OriginalID == originalID {
			return m.Entries[i].NewIDs
		}
	}
	return nil
}

// LookupByNewID returns the original ID that produced the given new ID,
// or -1 if not found. When multiple originals map to the same new ID
// (e.g., contraction), this returns the first one found.
func (m *IrToIr) LookupByNewID(newID int) int {
	for i := range m.Entries {
		for _, id := range m.Entries[i].NewIDs {
			if id == newID {
				return m.Entries[i].OriginalID
			}
		}
	}
	return -1
}

// ──────────────────────────────────────────────────────────────────────────────
// IrToMachineCodeEntry — one mapping from an IR instruction to the machine
// code bytes it produced
//
// Each entry is a triple: (ir_instruction_id, mc_byte_offset, mc_byte_length).
// For example, a LOAD_BYTE IR instruction might produce 8 bytes of RISC-V
// machine code starting at offset 0x14 in the .text section.
// ──────────────────────────────────────────────────────────────────────────────

type IrToMachineCodeEntry struct {
	IrID     int // IR instruction ID
	MCOffset int // byte offset in the .text section
	MCLength int // number of bytes of machine code
}

// ──────────────────────────────────────────────────────────────────────────────
// IrToMachineCode — Segment 4: IR instruction IDs → machine code byte offsets
// ──────────────────────────────────────────────────────────────────────────────

type IrToMachineCode struct {
	Entries []IrToMachineCodeEntry
}

// Add records that the given IR instruction produced machine code at the
// given offset with the given length.
func (m *IrToMachineCode) Add(irID, mcOffset, mcLength int) {
	m.Entries = append(m.Entries, IrToMachineCodeEntry{
		IrID:     irID,
		MCOffset: mcOffset,
		MCLength: mcLength,
	})
}

// LookupByIrID returns the machine code offset and length for the given
// IR instruction ID, or (-1, 0) if not found.
func (m *IrToMachineCode) LookupByIrID(irID int) (offset, length int) {
	for i := range m.Entries {
		if m.Entries[i].IrID == irID {
			return m.Entries[i].MCOffset, m.Entries[i].MCLength
		}
	}
	return -1, 0
}

// LookupByMCOffset returns the IR instruction ID whose machine code
// contains the given byte offset, or -1 if not found.
//
// A machine code offset "contains" an IR instruction if:
//   entry.MCOffset <= offset < entry.MCOffset + entry.MCLength
func (m *IrToMachineCode) LookupByMCOffset(offset int) int {
	for i := range m.Entries {
		e := m.Entries[i]
		if offset >= e.MCOffset && offset < e.MCOffset+e.MCLength {
			return e.IrID
		}
	}
	return -1
}

// ──────────────────────────────────────────────────────────────────────────────
// SourceMapChain — the full pipeline sidecar
//
// This is the central data structure that flows through every stage of the
// compiler pipeline. Each stage reads the existing segments and appends
// its own:
//
//   1. Frontend (brainfuck-ir-compiler) → fills SourceToAst + AstToIr
//   2. Optimiser (compiler-ir-optimizer) → appends IrToIr segments
//   3. Backend (codegen-riscv) → fills IrToMachineCode
//
// ──────────────────────────────────────────────────────────────────────────────

type SourceMapChain struct {
	SourceToAst     *SourceToAst
	AstToIr         *AstToIr
	IrToIr          []*IrToIr          // one per optimiser pass
	IrToMachineCode *IrToMachineCode   // filled by backend (nil until then)
}

// NewSourceMapChain creates an empty source map chain ready for use.
func NewSourceMapChain() *SourceMapChain {
	return &SourceMapChain{
		SourceToAst: &SourceToAst{},
		AstToIr:     &AstToIr{},
	}
}

// AddOptimizerPass appends an IrToIr segment from an optimiser pass.
func (c *SourceMapChain) AddOptimizerPass(segment *IrToIr) {
	c.IrToIr = append(c.IrToIr, segment)
}

// ──────────────────────────────────────────────────────────────────────────────
// Composite queries — compose all segments for end-to-end lookups
// ──────────────────────────────────────────────────────────────────────────────

// SourceToMC composes all segments to look up the machine code offset(s)
// for a given source position. Returns nil if the chain is incomplete
// or no mapping exists.
//
// Algorithm:
//   1. SourceToAst: source position → AST node ID
//   2. AstToIr: AST node ID → IR instruction IDs
//   3. IrToIr (each pass): follow IR IDs through each optimiser pass
//   4. IrToMachineCode: final IR IDs → machine code offsets
func (c *SourceMapChain) SourceToMC(pos SourcePosition) []IrToMachineCodeEntry {
	if c.IrToMachineCode == nil {
		return nil
	}

	// Step 1: source → AST node
	var astNodeID int = -1
	for _, entry := range c.SourceToAst.Entries {
		if entry.Pos.File == pos.File &&
			entry.Pos.Line == pos.Line &&
			entry.Pos.Column == pos.Column {
			astNodeID = entry.AstNodeID
			break
		}
	}
	if astNodeID == -1 {
		return nil
	}

	// Step 2: AST node → IR IDs
	irIDs := c.AstToIr.LookupByAstNodeID(astNodeID)
	if irIDs == nil {
		return nil
	}

	// Step 3: follow through optimiser passes
	currentIDs := irIDs
	for _, pass := range c.IrToIr {
		var nextIDs []int
		for _, id := range currentIDs {
			if pass.Deleted[id] {
				continue // instruction was optimised away
			}
			newIDs := pass.LookupByOriginalID(id)
			if newIDs != nil {
				nextIDs = append(nextIDs, newIDs...)
			}
		}
		currentIDs = nextIDs
	}

	if len(currentIDs) == 0 {
		return nil
	}

	// Step 4: IR IDs → machine code
	var results []IrToMachineCodeEntry
	for _, id := range currentIDs {
		offset, length := c.IrToMachineCode.LookupByIrID(id)
		if offset >= 0 {
			results = append(results, IrToMachineCodeEntry{
				IrID:     id,
				MCOffset: offset,
				MCLength: length,
			})
		}
	}
	return results
}

// MCToSource composes all segments in reverse to look up the source
// position for a given machine code offset. Returns nil if the chain
// is incomplete or no mapping exists.
//
// Algorithm (reverse of SourceToMC):
//   1. IrToMachineCode: MC offset → IR instruction ID
//   2. IrToIr (each pass, in reverse): follow IR ID back through passes
//   3. AstToIr: IR ID → AST node ID
//   4. SourceToAst: AST node ID → source position
func (c *SourceMapChain) MCToSource(mcOffset int) *SourcePosition {
	if c.IrToMachineCode == nil {
		return nil
	}

	// Step 1: MC offset → IR ID
	irID := c.IrToMachineCode.LookupByMCOffset(mcOffset)
	if irID == -1 {
		return nil
	}

	// Step 2: follow back through optimiser passes (in reverse order)
	currentID := irID
	for i := len(c.IrToIr) - 1; i >= 0; i-- {
		pass := c.IrToIr[i]
		originalID := pass.LookupByNewID(currentID)
		if originalID == -1 {
			return nil // can't trace back through this pass
		}
		currentID = originalID
	}

	// Step 3: IR ID → AST node ID
	astNodeID := c.AstToIr.LookupByIrID(currentID)
	if astNodeID == -1 {
		return nil
	}

	// Step 4: AST node ID → source position
	return c.SourceToAst.LookupByNodeID(astNodeID)
}
