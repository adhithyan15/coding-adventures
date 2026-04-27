package compilersourcemap

import (
	"testing"
)

// ──────────────────────────────────────────────────────────────────────────────
// Test helpers
// ──────────────────────────────────────────────────────────────────────────────

// buildTestChain creates a fully-populated source map chain for testing.
//
// The chain represents a tiny Brainfuck program "+." compiled through
// the full pipeline:
//
//	Source: "+" at (1,1) → AST node 0 → IR [2,3,4,5]
//	        "." at (1,2) → AST node 1 → IR [6,7]
//	Prologue IR [0,1] have no AST mapping.
//	Identity optimiser pass: every IR ID maps to itself.
//	Machine code: each IR instruction has a known offset and length.
func buildTestChain() *SourceMapChain {
	chain := NewSourceMapChain()

	// Segment 1: SourceToAst
	chain.SourceToAst.Add(SourcePosition{File: "hello.bf", Line: 1, Column: 1, Length: 1}, 0)
	chain.SourceToAst.Add(SourcePosition{File: "hello.bf", Line: 1, Column: 2, Length: 1}, 1)

	// Segment 2: AstToIr
	chain.AstToIr.Add(0, []int{2, 3, 4, 5}) // "+" → LOAD_BYTE, ADD_IMM, AND_IMM, STORE_BYTE
	chain.AstToIr.Add(1, []int{6, 7})        // "." → LOAD_BYTE, SYSCALL

	// Segment 3: IrToIr (identity pass)
	identity := NewIrToIr("identity")
	for i := 0; i <= 7; i++ {
		identity.AddMapping(i, []int{i})
	}
	chain.AddOptimizerPass(identity)

	// Segment 4: IrToMachineCode
	mc := &IrToMachineCode{}
	mc.Add(0, 0x00, 8) // LOAD_ADDR → 8 bytes
	mc.Add(1, 0x08, 4) // LOAD_IMM → 4 bytes
	mc.Add(2, 0x0C, 8) // LOAD_BYTE → 8 bytes
	mc.Add(3, 0x14, 4) // ADD_IMM → 4 bytes
	mc.Add(4, 0x18, 4) // AND_IMM → 4 bytes
	mc.Add(5, 0x1C, 8) // STORE_BYTE → 8 bytes
	mc.Add(6, 0x24, 8) // LOAD_BYTE → 8 bytes
	mc.Add(7, 0x2C, 8) // SYSCALL → 8 bytes
	chain.IrToMachineCode = mc

	return chain
}

// ──────────────────────────────────────────────────────────────────────────────
// SourcePosition tests
// ──────────────────────────────────────────────────────────────────────────────

func TestSourcePositionString(t *testing.T) {
	sp := SourcePosition{File: "test.bf", Line: 3, Column: 7, Length: 1}
	got := sp.String()
	want := "test.bf:3:7 (len=1)"
	if got != want {
		t.Errorf("SourcePosition.String() = %q, want %q", got, want)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// SourceToAst tests
// ──────────────────────────────────────────────────────────────────────────────

func TestSourceToAstAdd(t *testing.T) {
	s := &SourceToAst{}
	s.Add(SourcePosition{File: "a.bf", Line: 1, Column: 1, Length: 1}, 42)
	if len(s.Entries) != 1 {
		t.Fatalf("expected 1 entry, got %d", len(s.Entries))
	}
	if s.Entries[0].AstNodeID != 42 {
		t.Errorf("expected AstNodeID 42, got %d", s.Entries[0].AstNodeID)
	}
}

func TestSourceToAstLookupByNodeID(t *testing.T) {
	s := &SourceToAst{}
	s.Add(SourcePosition{File: "a.bf", Line: 1, Column: 1, Length: 1}, 10)
	s.Add(SourcePosition{File: "a.bf", Line: 1, Column: 2, Length: 1}, 11)

	// Found
	pos := s.LookupByNodeID(11)
	if pos == nil {
		t.Fatal("expected to find node 11")
	}
	if pos.Column != 2 {
		t.Errorf("expected column 2, got %d", pos.Column)
	}

	// Not found
	pos = s.LookupByNodeID(999)
	if pos != nil {
		t.Errorf("expected nil for missing node, got %v", pos)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// AstToIr tests
// ──────────────────────────────────────────────────────────────────────────────

func TestAstToIrAdd(t *testing.T) {
	a := &AstToIr{}
	a.Add(0, []int{10, 11, 12})
	if len(a.Entries) != 1 {
		t.Fatalf("expected 1 entry, got %d", len(a.Entries))
	}
	if len(a.Entries[0].IrIDs) != 3 {
		t.Errorf("expected 3 IR IDs, got %d", len(a.Entries[0].IrIDs))
	}
}

func TestAstToIrLookupByAstNodeID(t *testing.T) {
	a := &AstToIr{}
	a.Add(5, []int{20, 21})
	a.Add(6, []int{22})

	ids := a.LookupByAstNodeID(5)
	if len(ids) != 2 || ids[0] != 20 || ids[1] != 21 {
		t.Errorf("expected [20, 21], got %v", ids)
	}

	ids = a.LookupByAstNodeID(999)
	if ids != nil {
		t.Errorf("expected nil for missing AST node, got %v", ids)
	}
}

func TestAstToIrLookupByIrID(t *testing.T) {
	a := &AstToIr{}
	a.Add(5, []int{20, 21})
	a.Add(6, []int{22})

	// IR ID 21 belongs to AST node 5
	astID := a.LookupByIrID(21)
	if astID != 5 {
		t.Errorf("expected AST node 5, got %d", astID)
	}

	// IR ID 22 belongs to AST node 6
	astID = a.LookupByIrID(22)
	if astID != 6 {
		t.Errorf("expected AST node 6, got %d", astID)
	}

	// IR ID 999 not found
	astID = a.LookupByIrID(999)
	if astID != -1 {
		t.Errorf("expected -1 for missing IR ID, got %d", astID)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// IrToIr tests
// ──────────────────────────────────────────────────────────────────────────────

func TestIrToIrIdentityPass(t *testing.T) {
	m := NewIrToIr("identity")
	for i := 0; i < 5; i++ {
		m.AddMapping(i, []int{i})
	}

	// Every ID maps to itself
	for i := 0; i < 5; i++ {
		ids := m.LookupByOriginalID(i)
		if len(ids) != 1 || ids[0] != i {
			t.Errorf("identity pass: ID %d should map to [%d], got %v", i, i, ids)
		}
	}

	// Reverse lookup also works
	for i := 0; i < 5; i++ {
		orig := m.LookupByNewID(i)
		if orig != i {
			t.Errorf("reverse: new ID %d should come from %d, got %d", i, i, orig)
		}
	}
}

func TestIrToIrContraction(t *testing.T) {
	// Simulates a contraction pass that folds IDs 7,8,9 into ID 100
	m := NewIrToIr("contraction")
	m.AddMapping(7, []int{100})
	m.AddMapping(8, []int{100})
	m.AddMapping(9, []int{100})

	// All three originals map to 100
	for _, orig := range []int{7, 8, 9} {
		ids := m.LookupByOriginalID(orig)
		if len(ids) != 1 || ids[0] != 100 {
			t.Errorf("contraction: ID %d should map to [100], got %v", orig, ids)
		}
	}

	// Reverse: 100 maps back to the first original found (7)
	orig := m.LookupByNewID(100)
	if orig != 7 {
		t.Errorf("reverse: new ID 100 should come from 7, got %d", orig)
	}
}

func TestIrToIrDeletion(t *testing.T) {
	m := NewIrToIr("dead_store")
	m.AddMapping(1, []int{1}) // preserved
	m.AddDeletion(2)          // deleted

	// ID 1 is preserved
	ids := m.LookupByOriginalID(1)
	if len(ids) != 1 || ids[0] != 1 {
		t.Errorf("expected [1], got %v", ids)
	}

	// ID 2 is deleted
	ids = m.LookupByOriginalID(2)
	if ids != nil {
		t.Errorf("expected nil for deleted ID, got %v", ids)
	}
	if !m.Deleted[2] {
		t.Error("expected ID 2 to be in Deleted set")
	}
}

func TestIrToIrLookupByNewIDNotFound(t *testing.T) {
	m := NewIrToIr("test")
	m.AddMapping(1, []int{1})

	orig := m.LookupByNewID(999)
	if orig != -1 {
		t.Errorf("expected -1 for missing new ID, got %d", orig)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// IrToMachineCode tests
// ──────────────────────────────────────────────────────────────────────────────

func TestIrToMachineCodeAdd(t *testing.T) {
	mc := &IrToMachineCode{}
	mc.Add(5, 0x20, 4)
	if len(mc.Entries) != 1 {
		t.Fatalf("expected 1 entry, got %d", len(mc.Entries))
	}
	if mc.Entries[0].MCOffset != 0x20 || mc.Entries[0].MCLength != 4 {
		t.Errorf("unexpected entry: %+v", mc.Entries[0])
	}
}

func TestIrToMachineCodeLookupByIrID(t *testing.T) {
	mc := &IrToMachineCode{}
	mc.Add(5, 0x20, 4)
	mc.Add(6, 0x24, 8)

	offset, length := mc.LookupByIrID(5)
	if offset != 0x20 || length != 4 {
		t.Errorf("expected (0x20, 4), got (%d, %d)", offset, length)
	}

	offset, length = mc.LookupByIrID(999)
	if offset != -1 || length != 0 {
		t.Errorf("expected (-1, 0) for missing ID, got (%d, %d)", offset, length)
	}
}

func TestIrToMachineCodeLookupByMCOffset(t *testing.T) {
	mc := &IrToMachineCode{}
	mc.Add(5, 0x20, 4) // bytes 0x20..0x23
	mc.Add(6, 0x24, 8) // bytes 0x24..0x2B

	// Exact start of instruction 5
	irID := mc.LookupByMCOffset(0x20)
	if irID != 5 {
		t.Errorf("offset 0x20 should map to IR ID 5, got %d", irID)
	}

	// Middle of instruction 5
	irID = mc.LookupByMCOffset(0x22)
	if irID != 5 {
		t.Errorf("offset 0x22 should map to IR ID 5, got %d", irID)
	}

	// First byte of instruction 6
	irID = mc.LookupByMCOffset(0x24)
	if irID != 6 {
		t.Errorf("offset 0x24 should map to IR ID 6, got %d", irID)
	}

	// Last byte of instruction 6
	irID = mc.LookupByMCOffset(0x2B)
	if irID != 6 {
		t.Errorf("offset 0x2B should map to IR ID 6, got %d", irID)
	}

	// Outside any instruction
	irID = mc.LookupByMCOffset(0x2C)
	if irID != -1 {
		t.Errorf("offset 0x2C should return -1, got %d", irID)
	}
}

// ──────────────────────────────────────────────────────────────────────────────
// SourceMapChain tests
// ──────────────────────────────────────────────────────────────────────────────

func TestNewSourceMapChain(t *testing.T) {
	chain := NewSourceMapChain()
	if chain.SourceToAst == nil {
		t.Error("SourceToAst should not be nil")
	}
	if chain.AstToIr == nil {
		t.Error("AstToIr should not be nil")
	}
	if chain.IrToMachineCode != nil {
		t.Error("IrToMachineCode should be nil initially")
	}
	if len(chain.IrToIr) != 0 {
		t.Error("IrToIr should be empty initially")
	}
}

func TestSourceMapChainForwardLookup(t *testing.T) {
	chain := buildTestChain()

	// "+" at line 1 col 1 → machine code bytes 0x0C..0x23
	results := chain.SourceToMC(SourcePosition{File: "hello.bf", Line: 1, Column: 1, Length: 1})
	if len(results) != 4 {
		t.Fatalf("expected 4 MC entries for '+', got %d", len(results))
	}

	// The four entries should cover IR IDs 2, 3, 4, 5
	expectedIDs := map[int]bool{2: true, 3: true, 4: true, 5: true}
	for _, r := range results {
		if !expectedIDs[r.IrID] {
			t.Errorf("unexpected IR ID %d in results", r.IrID)
		}
	}

	// First MC entry should be at offset 0x0C
	if results[0].MCOffset != 0x0C {
		t.Errorf("expected first MC offset 0x0C, got 0x%X", results[0].MCOffset)
	}
}

func TestSourceMapChainForwardLookupDot(t *testing.T) {
	chain := buildTestChain()

	// "." at line 1 col 2 → machine code bytes 0x24..0x33
	results := chain.SourceToMC(SourcePosition{File: "hello.bf", Line: 1, Column: 2, Length: 1})
	if len(results) != 2 {
		t.Fatalf("expected 2 MC entries for '.', got %d", len(results))
	}

	expectedIDs := map[int]bool{6: true, 7: true}
	for _, r := range results {
		if !expectedIDs[r.IrID] {
			t.Errorf("unexpected IR ID %d in results", r.IrID)
		}
	}
}

func TestSourceMapChainReverseLookup(t *testing.T) {
	chain := buildTestChain()

	// MC offset 0x14 → IR #3 (ADD_IMM) → AST node 0 → line 1 col 1 ("+")
	pos := chain.MCToSource(0x14)
	if pos == nil {
		t.Fatal("expected source position for MC offset 0x14")
	}
	if pos.File != "hello.bf" || pos.Line != 1 || pos.Column != 1 {
		t.Errorf("expected hello.bf:1:1, got %s", pos.String())
	}

	// MC offset 0x2C → IR #7 (SYSCALL) → AST node 1 → line 1 col 2 (".")
	pos = chain.MCToSource(0x2C)
	if pos == nil {
		t.Fatal("expected source position for MC offset 0x2C")
	}
	if pos.File != "hello.bf" || pos.Line != 1 || pos.Column != 2 {
		t.Errorf("expected hello.bf:1:2, got %s", pos.String())
	}
}

func TestSourceMapChainReverseLookupMiddleOfInstruction(t *testing.T) {
	chain := buildTestChain()

	// MC offset 0x0E (middle of LOAD_BYTE, IR #2) → AST node 0 → "+"
	pos := chain.MCToSource(0x0E)
	if pos == nil {
		t.Fatal("expected source position for MC offset 0x0E")
	}
	if pos.Column != 1 {
		t.Errorf("expected column 1, got %d", pos.Column)
	}
}

func TestSourceMapChainForwardNotFound(t *testing.T) {
	chain := buildTestChain()

	// Position that doesn't exist in the source map
	results := chain.SourceToMC(SourcePosition{File: "hello.bf", Line: 99, Column: 1, Length: 1})
	if results != nil {
		t.Errorf("expected nil for missing position, got %v", results)
	}
}

func TestSourceMapChainReverseNotFound(t *testing.T) {
	chain := buildTestChain()

	// MC offset outside any instruction
	pos := chain.MCToSource(0xFF)
	if pos != nil {
		t.Errorf("expected nil for out-of-range MC offset, got %s", pos.String())
	}
}

func TestSourceMapChainIncomplete(t *testing.T) {
	chain := NewSourceMapChain()

	// Forward lookup with no IrToMachineCode segment
	results := chain.SourceToMC(SourcePosition{File: "a.bf", Line: 1, Column: 1, Length: 1})
	if results != nil {
		t.Errorf("expected nil for incomplete chain, got %v", results)
	}

	// Reverse lookup with no IrToMachineCode segment
	pos := chain.MCToSource(0)
	if pos != nil {
		t.Errorf("expected nil for incomplete chain, got %v", pos)
	}
}

func TestSourceMapChainMultipleOptimizerPasses(t *testing.T) {
	chain := NewSourceMapChain()
	chain.SourceToAst.Add(SourcePosition{File: "t.bf", Line: 1, Column: 1, Length: 1}, 0)
	chain.AstToIr.Add(0, []int{1, 2, 3})

	// Pass 1: identity (1→1, 2→2, 3→3)
	pass1 := NewIrToIr("identity")
	pass1.AddMapping(1, []int{1})
	pass1.AddMapping(2, []int{2})
	pass1.AddMapping(3, []int{3})
	chain.AddOptimizerPass(pass1)

	// Pass 2: contraction (1,2,3 → 100)
	pass2 := NewIrToIr("contraction")
	pass2.AddMapping(1, []int{100})
	pass2.AddMapping(2, []int{100})
	pass2.AddMapping(3, []int{100})
	chain.AddOptimizerPass(pass2)

	// Machine code
	mc := &IrToMachineCode{}
	mc.Add(100, 0x00, 4)
	chain.IrToMachineCode = mc

	// Forward: source → AST 0 → IR [1,2,3] → pass1 [1,2,3] → pass2 [100,100,100] → MC
	results := chain.SourceToMC(SourcePosition{File: "t.bf", Line: 1, Column: 1, Length: 1})
	if len(results) == 0 {
		t.Fatal("expected MC results for contracted code")
	}

	// Reverse: MC 0x00 → IR 100 → pass2 reverse → 1 → pass1 reverse → 1 → AST 0 → source
	pos := chain.MCToSource(0x00)
	if pos == nil {
		t.Fatal("expected source position for MC 0x00")
	}
	if pos.Line != 1 || pos.Column != 1 {
		t.Errorf("expected line 1 col 1, got %s", pos.String())
	}
}

func TestSourceMapChainDeletion(t *testing.T) {
	chain := NewSourceMapChain()
	chain.SourceToAst.Add(SourcePosition{File: "t.bf", Line: 1, Column: 1, Length: 1}, 0)
	chain.AstToIr.Add(0, []int{1, 2})

	// Optimizer deletes IR ID 2
	pass := NewIrToIr("dead_store")
	pass.AddMapping(1, []int{1})
	pass.AddDeletion(2)
	chain.AddOptimizerPass(pass)

	mc := &IrToMachineCode{}
	mc.Add(1, 0x00, 4)
	chain.IrToMachineCode = mc

	// Forward: only IR 1 survives → one MC entry
	results := chain.SourceToMC(SourcePosition{File: "t.bf", Line: 1, Column: 1, Length: 1})
	if len(results) != 1 {
		t.Fatalf("expected 1 MC entry after deletion, got %d", len(results))
	}
	if results[0].IrID != 1 {
		t.Errorf("expected surviving IR ID 1, got %d", results[0].IrID)
	}
}

func TestSourceMapChainPrologueMCReversesToNil(t *testing.T) {
	chain := buildTestChain()

	// MC offset 0x00 → IR #0 (prologue LOAD_ADDR)
	// IR #0 has no AST node mapping → reverse should return nil
	pos := chain.MCToSource(0x00)
	if pos != nil {
		t.Errorf("prologue instructions should not map to source, got %s", pos.String())
	}
}
