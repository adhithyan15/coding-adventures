package fontparser_test

import (
	"encoding/binary"
	"os"
	"path/filepath"
	"runtime"
	"testing"

	fp "github.com/adhithyan15/coding-adventures/code/packages/go/font-parser"
)

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

// interBytes loads Inter Regular from the shared fixtures directory.
// This file is at code/packages/go/font-parser/font_parser_test.go.
// Fixtures are at code/fixtures/fonts/.
func interBytes(t *testing.T) []byte {
	t.Helper()
	_, filename, _, _ := runtime.Caller(0)
	// Go up 4 levels: font-parser → go → packages → code
	root := filepath.Join(filepath.Dir(filename), "..", "..", "..")
	fontPath := filepath.Join(root, "fixtures", "fonts", "Inter-Regular.ttf")
	data, err := os.ReadFile(fontPath)
	if err != nil {
		t.Fatalf("could not read Inter-Regular.ttf: %v", err)
	}
	return data
}

// buildSyntheticFont builds a minimal valid OpenType font with a kern table.
//
// Tables: head, hhea, maxp, cmap (Format 4, sentinel), hmtx, kern.
// Used to test kern binary search logic without an external font file.
func buildSyntheticFont(t *testing.T, pairs [][3]int16) []byte {
	t.Helper()

	w16 := func(buf []byte, off int, v uint16) {
		binary.BigEndian.PutUint16(buf[off:], v)
	}
	wi16 := func(buf []byte, off int, v int16) {
		binary.BigEndian.PutUint16(buf[off:], uint16(v))
	}
	w32 := func(buf []byte, off int, v uint32) {
		binary.BigEndian.PutUint32(buf[off:], v)
	}

	const numTables = 6
	const dirSize   = 12 + numTables*16
	const headLen   = 54
	const hheaLen   = 36
	const maxpLen   = 6
	const cmapLen   = 36 // 4 + 8 + 24
	const hmtxLen   = 5 * 4
	nPairs := len(pairs)
	kernLen := 4 + 6 + 8 + nPairs*6

	headOff := dirSize
	hheaOff := headOff + headLen
	maxpOff := hheaOff + hheaLen
	cmapOff := maxpOff + maxpLen
	hmtxOff := cmapOff + cmapLen
	kernOff := hmtxOff + hmtxLen
	totalSize := kernOff + kernLen

	buf := make([]byte, totalSize)

	// ── Offset Table ─────────────────────────────────────────────────────────
	w32(buf, 0, 0x00010000) // sfntVersion
	w16(buf, 4, numTables)
	w16(buf, 6, 64)  // searchRange
	w16(buf, 8, 2)   // entrySelector
	w16(buf, 10, 32) // rangeShift

	// ── Table Records (sorted: cmap < head < hhea < hmtx < kern < maxp) ─────
	type recDef struct {
		tag string
		off int
		len int
	}
	recs := []recDef{
		{"cmap", cmapOff, cmapLen},
		{"head", headOff, headLen},
		{"hhea", hheaOff, hheaLen},
		{"hmtx", hmtxOff, hmtxLen},
		{"kern", kernOff, kernLen},
		{"maxp", maxpOff, maxpLen},
	}
	for i, r := range recs {
		base := 12 + i*16
		copy(buf[base:], r.tag)
		w32(buf, base+4, 0)         // checksum
		w32(buf, base+8, uint32(r.off))
		w32(buf, base+12, uint32(r.len))
	}

	// ── head table ───────────────────────────────────────────────────────────
	p := headOff
	w32(buf, p, 0x00010000)    // version
	w32(buf, p+4, 0x00010000)  // fontRevision
	w32(buf, p+8, 0)           // checksumAdjustment
	w32(buf, p+12, 0x5F0F3CF5) // magicNumber ← sentinel
	w16(buf, p+16, 0)          // flags
	w16(buf, p+18, 1000)       // unitsPerEm
	// bytes 20..36: created + modified = zeros
	wi16(buf, p+50, 0) // indexToLocFormat

	// ── hhea table ───────────────────────────────────────────────────────────
	p = hheaOff
	w32(buf, p, 0x00010000) // version
	wi16(buf, p+4, 800)     // ascender
	wi16(buf, p+6, -200)    // descender
	wi16(buf, p+8, 0)       // lineGap
	w16(buf, p+10, 1000)    // advanceWidthMax
	wi16(buf, p+32, 0)      // metricDataFormat
	w16(buf, p+34, 5)       // numberOfHMetrics

	// ── maxp table ───────────────────────────────────────────────────────────
	p = maxpOff
	w32(buf, p, 0x00005000) // version 0.5
	w16(buf, p+4, 5)        // numGlyphs

	// ── cmap table ───────────────────────────────────────────────────────────
	p = cmapOff
	w16(buf, p, 0)   // version
	w16(buf, p+2, 1) // numSubtables
	// Encoding record: platform 3, encoding 1, offset 12
	w16(buf, p+4, 3)
	w16(buf, p+6, 1)
	w32(buf, p+8, 12)
	// Format 4 subtable, segCount=1 (sentinel only)
	sub := p + 12
	w16(buf, sub, 4)       // format
	w16(buf, sub+2, 24)    // length
	w16(buf, sub+4, 0)     // language
	w16(buf, sub+6, 2)     // segCountX2
	w16(buf, sub+8, 2)     // searchRange
	w16(buf, sub+10, 0)    // entrySelector
	w16(buf, sub+12, 0)    // rangeShift
	w16(buf, sub+14, 0xFFFF) // endCode[0] sentinel
	w16(buf, sub+16, 0)    // reservedPad
	w16(buf, sub+18, 0xFFFF) // startCode[0] sentinel
	wi16(buf, sub+20, 1)   // idDelta[0]
	w16(buf, sub+22, 0)    // idRangeOffset[0]

	// ── hmtx table ───────────────────────────────────────────────────────────
	p = hmtxOff
	for i := 0; i < 5; i++ {
		w16(buf, p+i*4, 600)
		wi16(buf, p+i*4+2, 50)
	}

	// ── kern table ───────────────────────────────────────────────────────────
	p = kernOff
	w16(buf, p, 0)   // version
	w16(buf, p+2, 1) // nTables
	subLen := 6 + 8 + nPairs*6
	w16(buf, p+4, 0)              // subtable version
	w16(buf, p+6, uint16(subLen)) // subtable length
	w16(buf, p+8, 0x0001)         // coverage: format 0, horizontal
	w16(buf, p+10, uint16(nPairs)) // nPairs
	w16(buf, p+12, 0)
	w16(buf, p+14, 0)
	w16(buf, p+16, 0)

	// Sort pairs by composite key.
	sorted := make([][3]int16, len(pairs))
	copy(sorted, pairs)
	for i := 0; i < len(sorted)-1; i++ {
		for j := i + 1; j < len(sorted); j++ {
			ki := (uint32(sorted[i][0]) << 16) | uint32(sorted[i][1])
			kj := (uint32(sorted[j][0]) << 16) | uint32(sorted[j][1])
			if ki > kj {
				sorted[i], sorted[j] = sorted[j], sorted[i]
			}
		}
	}
	pairOff := p + 18
	for _, pair := range sorted {
		w16(buf, pairOff, uint16(pair[0]))
		w16(buf, pairOff+2, uint16(pair[1]))
		wi16(buf, pairOff+4, pair[2])
		pairOff += 6
	}

	return buf
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Load
// ─────────────────────────────────────────────────────────────────────────────

func TestLoad_EmptyBuffer(t *testing.T) {
	_, err := fp.Load(nil)
	if err == nil {
		t.Fatal("expected error for nil buffer")
	}
	var fe *fp.FontError
	if !errAs(err, &fe) || fe.Kind != fp.ErrBufferTooShort {
		t.Fatalf("expected ErrBufferTooShort, got %v", err)
	}
}

func TestLoad_WrongMagic(t *testing.T) {
	buf := make([]byte, 256)
	binary.BigEndian.PutUint32(buf, 0xDEADBEEF)
	_, err := fp.Load(buf)
	if err == nil {
		t.Fatal("expected error for wrong magic")
	}
	var fe *fp.FontError
	if !errAs(err, &fe) || fe.Kind != fp.ErrInvalidMagic {
		t.Fatalf("expected ErrInvalidMagic, got %v", err)
	}
}

func TestLoad_InterRegular(t *testing.T) {
	data := interBytes(t)
	font, err := fp.Load(data)
	if err != nil {
		t.Fatalf("Load failed: %v", err)
	}
	if font == nil {
		t.Fatal("Load returned nil font")
	}
}

func TestLoad_SyntheticFont(t *testing.T) {
	data := buildSyntheticFont(t, [][3]int16{{1, 2, -140}})
	_, err := fp.Load(data)
	if err != nil {
		t.Fatalf("Load synthetic font failed: %v", err)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: GetFontMetrics
// ─────────────────────────────────────────────────────────────────────────────

func TestGetFontMetrics_UnitsPerEm(t *testing.T) {
	font := mustLoadInter(t)
	m := fp.GetFontMetrics(font)
	if m.UnitsPerEm != 2048 {
		t.Fatalf("UnitsPerEm = %d, want 2048", m.UnitsPerEm)
	}
}

func TestGetFontMetrics_FamilyName(t *testing.T) {
	font := mustLoadInter(t)
	m := fp.GetFontMetrics(font)
	if m.FamilyName != "Inter" {
		t.Fatalf("FamilyName = %q, want %q", m.FamilyName, "Inter")
	}
}

func TestGetFontMetrics_SubfamilyName(t *testing.T) {
	font := mustLoadInter(t)
	m := fp.GetFontMetrics(font)
	if m.SubfamilyName != "Regular" {
		t.Fatalf("SubfamilyName = %q, want %q", m.SubfamilyName, "Regular")
	}
}

func TestGetFontMetrics_AscenderPositive(t *testing.T) {
	font := mustLoadInter(t)
	m := fp.GetFontMetrics(font)
	if m.Ascender <= 0 {
		t.Fatalf("Ascender = %d, want > 0", m.Ascender)
	}
}

func TestGetFontMetrics_DescenderNonPositive(t *testing.T) {
	font := mustLoadInter(t)
	m := fp.GetFontMetrics(font)
	if m.Descender > 0 {
		t.Fatalf("Descender = %d, want ≤ 0", m.Descender)
	}
}

func TestGetFontMetrics_NumGlyphs(t *testing.T) {
	font := mustLoadInter(t)
	m := fp.GetFontMetrics(font)
	if m.NumGlyphs < 100 {
		t.Fatalf("NumGlyphs = %d, want > 100", m.NumGlyphs)
	}
}

func TestGetFontMetrics_XHeight(t *testing.T) {
	font := mustLoadInter(t)
	m := fp.GetFontMetrics(font)
	if m.XHeight == nil {
		t.Fatal("XHeight = nil, want non-nil")
	}
	if *m.XHeight <= 0 {
		t.Fatalf("XHeight = %d, want > 0", *m.XHeight)
	}
}

func TestGetFontMetrics_CapHeight(t *testing.T) {
	font := mustLoadInter(t)
	m := fp.GetFontMetrics(font)
	if m.CapHeight == nil {
		t.Fatal("CapHeight = nil, want non-nil")
	}
	if *m.CapHeight <= 0 {
		t.Fatalf("CapHeight = %d, want > 0", *m.CapHeight)
	}
}

func TestGetFontMetrics_SyntheticUnknownFamily(t *testing.T) {
	font := mustLoadSynthetic(t, nil)
	m := fp.GetFontMetrics(font)
	if m.FamilyName != "(unknown)" {
		t.Fatalf("FamilyName = %q, want \"(unknown)\"", m.FamilyName)
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: GlyphID
// ─────────────────────────────────────────────────────────────────────────────

func TestGlyphID_LetterA(t *testing.T) {
	font := mustLoadInter(t)
	gid, ok := fp.GlyphID(font, 'A')
	if !ok {
		t.Fatal("GlyphID('A') = not found")
	}
	if gid == 0 {
		t.Fatal("GlyphID('A') = 0 (should be positive)")
	}
}

func TestGlyphID_LetterV(t *testing.T) {
	font := mustLoadInter(t)
	_, ok := fp.GlyphID(font, 'V')
	if !ok {
		t.Fatal("GlyphID('V') = not found")
	}
}

func TestGlyphID_Space(t *testing.T) {
	font := mustLoadInter(t)
	_, ok := fp.GlyphID(font, ' ')
	if !ok {
		t.Fatal("GlyphID(' ') = not found")
	}
}

func TestGlyphID_AVDiffer(t *testing.T) {
	font := mustLoadInter(t)
	gidA, _ := fp.GlyphID(font, 'A')
	gidV, _ := fp.GlyphID(font, 'V')
	if gidA == gidV {
		t.Fatalf("GlyphID('A') == GlyphID('V') = %d (should differ)", gidA)
	}
}

func TestGlyphID_AboveBMP(t *testing.T) {
	font := mustLoadInter(t)
	_, ok := fp.GlyphID(font, 0x10000)
	if ok {
		t.Fatal("GlyphID(0x10000) = found (should not be in BMP)")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: GetGlyphMetrics
// ─────────────────────────────────────────────────────────────────────────────

func TestGetGlyphMetrics_LetterA(t *testing.T) {
	font := mustLoadInter(t)
	gid, _ := fp.GlyphID(font, 'A')
	gm, ok := fp.GetGlyphMetrics(font, gid)
	if !ok || gm == nil {
		t.Fatal("GetGlyphMetrics('A') = nil")
	}
	if gm.AdvanceWidth == 0 {
		t.Fatal("AdvanceWidth = 0")
	}
}

func TestGetGlyphMetrics_OutOfRange(t *testing.T) {
	font := mustLoadInter(t)
	m := fp.GetFontMetrics(font)
	_, ok := fp.GetGlyphMetrics(font, m.NumGlyphs)
	if ok {
		t.Fatal("GetGlyphMetrics(numGlyphs) should return false")
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// Tests: Kerning
// ─────────────────────────────────────────────────────────────────────────────

func TestKerning_InterNoKernTable(t *testing.T) {
	// Inter v4.0 uses GPOS; legacy kern table absent.
	font := mustLoadInter(t)
	gidA, _ := fp.GlyphID(font, 'A')
	gidV, _ := fp.GlyphID(font, 'V')
	kern := fp.Kerning(font, gidA, gidV)
	if kern != 0 {
		t.Fatalf("Kerning(A,V) for Inter = %d, want 0 (no kern table)", kern)
	}
}

func TestKerning_SyntheticPairFound(t *testing.T) {
	font := mustLoadSynthetic(t, [][3]int16{{1, 2, -140}, {3, 4, 80}})
	if v := fp.Kerning(font, 1, 2); v != -140 {
		t.Fatalf("Kerning(1,2) = %d, want -140", v)
	}
}

func TestKerning_SyntheticSecondPair(t *testing.T) {
	font := mustLoadSynthetic(t, [][3]int16{{1, 2, -140}, {3, 4, 80}})
	if v := fp.Kerning(font, 3, 4); v != 80 {
		t.Fatalf("Kerning(3,4) = %d, want 80", v)
	}
}

func TestKerning_SyntheticAbsentPair(t *testing.T) {
	font := mustLoadSynthetic(t, [][3]int16{{1, 2, -140}, {3, 4, 80}})
	if v := fp.Kerning(font, 1, 4); v != 0 {
		t.Fatalf("Kerning(1,4) = %d, want 0 (not in table)", v)
	}
}

func TestKerning_ReversedPair(t *testing.T) {
	font := mustLoadSynthetic(t, [][3]int16{{1, 2, -140}})
	if v := fp.Kerning(font, 2, 1); v != 0 {
		t.Fatalf("Kerning(2,1) = %d, want 0 (reversed not in table)", v)
	}
}

func TestGlyphID_NegativeCodpoint(t *testing.T) {
	font := mustLoadInter(t)
	_, ok := fp.GlyphID(font, -1)
	if ok {
		t.Fatal("GlyphID(-1) = found (should be false)")
	}
}

func TestGetGlyphMetrics_AdvanceWidthInRange(t *testing.T) {
	font := mustLoadInter(t)
	gid, ok := fp.GlyphID(font, 'A')
	if !ok {
		t.Fatal("GlyphID('A') not found")
	}
	gm, ok := fp.GetGlyphMetrics(font, gid)
	if !ok {
		t.Fatal("GetGlyphMetrics('A') = false")
	}
	if gm.AdvanceWidth < 100 || gm.AdvanceWidth > 2400 {
		t.Fatalf("AdvanceWidth = %d outside expected range 100–2400", gm.AdvanceWidth)
	}
}

func TestKerning_NoKernTableSynthetic(t *testing.T) {
	// A minimal font with no kern table (load Inter which has no kern table).
	font := mustLoadInter(t)
	v := fp.Kerning(font, 0, 0)
	if v != 0 {
		t.Fatalf("Kerning(0,0) = %d, want 0", v)
	}
}

func TestFontError_ErrorString(t *testing.T) {
	// Covers FontError.Error() method.
	err := &fp.FontError{Kind: fp.ErrInvalidMagic, Message: "test error message"}
	if err.Error() != "test error message" {
		t.Fatalf("Error() = %q, want %q", err.Error(), "test error message")
	}
}

func TestLoad_MissingRequiredTable(t *testing.T) {
	// Build a buffer with a valid sfntVersion but numTables=0 (no tables).
	// This will fail on the first requireTable call for "head".
	buf := make([]byte, 12)
	binary.BigEndian.PutUint32(buf, 0x00010000) // sfntVersion
	binary.BigEndian.PutUint16(buf[4:], 0)      // numTables = 0
	_, err := fp.Load(buf)
	if err == nil {
		t.Fatal("expected error for font with no tables")
	}
	var fe *fp.FontError
	if !errAs(err, &fe) || fe.Kind != fp.ErrTableNotFound {
		t.Fatalf("expected ErrTableNotFound, got %v", err)
	}
}

func TestGetGlyphMetrics_SharedAdvance(t *testing.T) {
	// In the synthetic font, numHMetrics=5 and numGlyphs=5, so there are
	// no shared-advance glyphs. We need a font where gid >= numHMetrics.
	// Build a synthetic font with numHMetrics=2, numGlyphs=5.
	// This exercises the "shared advance" branch in GetGlyphMetrics.
	t.Helper()
	// We'll directly test that GetGlyphMetrics works for a glyph ID
	// that exceeds numHMetrics. Build manually.
	data := buildFontWithSharedAdvance(t)
	font, err := fp.Load(data)
	if err != nil {
		t.Fatalf("Load: %v", err)
	}
	// glyph 3 should use shared advance (numHMetrics=2, glyph 3 >= 2)
	gm, ok := fp.GetGlyphMetrics(font, 3)
	if !ok {
		t.Fatal("GetGlyphMetrics(3) = false, want true")
	}
	if gm.AdvanceWidth != 700 { // last advance from hMetrics[1]
		t.Fatalf("AdvanceWidth = %d, want 700", gm.AdvanceWidth)
	}
}

// buildFontWithSharedAdvance builds a minimal font where numHMetrics=2
// but numGlyphs=5, exercising the shared-advance path in GetGlyphMetrics.
func buildFontWithSharedAdvance(t *testing.T) []byte {
	t.Helper()

	w16 := func(buf []byte, off int, v uint16) { binary.BigEndian.PutUint16(buf[off:], v) }
	wi16 := func(buf []byte, off int, v int16) { binary.BigEndian.PutUint16(buf[off:], uint16(v)) }
	w32 := func(buf []byte, off int, v uint32) { binary.BigEndian.PutUint32(buf[off:], v) }

	const (
		numTables    = 5 // head, hhea, maxp, cmap, hmtx (no kern)
		dirSize      = 12 + numTables*16
		headLen      = 54
		hheaLen      = 36
		maxpLen      = 6
		cmapLen      = 36
		numHMetrics  = 2
		numGlyphs    = 5
	)
	// hmtx: numHMetrics full records + (numGlyphs-numHMetrics) lsb-only
	hmtxLen := numHMetrics*4 + (numGlyphs-numHMetrics)*2

	headOff := dirSize
	hheaOff := headOff + headLen
	maxpOff := hheaOff + hheaLen
	cmapOff := maxpOff + maxpLen
	hmtxOff := cmapOff + cmapLen
	total   := hmtxOff + hmtxLen

	buf := make([]byte, total)

	w32(buf, 0, 0x00010000) // sfntVersion
	w16(buf, 4, numTables)
	w16(buf, 6, 64); w16(buf, 8, 2); w16(buf, 10, 32)

	type recDef struct{ tag string; off, len int }
	recs := []recDef{
		{"cmap", cmapOff, cmapLen},
		{"head", headOff, headLen},
		{"hhea", hheaOff, hheaLen},
		{"hmtx", hmtxOff, hmtxLen},
		{"maxp", maxpOff, maxpLen},
	}
	for i, r := range recs {
		base := 12 + i*16
		copy(buf[base:], r.tag)
		w32(buf, base+4, 0); w32(buf, base+8, uint32(r.off)); w32(buf, base+12, uint32(r.len))
	}

	// head
	p := headOff
	w32(buf, p, 0x00010000); w32(buf, p+4, 0x00010000); w32(buf, p+8, 0)
	w32(buf, p+12, 0x5F0F3CF5); w16(buf, p+16, 0); w16(buf, p+18, 1000)

	// hhea
	p = hheaOff
	w32(buf, p, 0x00010000); wi16(buf, p+4, 800); wi16(buf, p+6, -200); wi16(buf, p+8, 0)
	w16(buf, p+10, 1000); wi16(buf, p+32, 0); w16(buf, p+34, numHMetrics)

	// maxp
	p = maxpOff; w32(buf, p, 0x00005000); w16(buf, p+4, numGlyphs)

	// cmap (same sentinel as buildSyntheticFont)
	p = cmapOff
	w16(buf, p, 0); w16(buf, p+2, 1)
	w16(buf, p+4, 3); w16(buf, p+6, 1); w32(buf, p+8, 12)
	sub := p + 12
	w16(buf, sub, 4); w16(buf, sub+2, 24); w16(buf, sub+4, 0)
	w16(buf, sub+6, 2); w16(buf, sub+8, 2); w16(buf, sub+10, 0); w16(buf, sub+12, 0)
	w16(buf, sub+14, 0xFFFF); w16(buf, sub+16, 0); w16(buf, sub+18, 0xFFFF)
	wi16(buf, sub+20, 1); w16(buf, sub+22, 0)

	// hmtx: 2 full records + 3 lsb-only
	p = hmtxOff
	w16(buf, p+0, 600); wi16(buf, p+2, 50)   // glyph 0: advance=600
	w16(buf, p+4, 700); wi16(buf, p+6, 60)   // glyph 1: advance=700 (last advance)
	wi16(buf, p+8, 55)                         // glyph 2: lsb=55, shared advance=700
	wi16(buf, p+10, 45)                        // glyph 3: lsb=45, shared advance=700
	wi16(buf, p+12, 35)                        // glyph 4: lsb=35, shared advance=700

	return buf
}

// ─────────────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────────────

func mustLoadInter(t *testing.T) *fp.FontFile {
	t.Helper()
	font, err := fp.Load(interBytes(t))
	if err != nil {
		t.Fatalf("Load Inter: %v", err)
	}
	return font
}

func mustLoadSynthetic(t *testing.T, pairs [][3]int16) *fp.FontFile {
	t.Helper()
	data := buildSyntheticFont(t, pairs)
	font, err := fp.Load(data)
	if err != nil {
		t.Fatalf("Load synthetic: %v", err)
	}
	return font
}

// errAs mirrors errors.As without importing "errors" in test file.
func errAs(err error, target **fp.FontError) bool {
	var fe *fp.FontError
	if x, ok := err.(*fp.FontError); ok {
		fe = x
	}
	if fe == nil {
		return false
	}
	*target = fe
	return true
}
