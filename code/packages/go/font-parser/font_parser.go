// Package fontparser parses OpenType/TrueType font metrics from raw bytes.
//
// # What this package does
//
// An OpenType font file is a binary table database. The first few bytes list
// all named tables and where each one lives. This package reads the subset of
// tables needed to measure text without touching the OS font stack:
//
//   - head  — unitsPerEm, magicNumber validation
//   - hhea  — ascender / descender / lineGap / numberOfHMetrics
//   - maxp  — numGlyphs
//   - cmap  — Format 4 Unicode BMP → glyph ID
//   - hmtx  — advance width + left side bearing per glyph
//   - kern  — Format 0 kerning pairs
//   - name  — family / subfamily names (UTF-16 BE)
//   - OS/2  — typographic metrics + xHeight / capHeight (version ≥ 2)
//
// # Usage
//
//	data, _ := os.ReadFile("Inter-Regular.ttf")
//	font, err := fontparser.Load(data)
//	if err != nil { log.Fatal(err) }
//
//	m := fontparser.FontMetrics(font)
//	fmt.Println(m.UnitsPerEm)  // 2048 for Inter
//	fmt.Println(m.FamilyName)  // "Inter"
//
//	gidA, _ := fontparser.GlyphID(font, 'A')
//	gidV, _ := fontparser.GlyphID(font, 'V')
//	kern := fontparser.Kerning(font, gidA, gidV) // negative
package fontparser

import (
	"encoding/binary"
	"errors"
	"fmt"
	"unicode/utf16"
)

// ─────────────────────────────────────────────────────────────────────────────
// Error types
// ─────────────────────────────────────────────────────────────────────────────

// FontErrorKind classifies parse failures.
type FontErrorKind int

const (
	ErrInvalidMagic          FontErrorKind = iota
	ErrInvalidHeadMagic
	ErrTableNotFound
	ErrBufferTooShort
	ErrUnsupportedCmapFormat
)

// FontError is returned when font bytes cannot be parsed.
type FontError struct {
	Kind    FontErrorKind
	Message string
}

func (e *FontError) Error() string { return e.Message }

func errBufferTooShort(op string) error {
	return &FontError{Kind: ErrBufferTooShort, Message: fmt.Sprintf("%s: buffer too short", op)}
}

// ─────────────────────────────────────────────────────────────────────────────
// Public metric types
// ─────────────────────────────────────────────────────────────────────────────

// FontMetrics holds global typographic metrics.
//
// All integer fields are in design units. Convert to pixels:
//
//	pixels = designUnits * fontSizePx / UnitsPerEm
//
// For Inter Regular at 16px: 16 / 2048 = 0.0078125 px per design unit.
type Metrics struct {
	// UnitsPerEm is the fundamental scale of the font's coordinate system.
	// Inter Regular uses 2048; older PostScript-derived fonts use 1000.
	UnitsPerEm uint16

	// Ascender is the distance from the baseline to the top of the tallest
	// glyph (positive). Prefers OS/2 typoAscender over hhea.ascender.
	Ascender int16

	// Descender is the distance below the baseline (negative, e.g. -512).
	// Prefers OS/2 typoDescender over hhea.descender.
	Descender int16

	// LineGap is extra inter-line spacing. Often 0.
	// Natural line height = Ascender - Descender + LineGap.
	LineGap int16

	// XHeight is the height of lowercase 'x' in design units.
	// nil if OS/2 table is absent or has version < 2.
	XHeight *int16

	// CapHeight is the height of uppercase 'H' in design units.
	// nil if OS/2 table is absent or has version < 2.
	CapHeight *int16

	// NumGlyphs is the total number of glyphs in the font.
	NumGlyphs uint16

	// FamilyName is the font family name, e.g. "Inter".
	FamilyName string

	// SubfamilyName is the style name, e.g. "Regular", "Bold Italic".
	SubfamilyName string
}

// GlyphMetrics holds per-glyph horizontal metrics in design units.
type GlyphMetrics struct {
	// AdvanceWidth is the horizontal distance to advance the pen.
	AdvanceWidth uint16

	// LeftSideBearing is the space between the pen and the left ink edge.
	// Positive = ink to the right of the pen (normal).
	// Negative = ink to the left of the pen (rare).
	LeftSideBearing int16
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal parsed table offsets
// ─────────────────────────────────────────────────────────────────────────────

type tables struct {
	head uint32
	hhea uint32
	maxp uint32
	cmap uint32
	hmtx uint32
	kern *uint32 // nil = absent
	name *uint32
	os2  *uint32
}

// ─────────────────────────────────────────────────────────────────────────────
// FontFile
// ─────────────────────────────────────────────────────────────────────────────

// FontFile is an opaque handle to a parsed font.
// Created by Load. Pass to the metric functions.
type FontFile struct {
	data   []byte
	tables tables
}

// ─────────────────────────────────────────────────────────────────────────────
// Big-endian reading helpers
// ─────────────────────────────────────────────────────────────────────────────
//
// All OpenType fields are big-endian. encoding/binary.BigEndian handles this.

func readU16(buf []byte, off int) (uint16, error) {
	if off+2 > len(buf) {
		return 0, errBufferTooShort("readU16")
	}
	return binary.BigEndian.Uint16(buf[off:]), nil
}

func readI16(buf []byte, off int) (int16, error) {
	v, err := readU16(buf, off)
	return int16(v), err
}

func readU32(buf []byte, off int) (uint32, error) {
	if off+4 > len(buf) {
		return 0, errBufferTooShort("readU32")
	}
	return binary.BigEndian.Uint32(buf[off:]), nil
}

// ─────────────────────────────────────────────────────────────────────────────
// Table directory
// ─────────────────────────────────────────────────────────────────────────────

func findTable(buf []byte, numTables uint16, tag [4]byte) *uint32 {
	for i := 0; i < int(numTables); i++ {
		rec := 12 + i*16
		if len(buf) < rec+16 {
			return nil
		}
		if buf[rec] == tag[0] && buf[rec+1] == tag[1] &&
			buf[rec+2] == tag[2] && buf[rec+3] == tag[3] {
			off := binary.BigEndian.Uint32(buf[rec+8:])
			v := off
			return &v
		}
	}
	return nil
}

func requireTable(buf []byte, numTables uint16, tag [4]byte, name string) (uint32, error) {
	p := findTable(buf, numTables, tag)
	if p == nil {
		return 0, &FontError{Kind: ErrTableNotFound, Message: fmt.Sprintf("required table '%s' not found", name)}
	}
	return *p, nil
}

// ─────────────────────────────────────────────────────────────────────────────
// Load
// ─────────────────────────────────────────────────────────────────────────────

// Load parses raw font bytes and returns a FontFile handle.
//
// Returns a *FontError if the bytes are not a valid OpenType/TrueType font,
// a required table is missing, or any byte read goes out of bounds.
//
//	data, _ := os.ReadFile("Inter-Regular.ttf")
//	font, err := fontparser.Load(data)
func Load(data []byte) (*FontFile, error) {
	if len(data) < 12 {
		return nil, errBufferTooShort("Load")
	}

	// sfntVersion: 0x00010000 (TrueType) or 0x4F54544F ("OTTO", CFF).
	sfntVersion := binary.BigEndian.Uint32(data[:4])
	if sfntVersion != 0x00010000 && sfntVersion != 0x4F54544F {
		return nil, &FontError{
			Kind:    ErrInvalidMagic,
			Message: fmt.Sprintf("invalid sfntVersion 0x%08X", sfntVersion),
		}
	}

	numTables := binary.BigEndian.Uint16(data[4:])

	t := tables{}
	var err error

	if t.head, err = requireTable(data, numTables, [4]byte{'h', 'e', 'a', 'd'}, "head"); err != nil {
		return nil, err
	}
	if t.hhea, err = requireTable(data, numTables, [4]byte{'h', 'h', 'e', 'a'}, "hhea"); err != nil {
		return nil, err
	}
	if t.maxp, err = requireTable(data, numTables, [4]byte{'m', 'a', 'x', 'p'}, "maxp"); err != nil {
		return nil, err
	}
	if t.cmap, err = requireTable(data, numTables, [4]byte{'c', 'm', 'a', 'p'}, "cmap"); err != nil {
		return nil, err
	}
	if t.hmtx, err = requireTable(data, numTables, [4]byte{'h', 'm', 't', 'x'}, "hmtx"); err != nil {
		return nil, err
	}
	t.kern = findTable(data, numTables, [4]byte{'k', 'e', 'r', 'n'})
	t.name = findTable(data, numTables, [4]byte{'n', 'a', 'm', 'e'})
	t.os2  = findTable(data, numTables, [4]byte{'O', 'S', '/', '2'})

	// Validate head.magicNumber sentinel (at offset 12 within the head table).
	magic, err := readU32(data, int(t.head)+12)
	if err != nil {
		return nil, err
	}
	if magic != 0x5F0F3CF5 {
		return nil, &FontError{Kind: ErrInvalidHeadMagic, Message: fmt.Sprintf("invalid head.magicNumber 0x%08X", magic)}
	}

	// Copy the data so the caller can release their slice.
	buf := make([]byte, len(data))
	copy(buf, data)

	return &FontFile{data: buf, tables: t}, nil
}

// ─────────────────────────────────────────────────────────────────────────────
// GetFontMetrics
// ─────────────────────────────────────────────────────────────────────────────

// GetFontMetrics returns global typographic metrics.
//
// Prefers OS/2 typographic values over hhea values when the OS/2 table is
// present — matches CSS, Core Text, and DirectWrite behaviour.
func GetFontMetrics(font *FontFile) *Metrics {
	buf := font.data
	t := font.tables

	// head: unitsPerEm at offset 18.
	unitsPerEm, _ := readU16(buf, int(t.head)+18)

	// hhea: fallback values.
	hheaAscender, _  := readI16(buf, int(t.hhea)+4)
	hheaDescender, _ := readI16(buf, int(t.hhea)+6)
	hheaLineGap, _   := readI16(buf, int(t.hhea)+8)

	// maxp: numGlyphs at offset 4.
	numGlyphs, _ := readU16(buf, int(t.maxp)+4)

	// OS/2: prefer typo metrics.
	ascender  := hheaAscender
	descender := hheaDescender
	lineGap   := hheaLineGap
	var xHeight, capHeight *int16

	if t.os2 != nil {
		base := int(*t.os2)
		version, _ := readU16(buf, base)
		if a, err := readI16(buf, base+68); err == nil {
			ascender = a
		}
		if d, err := readI16(buf, base+70); err == nil {
			descender = d
		}
		if g, err := readI16(buf, base+72); err == nil {
			lineGap = g
		}
		if version >= 2 {
			if xh, err := readI16(buf, base+86); err == nil {
				xHeight = &xh
			}
			if ch, err := readI16(buf, base+88); err == nil {
				capHeight = &ch
			}
		}
	}

	familyName    := readNameString(buf, t.name, 1)
	subfamilyName := readNameString(buf, t.name, 2)
	if familyName == "" {
		familyName = "(unknown)"
	}
	if subfamilyName == "" {
		subfamilyName = "(unknown)"
	}

	return &Metrics{
		UnitsPerEm:    unitsPerEm,
		Ascender:      ascender,
		Descender:     descender,
		LineGap:       lineGap,
		XHeight:       xHeight,
		CapHeight:     capHeight,
		NumGlyphs:     numGlyphs,
		FamilyName:    familyName,
		SubfamilyName: subfamilyName,
	}
}

// ─────────────────────────────────────────────────────────────────────────────
// GlyphID — cmap Format 4 lookup
// ─────────────────────────────────────────────────────────────────────────────

// GlyphID maps a Unicode codepoint to a glyph ID.
//
// Returns (glyphID, true) if found, (0, false) if not in the font or outside
// the BMP (> 0xFFFF).
func GlyphID(font *FontFile, codepoint rune) (uint16, bool) {
	if codepoint < 0 || codepoint > 0xFFFF {
		return 0, false
	}
	cp := uint16(codepoint)
	buf := font.data
	cmapOff := int(font.tables.cmap)

	// ── Find the Format 4 subtable ──────────────────────────────────────────
	numSubtables, err := readU16(buf, cmapOff+2)
	if err != nil {
		return 0, false
	}

	subtableAbs := -1
	for i := 0; i < int(numSubtables); i++ {
		rec := cmapOff + 4 + i*8
		platformID, err1 := readU16(buf, rec)
		encodingID, err2 := readU16(buf, rec+2)
		subOff, err3 := readU32(buf, rec+4)
		if err1 != nil || err2 != nil || err3 != nil {
			continue
		}
		if platformID == 3 && encodingID == 1 {
			subtableAbs = cmapOff + int(subOff)
			break
		}
		if platformID == 0 && subtableAbs < 0 {
			subtableAbs = cmapOff + int(subOff)
		}
	}
	if subtableAbs < 0 {
		return 0, false
	}

	// Verify Format 4.
	format, err := readU16(buf, subtableAbs)
	if err != nil || format != 4 {
		return 0, false
	}

	// ── Parse Format 4 header ───────────────────────────────────────────────
	segCountX2, err := readU16(buf, subtableAbs+6)
	if err != nil {
		return 0, false
	}
	segCount := int(segCountX2 / 2)

	endCodesBase       := subtableAbs + 14
	startCodesBase     := subtableAbs + 16 + segCount*2
	idDeltaBase        := subtableAbs + 16 + segCount*4
	idRangeOffsetBase  := subtableAbs + 16 + segCount*6

	// ── Binary search on endCode[] ──────────────────────────────────────────
	lo, hi := 0, segCount
	for lo < hi {
		mid := (lo + hi) / 2
		endCode, err := readU16(buf, endCodesBase+mid*2)
		if err != nil {
			return 0, false
		}
		if endCode < cp {
			lo = mid + 1
		} else {
			hi = mid
		}
	}
	if lo >= segCount {
		return 0, false
	}

	endCode, err1   := readU16(buf, endCodesBase+lo*2)
	startCode, err2 := readU16(buf, startCodesBase+lo*2)
	if err1 != nil || err2 != nil {
		return 0, false
	}
	if cp < startCode || cp > endCode {
		return 0, false
	}

	idDelta, err1       := readI16(buf, idDeltaBase+lo*2)
	idRangeOffset, err2 := readU16(buf, idRangeOffsetBase+lo*2)
	if err1 != nil || err2 != nil {
		return 0, false
	}

	var glyph uint16
	if idRangeOffset == 0 {
		// Direct delta: (cp + idDelta) mod 65536.
		glyph = uint16(int(cp) + int(idDelta))
	} else {
		// Indirect: absolute byte offset of glyphIdArray[(cp - startCode)].
		//   (idRangeOffsetBase + lo*2) + idRangeOffset + (cp - startCode)*2
		absOff := (idRangeOffsetBase + lo*2) + int(idRangeOffset) + (int(cp)-int(startCode))*2
		g, err := readU16(buf, absOff)
		if err != nil {
			return 0, false
		}
		glyph = g
	}

	if glyph == 0 {
		return 0, false
	}
	return glyph, true
}

// ─────────────────────────────────────────────────────────────────────────────
// GetGlyphMetrics
// ─────────────────────────────────────────────────────────────────────────────

// GetGlyphMetrics returns horizontal metrics for a glyph ID.
//
// Returns (nil, false) if glyph_id is out of range.
func GetGlyphMetrics(font *FontFile, gid uint16) (*GlyphMetrics, bool) {
	buf := font.data
	t := font.tables

	numGlyphs, err1   := readU16(buf, int(t.maxp)+4)
	numHMetrics, err2 := readU16(buf, int(t.hhea)+34)
	if err1 != nil || err2 != nil {
		return nil, false
	}
	hmtxOff := int(t.hmtx)

	if gid >= numGlyphs {
		return nil, false
	}

	if gid < numHMetrics {
		base := hmtxOff + int(gid)*4
		aw, err1 := readU16(buf, base)
		lsb, err2 := readI16(buf, base+2)
		if err1 != nil || err2 != nil {
			return nil, false
		}
		return &GlyphMetrics{AdvanceWidth: aw, LeftSideBearing: lsb}, true
	}

	// Shared advance.
	lastAdv, err1 := readU16(buf, hmtxOff+(int(numHMetrics)-1)*4)
	lsbOff := hmtxOff + int(numHMetrics)*4 + (int(gid)-int(numHMetrics))*2
	lsb, err2 := readI16(buf, lsbOff)
	if err1 != nil || err2 != nil {
		return nil, false
	}
	return &GlyphMetrics{AdvanceWidth: lastAdv, LeftSideBearing: lsb}, true
}

// ─────────────────────────────────────────────────────────────────────────────
// Kerning — kern Format 0
// ─────────────────────────────────────────────────────────────────────────────

// Kerning returns the kern adjustment for a pair (design units).
//
// Returns 0 if the font has no kern table or the pair is not found.
// Negative = tighter spacing; positive = wider.
func Kerning(font *FontFile, left, right uint16) int16 {
	if font.tables.kern == nil {
		return 0
	}
	buf := font.data
	kernOff := int(*font.tables.kern)

	nTables, err := readU16(buf, kernOff+2)
	if err != nil {
		return 0
	}

	target := (uint32(left) << 16) | uint32(right)
	pos := kernOff + 4

	for i := 0; i < int(nTables); i++ {
		if pos+6 > len(buf) {
			break
		}
		length, err1   := readU16(buf, pos+2)
		coverage, err2 := readU16(buf, pos+4)
		if err1 != nil || err2 != nil {
			break
		}
		subFormat := coverage >> 8

		if subFormat == 0 {
			nPairs, err := readU16(buf, pos+6)
			if err != nil {
				break
			}
			pairsBase := pos + 14 // 6 (hdr) + 8 (format0 hdr)

			lo, hi := 0, int(nPairs)
			for lo < hi {
				mid := (lo + hi) / 2
				pairOff := pairsBase + mid*6
				pL, e1 := readU16(buf, pairOff)
				pR, e2 := readU16(buf, pairOff+2)
				if e1 != nil || e2 != nil {
					break
				}
				key := (uint32(pL) << 16) | uint32(pR)
				switch {
				case key == target:
					v, err := readI16(buf, pairOff+4)
					if err != nil {
						return 0
					}
					return v
				case key < target:
					lo = mid + 1
				default:
					hi = mid
				}
			}
		}

		pos += int(length)
	}

	return 0
}

// ─────────────────────────────────────────────────────────────────────────────
// name table reading
// ─────────────────────────────────────────────────────────────────────────────

func readNameString(buf []byte, nameOff *uint32, nameID uint16) string {
	if nameOff == nil {
		return ""
	}
	base := int(*nameOff)

	count, err1        := readU16(buf, base+2)
	stringOffset, err2 := readU16(buf, base+4)
	if err1 != nil || err2 != nil {
		return ""
	}

	type candidate struct {
		platformID uint16
		start      int
		length     int
	}
	var best *candidate

	for i := 0; i < int(count); i++ {
		rec := base + 6 + i*12
		if rec+12 > len(buf) {
			break
		}
		platformID, _ := readU16(buf, rec)
		encodingID, _  := readU16(buf, rec+2)
		nid, _         := readU16(buf, rec+6)
		length, _      := readU16(buf, rec+8)
		strOff, _      := readU16(buf, rec+10)

		if nid != nameID {
			continue
		}

		absStart := base + int(stringOffset) + int(strOff)
		if platformID == 3 && encodingID == 1 {
			best = &candidate{platformID: 3, start: absStart, length: int(length)}
			break
		}
		if platformID == 0 && best == nil {
			best = &candidate{platformID: 0, start: absStart, length: int(length)}
		}
	}

	if best == nil {
		return ""
	}

	end := best.start + best.length
	if end > len(buf) {
		return ""
	}
	raw := buf[best.start:end]

	// Decode UTF-16 BE: read pairs of bytes as big-endian uint16 code units.
	codeUnits := make([]uint16, len(raw)/2)
	for i := range codeUnits {
		codeUnits[i] = binary.BigEndian.Uint16(raw[i*2:])
	}
	runes := utf16.Decode(codeUnits)
	return string(runes)
}

// Ensure FontError implements the error interface.
var _ error = (*FontError)(nil)

// Ensure errors.As works with *FontError.
var _ interface{ Is(error) bool } = (*FontError)(nil)

func (e *FontError) Is(target error) bool {
	var t *FontError
	if errors.As(target, &t) {
		return e.Kind == t.Kind
	}
	return false
}
