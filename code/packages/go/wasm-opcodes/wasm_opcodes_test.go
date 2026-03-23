package wasmopcodes

// Tests for the wasm-opcodes package.
//
// Test plan:
//  1.  Total opcode count is exactly 172.
//  2.  Byte lookup: GetOpcode(0x6A) returns i32.add.
//  3.  Name lookup: GetOpcodeByName("i32.add") returns correct info.
//  4.  Stack effects: i32.add pops 2, pushes 1.
//  5.  i32.const has immediate "i32".
//  6.  Memory loads have immediate "memarg".
//  7.  Control instructions have correct immediates.
//  8.  Unknown byte returns ok=false.
//  9.  Unknown name returns ok=false.
//  10. All opcodes have non-empty name and valid category.
//  11. All opcode bytes are unique (no duplicates).
//  12. All opcode names are unique.
//  13. Opcodes and OpcodesByName have same count.
//  14. Memory stores pop 2, push 0.
//  15. Conversion instructions pop 1, push 1.
//  16. select pops 3, pushes 1.

import (
	"testing"
)

// validCategories holds the complete set of category strings used in this package.
var validCategories = map[string]bool{
	"control":     true,
	"parametric":  true,
	"variable":    true,
	"memory":      true,
	"numeric_i32": true,
	"numeric_i64": true,
	"numeric_f32": true,
	"numeric_f64": true,
	"conversion":  true,
}

func TestPackageLoads(t *testing.T) {
	t.Log("wasm-opcodes package loaded successfully")
}

// ---------------------------------------------------------------------------
// COUNT TESTS
// ---------------------------------------------------------------------------

func TestTotalOpcodeCount(t *testing.T) {
	// 13 control + 2 parametric + 5 variable
	// + 14 loads + 9 stores + 2 memory_mgmt
	// + 30 i32 + 30 i64 + 21 f32 + 21 f64
	// + 25 conversion = 172
	const want = 172
	if got := len(Opcodes); got != want {
		t.Errorf("len(Opcodes) = %d, want %d", got, want)
	}
}

func TestBothMapsHaveSameCount(t *testing.T) {
	if len(Opcodes) != len(OpcodesByName) {
		t.Errorf("len(Opcodes)=%d != len(OpcodesByName)=%d", len(Opcodes), len(OpcodesByName))
	}
}

// ---------------------------------------------------------------------------
// UNIQUENESS TESTS
// ---------------------------------------------------------------------------

func TestOpcodeNamesAreUnique(t *testing.T) {
	// OpcodesByName is a map keyed by name. If two raw entries shared a name,
	// one would silently overwrite the other and the count would be smaller.
	// Verify by counting the raw table entries directly.
	if len(rawTable) != len(OpcodesByName) {
		t.Errorf("rawTable has %d entries but OpcodesByName has %d (duplicates?)",
			len(rawTable), len(OpcodesByName))
	}
}

func TestOpcodeBytesAreUnique(t *testing.T) {
	if len(rawTable) != len(Opcodes) {
		t.Errorf("rawTable has %d entries but Opcodes has %d (duplicate bytes?)",
			len(rawTable), len(Opcodes))
	}
}

// ---------------------------------------------------------------------------
// BYTE LOOKUP TESTS
// ---------------------------------------------------------------------------

func TestGetOpcodeI32Add(t *testing.T) {
	info, ok := GetOpcode(0x6A)
	if !ok {
		t.Fatal("GetOpcode(0x6A) returned ok=false")
	}
	if info.Name != "i32.add" {
		t.Errorf("Name = %q, want %q", info.Name, "i32.add")
	}
	if info.Opcode != 0x6A {
		t.Errorf("Opcode = 0x%02X, want 0x6A", info.Opcode)
	}
	if info.Category != "numeric_i32" {
		t.Errorf("Category = %q, want %q", info.Category, "numeric_i32")
	}
}

func TestGetOpcodeUnreachable(t *testing.T) {
	info, ok := GetOpcode(0x00)
	if !ok {
		t.Fatal("GetOpcode(0x00) returned ok=false")
	}
	if info.Name != "unreachable" {
		t.Errorf("Name = %q, want %q", info.Name, "unreachable")
	}
}

func TestGetOpcodeMemoryGrow(t *testing.T) {
	info, ok := GetOpcode(0x40)
	if !ok {
		t.Fatal("GetOpcode(0x40) returned ok=false")
	}
	if info.Name != "memory.grow" {
		t.Errorf("Name = %q, want %q", info.Name, "memory.grow")
	}
}

func TestGetOpcodeLastConversion(t *testing.T) {
	info, ok := GetOpcode(0xBF)
	if !ok {
		t.Fatal("GetOpcode(0xBF) returned ok=false")
	}
	if info.Name != "f64.reinterpret_i64" {
		t.Errorf("Name = %q, want %q", info.Name, "f64.reinterpret_i64")
	}
}

func TestGetOpcodeUnknownReturnsNotOk(t *testing.T) {
	_, ok := GetOpcode(0xFF)
	if ok {
		t.Error("GetOpcode(0xFF) should return ok=false for unknown opcode")
	}
}

func TestGetOpcodeAnotherUnknown(t *testing.T) {
	_, ok := GetOpcode(0xC0)
	if ok {
		t.Error("GetOpcode(0xC0) should return ok=false")
	}
}

// ---------------------------------------------------------------------------
// NAME LOOKUP TESTS
// ---------------------------------------------------------------------------

func TestGetOpcodeByNameI32Add(t *testing.T) {
	info, ok := GetOpcodeByName("i32.add")
	if !ok {
		t.Fatal("GetOpcodeByName(\"i32.add\") returned ok=false")
	}
	if info.Opcode != 0x6A {
		t.Errorf("Opcode = 0x%02X, want 0x6A", info.Opcode)
	}
}

func TestGetOpcodeByNameLocalGet(t *testing.T) {
	info, ok := GetOpcodeByName("local.get")
	if !ok {
		t.Fatal("GetOpcodeByName(\"local.get\") returned ok=false")
	}
	if info.Opcode != 0x20 {
		t.Errorf("Opcode = 0x%02X, want 0x20", info.Opcode)
	}
	if info.Category != "variable" {
		t.Errorf("Category = %q, want %q", info.Category, "variable")
	}
}

func TestGetOpcodeByNameUnknownReturnsNotOk(t *testing.T) {
	_, ok := GetOpcodeByName("not_a_real_op")
	if ok {
		t.Error("GetOpcodeByName(\"not_a_real_op\") should return ok=false")
	}
}

func TestGetOpcodeByNameEmptyStringReturnsNotOk(t *testing.T) {
	_, ok := GetOpcodeByName("")
	if ok {
		t.Error("GetOpcodeByName(\"\") should return ok=false")
	}
}

// ---------------------------------------------------------------------------
// STACK EFFECT TESTS
// ---------------------------------------------------------------------------

func TestI32AddStackEffect(t *testing.T) {
	info, ok := GetOpcode(0x6A)
	if !ok {
		t.Fatal("i32.add not found")
	}
	if info.StackPop != 2 {
		t.Errorf("StackPop = %d, want 2", info.StackPop)
	}
	if info.StackPush != 1 {
		t.Errorf("StackPush = %d, want 1", info.StackPush)
	}
}

func TestI32ConstStackEffect(t *testing.T) {
	info, ok := GetOpcodeByName("i32.const")
	if !ok {
		t.Fatal("i32.const not found")
	}
	if info.StackPop != 0 {
		t.Errorf("StackPop = %d, want 0", info.StackPop)
	}
	if info.StackPush != 1 {
		t.Errorf("StackPush = %d, want 1", info.StackPush)
	}
}

func TestDropStackEffect(t *testing.T) {
	info, ok := GetOpcodeByName("drop")
	if !ok {
		t.Fatal("drop not found")
	}
	if info.StackPop != 1 {
		t.Errorf("StackPop = %d, want 1", info.StackPop)
	}
	if info.StackPush != 0 {
		t.Errorf("StackPush = %d, want 0", info.StackPush)
	}
}

func TestSelectStackEffect(t *testing.T) {
	info, ok := GetOpcodeByName("select")
	if !ok {
		t.Fatal("select not found")
	}
	if info.StackPop != 3 {
		t.Errorf("StackPop = %d, want 3", info.StackPop)
	}
	if info.StackPush != 1 {
		t.Errorf("StackPush = %d, want 1", info.StackPush)
	}
}

func TestLocalTeeStackEffect(t *testing.T) {
	info, ok := GetOpcodeByName("local.tee")
	if !ok {
		t.Fatal("local.tee not found")
	}
	if info.StackPop != 1 {
		t.Errorf("StackPop = %d, want 1", info.StackPop)
	}
	if info.StackPush != 1 {
		t.Errorf("StackPush = %d, want 1", info.StackPush)
	}
}

func TestMemoryLoadsStackEffect(t *testing.T) {
	loadNames := []string{
		"i32.load", "i64.load", "f32.load", "f64.load",
		"i32.load8_s", "i32.load8_u", "i32.load16_s", "i32.load16_u",
		"i64.load8_s", "i64.load8_u", "i64.load16_s", "i64.load16_u",
		"i64.load32_s", "i64.load32_u",
	}
	for _, name := range loadNames {
		info, ok := GetOpcodeByName(name)
		if !ok {
			t.Errorf("%s: not found", name)
			continue
		}
		if info.StackPop != 1 {
			t.Errorf("%s: StackPop = %d, want 1", name, info.StackPop)
		}
		if info.StackPush != 1 {
			t.Errorf("%s: StackPush = %d, want 1", name, info.StackPush)
		}
	}
}

func TestMemoryStoresStackEffect(t *testing.T) {
	storeNames := []string{
		"i32.store", "i64.store", "f32.store", "f64.store",
		"i32.store8", "i32.store16", "i64.store8", "i64.store16", "i64.store32",
	}
	for _, name := range storeNames {
		info, ok := GetOpcodeByName(name)
		if !ok {
			t.Errorf("%s: not found", name)
			continue
		}
		if info.StackPop != 2 {
			t.Errorf("%s: StackPop = %d, want 2", name, info.StackPop)
		}
		if info.StackPush != 0 {
			t.Errorf("%s: StackPush = %d, want 0", name, info.StackPush)
		}
	}
}

func TestAllConversionsStackEffect(t *testing.T) {
	convNames := []string{
		"i32.wrap_i64", "i32.trunc_f32_s", "i32.trunc_f32_u",
		"i32.trunc_f64_s", "i32.trunc_f64_u",
		"i64.extend_i32_s", "i64.extend_i32_u",
		"i64.trunc_f32_s", "i64.trunc_f32_u",
		"i64.trunc_f64_s", "i64.trunc_f64_u",
		"f32.convert_i32_s", "f32.convert_i32_u",
		"f32.convert_i64_s", "f32.convert_i64_u",
		"f32.demote_f64", "f64.convert_i32_s", "f64.convert_i32_u",
		"f64.convert_i64_s", "f64.convert_i64_u", "f64.promote_f32",
		"i32.reinterpret_f32", "i64.reinterpret_f64",
		"f32.reinterpret_i32", "f64.reinterpret_i64",
	}
	for _, name := range convNames {
		info, ok := GetOpcodeByName(name)
		if !ok {
			t.Errorf("%s: not found", name)
			continue
		}
		if info.StackPop != 1 {
			t.Errorf("%s: StackPop = %d, want 1", name, info.StackPop)
		}
		if info.StackPush != 1 {
			t.Errorf("%s: StackPush = %d, want 1", name, info.StackPush)
		}
	}
}

// ---------------------------------------------------------------------------
// IMMEDIATES TESTS
// ---------------------------------------------------------------------------

func TestI32ConstHasI32Immediate(t *testing.T) {
	info, ok := GetOpcodeByName("i32.const")
	if !ok {
		t.Fatal("i32.const not found")
	}
	if len(info.Immediates) != 1 || info.Immediates[0] != "i32" {
		t.Errorf("Immediates = %v, want [\"i32\"]", info.Immediates)
	}
}

func TestI64ConstHasI64Immediate(t *testing.T) {
	info, ok := GetOpcodeByName("i64.const")
	if !ok {
		t.Fatal("i64.const not found")
	}
	if len(info.Immediates) != 1 || info.Immediates[0] != "i64" {
		t.Errorf("Immediates = %v, want [\"i64\"]", info.Immediates)
	}
}

func TestF32ConstHasF32Immediate(t *testing.T) {
	info, ok := GetOpcodeByName("f32.const")
	if !ok {
		t.Fatal("f32.const not found")
	}
	if len(info.Immediates) != 1 || info.Immediates[0] != "f32" {
		t.Errorf("Immediates = %v, want [\"f32\"]", info.Immediates)
	}
}

func TestF64ConstHasF64Immediate(t *testing.T) {
	info, ok := GetOpcodeByName("f64.const")
	if !ok {
		t.Fatal("f64.const not found")
	}
	if len(info.Immediates) != 1 || info.Immediates[0] != "f64" {
		t.Errorf("Immediates = %v, want [\"f64\"]", info.Immediates)
	}
}

func TestAllMemoryLoadsHaveMemarg(t *testing.T) {
	loadNames := []string{
		"i32.load", "i64.load", "f32.load", "f64.load",
		"i32.load8_s", "i32.load8_u", "i32.load16_s", "i32.load16_u",
		"i64.load8_s", "i64.load8_u", "i64.load16_s", "i64.load16_u",
		"i64.load32_s", "i64.load32_u",
	}
	for _, name := range loadNames {
		info, ok := GetOpcodeByName(name)
		if !ok {
			t.Errorf("%s: not found", name)
			continue
		}
		if len(info.Immediates) != 1 || info.Immediates[0] != "memarg" {
			t.Errorf("%s: Immediates = %v, want [\"memarg\"]", name, info.Immediates)
		}
	}
}

func TestAllMemoryStoresHaveMemarg(t *testing.T) {
	storeNames := []string{
		"i32.store", "i64.store", "f32.store", "f64.store",
		"i32.store8", "i32.store16", "i64.store8", "i64.store16", "i64.store32",
	}
	for _, name := range storeNames {
		info, ok := GetOpcodeByName(name)
		if !ok {
			t.Errorf("%s: not found", name)
			continue
		}
		if len(info.Immediates) != 1 || info.Immediates[0] != "memarg" {
			t.Errorf("%s: Immediates = %v, want [\"memarg\"]", name, info.Immediates)
		}
	}
}

func TestBrHasLabelidxImmediate(t *testing.T) {
	info, ok := GetOpcodeByName("br")
	if !ok {
		t.Fatal("br not found")
	}
	if len(info.Immediates) != 1 || info.Immediates[0] != "labelidx" {
		t.Errorf("Immediates = %v, want [\"labelidx\"]", info.Immediates)
	}
}

func TestBrTableHasVecLabelidxImmediate(t *testing.T) {
	info, ok := GetOpcodeByName("br_table")
	if !ok {
		t.Fatal("br_table not found")
	}
	if len(info.Immediates) != 1 || info.Immediates[0] != "vec_labelidx" {
		t.Errorf("Immediates = %v, want [\"vec_labelidx\"]", info.Immediates)
	}
}

func TestCallIndirectHasTwoImmediates(t *testing.T) {
	info, ok := GetOpcodeByName("call_indirect")
	if !ok {
		t.Fatal("call_indirect not found")
	}
	if len(info.Immediates) != 2 ||
		info.Immediates[0] != "typeidx" ||
		info.Immediates[1] != "tableidx" {
		t.Errorf("Immediates = %v, want [\"typeidx\",\"tableidx\"]", info.Immediates)
	}
}

func TestBlockHasBlocktypeImmediate(t *testing.T) {
	info, ok := GetOpcodeByName("block")
	if !ok {
		t.Fatal("block not found")
	}
	if len(info.Immediates) != 1 || info.Immediates[0] != "blocktype" {
		t.Errorf("Immediates = %v, want [\"blocktype\"]", info.Immediates)
	}
}

func TestMemorySizeHasMemidxImmediate(t *testing.T) {
	info, ok := GetOpcodeByName("memory.size")
	if !ok {
		t.Fatal("memory.size not found")
	}
	if len(info.Immediates) != 1 || info.Immediates[0] != "memidx" {
		t.Errorf("Immediates = %v, want [\"memidx\"]", info.Immediates)
	}
}

func TestI32AddHasNoImmediates(t *testing.T) {
	info, ok := GetOpcodeByName("i32.add")
	if !ok {
		t.Fatal("i32.add not found")
	}
	if len(info.Immediates) != 0 {
		t.Errorf("Immediates = %v, want []", info.Immediates)
	}
}

func TestConversionsHaveNoImmediates(t *testing.T) {
	info, ok := GetOpcodeByName("i32.wrap_i64")
	if !ok {
		t.Fatal("i32.wrap_i64 not found")
	}
	if len(info.Immediates) != 0 {
		t.Errorf("Immediates = %v, want []", info.Immediates)
	}
}

// ---------------------------------------------------------------------------
// CATEGORY TESTS
// ---------------------------------------------------------------------------

func TestAllOpcodesHaveValidCategory(t *testing.T) {
	for b, info := range Opcodes {
		if !validCategories[info.Category] {
			t.Errorf("opcode 0x%02X (%s): invalid category %q", b, info.Name, info.Category)
		}
	}
}

func TestAllOpcodesHaveNonEmptyName(t *testing.T) {
	for b, info := range Opcodes {
		if info.Name == "" {
			t.Errorf("opcode 0x%02X has empty name", b)
		}
	}
}

func TestControlCategory(t *testing.T) {
	for _, name := range []string{"unreachable", "call", "br_table", "end"} {
		info, ok := GetOpcodeByName(name)
		if !ok {
			t.Errorf("%s not found", name)
			continue
		}
		if info.Category != "control" {
			t.Errorf("%s: Category = %q, want \"control\"", name, info.Category)
		}
	}
}

func TestMemoryCategory(t *testing.T) {
	for _, name := range []string{"i32.load", "i64.store32", "memory.size", "memory.grow"} {
		info, ok := GetOpcodeByName(name)
		if !ok {
			t.Errorf("%s not found", name)
			continue
		}
		if info.Category != "memory" {
			t.Errorf("%s: Category = %q, want \"memory\"", name, info.Category)
		}
	}
}

func TestConversionCategory(t *testing.T) {
	for _, name := range []string{"i32.wrap_i64", "f64.promote_f32", "i32.reinterpret_f32"} {
		info, ok := GetOpcodeByName(name)
		if !ok {
			t.Errorf("%s not found", name)
			continue
		}
		if info.Category != "conversion" {
			t.Errorf("%s: Category = %q, want \"conversion\"", name, info.Category)
		}
	}
}

// ---------------------------------------------------------------------------
// CONSISTENCY TESTS
// ---------------------------------------------------------------------------

func TestEveryOpcodeReachableByName(t *testing.T) {
	for b, info := range Opcodes {
		byName, ok := OpcodesByName[info.Name]
		if !ok {
			t.Errorf("name %q (from opcode 0x%02X) not in OpcodesByName", info.Name, b)
			continue
		}
		if byName.Opcode != b {
			t.Errorf("OpcodesByName[%q].Opcode = 0x%02X, want 0x%02X", info.Name, byName.Opcode, b)
		}
	}
}

func TestEveryNameReachableByOpcode(t *testing.T) {
	for name, info := range OpcodesByName {
		byOpcode, ok := Opcodes[info.Opcode]
		if !ok {
			t.Errorf("opcode 0x%02X (from name %q) not in Opcodes", info.Opcode, name)
			continue
		}
		if byOpcode.Name != name {
			t.Errorf("Opcodes[0x%02X].Name = %q, want %q", info.Opcode, byOpcode.Name, name)
		}
	}
}

// ---------------------------------------------------------------------------
// SPOT-CHECK SPECIFIC OPCODES
// ---------------------------------------------------------------------------

func TestEndOpcode(t *testing.T) {
	info, ok := GetOpcode(0x0B)
	if !ok {
		t.Fatal("end (0x0B) not found")
	}
	if info.Name != "end" {
		t.Errorf("Name = %q, want \"end\"", info.Name)
	}
	if info.StackPop != 0 || info.StackPush != 0 {
		t.Errorf("stack effect = pop:%d push:%d, want pop:0 push:0", info.StackPop, info.StackPush)
	}
}

func TestI64DivS(t *testing.T) {
	info, ok := GetOpcodeByName("i64.div_s")
	if !ok {
		t.Fatal("i64.div_s not found")
	}
	if info.Opcode != 0x7F {
		t.Errorf("Opcode = 0x%02X, want 0x7F", info.Opcode)
	}
	if info.StackPop != 2 {
		t.Errorf("StackPop = %d, want 2", info.StackPop)
	}
}

func TestF32Nearest(t *testing.T) {
	info, ok := GetOpcodeByName("f32.nearest")
	if !ok {
		t.Fatal("f32.nearest not found")
	}
	if info.Opcode != 0x90 {
		t.Errorf("Opcode = 0x%02X, want 0x90", info.Opcode)
	}
}

func TestI32Popcnt(t *testing.T) {
	info, ok := GetOpcodeByName("i32.popcnt")
	if !ok {
		t.Fatal("i32.popcnt not found")
	}
	if info.Opcode != 0x69 {
		t.Errorf("Opcode = 0x%02X, want 0x69", info.Opcode)
	}
	if info.StackPop != 1 || info.StackPush != 1 {
		t.Errorf("stack = pop:%d push:%d, want pop:1 push:1", info.StackPop, info.StackPush)
	}
}

func TestI64Rotl(t *testing.T) {
	info, ok := GetOpcodeByName("i64.rotl")
	if !ok {
		t.Fatal("i64.rotl not found")
	}
	if info.Opcode != 0x89 {
		t.Errorf("Opcode = 0x%02X, want 0x89", info.Opcode)
	}
}

func TestF64PromoteF32(t *testing.T) {
	info, ok := GetOpcodeByName("f64.promote_f32")
	if !ok {
		t.Fatal("f64.promote_f32 not found")
	}
	if info.Opcode != 0xBB {
		t.Errorf("Opcode = 0x%02X, want 0xBB", info.Opcode)
	}
	if info.Category != "conversion" {
		t.Errorf("Category = %q, want \"conversion\"", info.Category)
	}
}
