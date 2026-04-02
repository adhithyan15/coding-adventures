/// FontParser — metrics-only OpenType/TrueType font parser.
///
/// OpenType and TrueType font files are binary table databases.  The first
/// 12 bytes (the "offset table") identify the format and count the tables.
/// Starting at byte 12, an array of 16-byte table records (tag + checksum +
/// offset + length) lets us locate any table by its 4-byte ASCII tag.
///
/// All multi-byte integers are **big-endian**.  We decode them manually with
/// shifts so we have no Foundation import requirement (just Swift stdlib +
/// Foundation for String.Encoding.utf16BigEndian).
///
/// Tables parsed:
///
/// | Tag  | Contents |
/// |------|----------|
/// | head | unitsPerEm |
/// | hhea | ascender, descender, lineGap, numberOfHMetrics |
/// | maxp | numGlyphs |
/// | cmap | Format 4, Unicode BMP → glyph index |
/// | hmtx | advance width + left-side bearing per glyph |
/// | kern | Format 0 sorted pairs (optional) |
/// | name | family / subfamily strings, UTF-16 BE (optional) |
/// | OS/2 | xHeight, capHeight (optional, version ≥ 2) |

import Foundation

// ─────────────────────────────────────────────────────────────────────────────
// Public error type
// ─────────────────────────────────────────────────────────────────────────────

/// Errors thrown by `FontParser.load(_:)`.
public enum FontError: Error, Equatable {
    /// The binary is too short to contain a valid font.
    case bufferTooShort
    /// The sfntVersion magic bytes are not recognised.
    case invalidMagic
    /// A required table (head, hhea, maxp, cmap, hmtx) is missing.
    case tableNotFound(String)
    /// A table's internal structure is invalid.
    case parseError(String)
}

// ─────────────────────────────────────────────────────────────────────────────
// Public metric types
// ─────────────────────────────────────────────────────────────────────────────

/// Global metrics extracted from a loaded font.
public struct FontMetrics: Equatable {
    /// Design units per em square (typically 1000 or 2048).
    public let unitsPerEm: UInt16
    /// Typographic ascender in font units (from `hhea`).
    public let ascender: Int16
    /// Typographic descender in font units (from `hhea`, usually negative).
    public let descender: Int16
    /// Line gap in font units.
    public let lineGap: Int16
    /// Height of lowercase 'x' in font units, or `nil` if OS/2 version < 2.
    public let xHeight: Int16?
    /// Height of capital letters in font units, or `nil` if OS/2 version < 2.
    public let capHeight: Int16?
    /// Total number of glyphs in the font.
    public let numGlyphs: UInt16
    /// Family name (e.g. "Inter") decoded from the `name` table.
    public let familyName: String
    /// Subfamily name (e.g. "Regular") decoded from the `name` table.
    public let subfamilyName: String
}

/// Per-glyph horizontal metrics from the `hmtx` table.
public struct GlyphMetrics: Equatable {
    /// Horizontal advance width in font units.
    public let advanceWidth: UInt16
    /// Left-side bearing in font units (signed).
    public let leftSideBearing: Int16
}

// ─────────────────────────────────────────────────────────────────────────────
// Internal representation
// ─────────────────────────────────────────────────────────────────────────────

/// Parsed representation of a font file. Treat as opaque.
public final class FontFile {
    // We keep the raw bytes so per-glyph reads can index into hmtx and the
    // cmap glyphIdArray without having to pre-materialise every glyph.
    let raw: Data
    public let metrics: FontMetrics
    let cmapSegments: [CmapSegment]
    let numHMetrics: Int
    let numGlyphs: Int
    let hmtxOffset: Int
    let kernMap: [Int: Int16]  // (left * 65536 + right) → value

    init(raw: Data, metrics: FontMetrics, cmapSegments: [CmapSegment],
         numHMetrics: Int, numGlyphs: Int, hmtxOffset: Int, kernMap: [Int: Int16])
    {
        self.raw           = raw
        self.metrics       = metrics
        self.cmapSegments  = cmapSegments
        self.numHMetrics   = numHMetrics
        self.numGlyphs     = numGlyphs
        self.hmtxOffset    = hmtxOffset
        self.kernMap       = kernMap
    }
}

/// A single cmap Format 4 segment.
struct CmapSegment {
    let endCode:       UInt16
    let startCode:     UInt16
    let idDelta:       Int16
    let idRangeOffset: UInt16
    /// Absolute byte offset of idRangeOffset[i] in the raw data.
    /// This is the base for the self-relative pointer calculation.
    let iroAbs:        Int
}

// ─────────────────────────────────────────────────────────────────────────────
// Public API
// ─────────────────────────────────────────────────────────────────────────────

/// Load a font from raw binary data.
///
/// - Parameter data: Binary font data (from `Data(contentsOf:)` etc.)
/// - Returns: An opaque `FontFile` on success.
/// - Throws: `FontError` on failure.
public func load(_ data: Data) throws -> FontFile {
    guard data.count >= 12 else { throw FontError.bufferTooShort }

    let sfntVer = ru32(data, 0)
    guard sfntVer == 0x00010000 || sfntVer == 0x4F54544F else {
        throw FontError.invalidMagic
    }

    let numTables = Int(ru16(data, 4))
    let tables    = try parseTableDirectory(data, numTables: numTables)

    let headD    = try parseHead(data, tables)
    let hheaD    = try parseHhea(data, tables)
    let ngGlyphs = try parseMaxp(data, tables)
    let cmapSegs = try parseCmap(data, tables)
    let hmtxOff  = try hmtxOffset(data, tables)

    let kernMap  = parseKern(data, tables)
    let (family, subfamily) = parseName(data, tables)
    let (xH, capH)          = parseOs2(data, tables)

    let m = FontMetrics(
        unitsPerEm:    headD.unitsPerEm,
        ascender:      hheaD.ascender,
        descender:     hheaD.descender,
        lineGap:       hheaD.lineGap,
        xHeight:       xH,
        capHeight:     capH,
        numGlyphs:     UInt16(ngGlyphs),
        familyName:    family,
        subfamilyName: subfamily
    )

    return FontFile(
        raw:          data,
        metrics:      m,
        cmapSegments: cmapSegs,
        numHMetrics:  hheaD.numHMetrics,
        numGlyphs:    ngGlyphs,
        hmtxOffset:   hmtxOff,
        kernMap:      kernMap
    )
}

/// Return the `FontMetrics` for a loaded font.
public func fontMetrics(_ font: FontFile) -> FontMetrics {
    font.metrics
}

/// Map a Unicode codepoint to a glyph index.
///
/// Returns `nil` for codepoints outside the BMP (> 0xFFFF), negative values,
/// or unmapped codepoints.
public func glyphId(_ font: FontFile, codepoint: Int) -> UInt16? {
    guard codepoint >= 0, codepoint <= 0xFFFF else { return nil }
    return cmapLookup(font.cmapSegments, data: font.raw, cp: UInt16(codepoint))
}

/// Return per-glyph metrics for the given glyph index, or `nil` if out of range.
public func glyphMetrics(_ font: FontFile, glyphId gid: Int) -> GlyphMetrics? {
    guard gid >= 0, gid < font.numGlyphs else { return nil }
    return lookupGlyphMetrics(font, gid: gid)
}

/// Return the kern value (in font units) for the ordered glyph pair.
///
/// Returns `0` when no `kern` table is present or the pair is not listed.
public func kerning(_ font: FontFile, left: Int, right: Int) -> Int16 {
    font.kernMap[left * 65536 + right] ?? 0
}

// ─────────────────────────────────────────────────────────────────────────────
// Big-endian read helpers
// ─────────────────────────────────────────────────────────────────────────────
//
// All offsets are 0-based. We use `data.startIndex + offset` to support
// Data slices whose startIndex may not be zero.

@inline(__always)
func ru8(_ data: Data, _ off: Int) -> UInt8 {
    data[data.startIndex + off]
}

@inline(__always)
func ru16(_ data: Data, _ off: Int) -> UInt16 {
    UInt16(data[data.startIndex + off]) << 8 |
    UInt16(data[data.startIndex + off + 1])
}

@inline(__always)
func ri16(_ data: Data, _ off: Int) -> Int16 {
    Int16(bitPattern: ru16(data, off))
}

@inline(__always)
func ru32(_ data: Data, _ off: Int) -> UInt32 {
    UInt32(data[data.startIndex + off])     << 24 |
    UInt32(data[data.startIndex + off + 1]) << 16 |
    UInt32(data[data.startIndex + off + 2]) <<  8 |
    UInt32(data[data.startIndex + off + 3])
}

// ─────────────────────────────────────────────────────────────────────────────
// Table directory
// ─────────────────────────────────────────────────────────────────────────────

struct TableRecord {
    let offset: Int
    let length: Int
}

func parseTableDirectory(_ data: Data, numTables: Int) throws -> [String: TableRecord] {
    var tables = [String: TableRecord]()
    for i in 0 ..< numTables {
        let base = 12 + i * 16
        guard base + 16 <= data.count else {
            throw FontError.parseError("table directory overflows buffer")
        }
        let tag = String(bytes: data[(data.startIndex + base) ..< (data.startIndex + base + 4)],
                         encoding: .ascii) ?? ""
        let off = Int(ru32(data, base + 8))
        let len = Int(ru32(data, base + 12))
        tables[tag] = TableRecord(offset: off, length: len)
    }
    return tables
}

func requireTable(_ tables: [String: TableRecord], tag: String) throws -> TableRecord {
    guard let t = tables[tag] else {
        throw FontError.tableNotFound(tag)
    }
    return t
}

// ─────────────────────────────────────────────────────────────────────────────
// head
// ─────────────────────────────────────────────────────────────────────────────

struct HeadData { let unitsPerEm: UInt16 }

func parseHead(_ data: Data, _ tables: [String: TableRecord]) throws -> HeadData {
    let t = try requireTable(tables, tag: "head")
    return HeadData(unitsPerEm: ru16(data, t.offset + 18))
}

// ─────────────────────────────────────────────────────────────────────────────
// hhea
// ─────────────────────────────────────────────────────────────────────────────

struct HheaData {
    let ascender: Int16
    let descender: Int16
    let lineGap: Int16
    let numHMetrics: Int
}

func parseHhea(_ data: Data, _ tables: [String: TableRecord]) throws -> HheaData {
    let t = try requireTable(tables, tag: "hhea")
    return HheaData(
        ascender:     ri16(data, t.offset + 4),
        descender:    ri16(data, t.offset + 6),
        lineGap:      ri16(data, t.offset + 8),
        numHMetrics:  Int(ru16(data, t.offset + 34))
    )
}

// ─────────────────────────────────────────────────────────────────────────────
// maxp — numGlyphs at offset 4
// ─────────────────────────────────────────────────────────────────────────────

func parseMaxp(_ data: Data, _ tables: [String: TableRecord]) throws -> Int {
    let t = try requireTable(tables, tag: "maxp")
    return Int(ru16(data, t.offset + 4))
}

// ─────────────────────────────────────────────────────────────────────────────
// cmap — Format 4 BMP subtable
// ─────────────────────────────────────────────────────────────────────────────
//
// Encoding record (8 bytes): platformID(2) + encodingID(2) + offset(4)
//
// Format 4 layout:
//   0    format         u16  = 4
//   6    segCountX2     u16
//   14   endCode[n]          2n bytes
//   16+2n startCode[n]       2n bytes
//   16+4n idDelta[n]         2n bytes  (signed)
//   16+6n idRangeOffset[n]   2n bytes

func parseCmap(_ data: Data, _ tables: [String: TableRecord]) throws -> [CmapSegment] {
    let cmapTable = try requireTable(tables, tag: "cmap")
    let cmapOff   = cmapTable.offset
    let numSubs   = Int(ru16(data, cmapOff + 2))

    var subOff: Int? = nil
    for i in 0 ..< numSubs {
        let rec  = cmapOff + 4 + i * 8
        let plat = ru16(data, rec)
        let enc  = ru16(data, rec + 2)
        let rel  = Int(ru32(data, rec + 4))
        if plat == 3 && enc == 1 {
            subOff = cmapOff + rel
            break
        }
    }

    guard let so = subOff else {
        throw FontError.tableNotFound("cmap Format 4 subtable")
    }
    guard ru16(data, so) == 4 else {
        throw FontError.parseError("expected cmap Format 4")
    }

    let segCount          = Int(ru16(data, so + 6)) >> 1
    let endCodesBase      = so + 14
    let startCodesBase    = so + 16 + segCount * 2
    let idDeltaBase       = so + 16 + segCount * 4
    let idRangeOffsetBase = so + 16 + segCount * 6

    return (0 ..< segCount).map { i in
        CmapSegment(
            endCode:       ru16(data, endCodesBase      + i * 2),
            startCode:     ru16(data, startCodesBase    + i * 2),
            idDelta:       ri16(data, idDeltaBase       + i * 2),
            idRangeOffset: ru16(data, idRangeOffsetBase + i * 2),
            iroAbs:        idRangeOffsetBase + i * 2
        )
    }
}

/// cmap_lookup: scan segments for codepoint `cp`.
///
/// The idRangeOffset self-relative pointer:
///   - If `idRangeOffset == 0`: glyphId = (cp + idDelta) & 0xFFFF
///   - Otherwise:               abs_off = iroAbs + idRangeOffset + (cp - startCode) * 2
///                               glyphId = ru16(data, abs_off)
func cmapLookup(_ segments: [CmapSegment], data: Data, cp: UInt16) -> UInt16? {
    for seg in segments {
        guard cp <= seg.endCode else { continue }
        guard cp >= seg.startCode else { return nil }

        let gid: UInt16
        if seg.idRangeOffset == 0 {
            gid = UInt16(bitPattern: Int16(bitPattern: cp) &+ seg.idDelta)
        } else {
            let absOff = seg.iroAbs + Int(seg.idRangeOffset) + Int(cp - seg.startCode) * 2
            gid = ru16(data, absOff)
        }
        return gid == 0 ? nil : gid
    }
    return nil
}

// ─────────────────────────────────────────────────────────────────────────────
// hmtx
// ─────────────────────────────────────────────────────────────────────────────

func hmtxOffset(_ data: Data, _ tables: [String: TableRecord]) throws -> Int {
    let t = try requireTable(tables, tag: "hmtx")
    _ = ru8(data, t.offset)  // sanity probe
    return t.offset
}

func lookupGlyphMetrics(_ font: FontFile, gid: Int) -> GlyphMetrics? {
    let nhm  = font.numHMetrics
    let off  = font.hmtxOffset
    let data = font.raw

    let metricIdx = min(gid, nhm - 1)
    let advance   = ru16(data, off + metricIdx * 4)

    let lsb: Int16
    if gid < nhm {
        lsb = ri16(data, off + gid * 4 + 2)
    } else {
        lsb = ri16(data, off + nhm * 4 + (gid - nhm) * 2)
    }

    return GlyphMetrics(advanceWidth: advance, leftSideBearing: lsb)
}

// ─────────────────────────────────────────────────────────────────────────────
// kern — Format 0
// ─────────────────────────────────────────────────────────────────────────────
//
// coverage HIGH byte = format (0 = sorted pairs)

func parseKern(_ data: Data, _ tables: [String: TableRecord]) -> [Int: Int16] {
    guard let t = tables["kern"] else { return [:] }

    let off      = t.offset
    let nTables  = Int(ru16(data, off + 2))
    var kernMap  = [Int: Int16]()
    var cur      = off + 4

    for _ in 0 ..< nTables {
        let subLen   = Int(ru16(data, cur + 2))
        let coverage = ru16(data, cur + 4)
        let fmt      = coverage >> 8  // format in HIGH byte

        if fmt == 0 {
            let nPairs    = Int(ru16(data, cur + 6))
            let pairsBase = cur + 14

            for j in 0 ..< nPairs {
                let poff  = pairsBase + j * 6
                let left  = Int(ru16(data, poff))
                let right = Int(ru16(data, poff + 2))
                let value = ri16(data, poff + 4)
                kernMap[left * 65536 + right] = value
            }
        }

        cur += subLen
    }

    return kernMap
}

// ─────────────────────────────────────────────────────────────────────────────
// name table
// ─────────────────────────────────────────────────────────────────────────────

func parseName(_ data: Data, _ tables: [String: TableRecord]) -> (String, String) {
    guard let t = tables["name"] else { return ("(unknown)", "(unknown)") }

    let tblOff  = t.offset
    let count   = Int(ru16(data, tblOff + 2))
    let strBase = tblOff + Int(ru16(data, tblOff + 4))

    var family: String? = nil
    var subfam: String? = nil

    for i in 0 ..< count {
        let rec  = tblOff + 6 + i * 12
        let plat = ru16(data, rec)
        let enc  = ru16(data, rec + 2)
        let nid  = ru16(data, rec + 6)
        let nlen = Int(ru16(data, rec + 8))
        let noff = Int(ru16(data, rec + 10))

        if plat == 3 && enc == 1 {
            let start = data.startIndex + strBase + noff
            let slice = data[start ..< (start + nlen)]
            let str   = String(data: slice, encoding: .utf16BigEndian)
            if nid == 1 && family   == nil { family = str }
            if nid == 2 && subfam   == nil { subfam = str }
        }

        if family != nil && subfam != nil { break }
    }

    return (family ?? "(unknown)", subfam ?? "(unknown)")
}

// ─────────────────────────────────────────────────────────────────────────────
// OS/2 table
// ─────────────────────────────────────────────────────────────────────────────
//
// sxHeight  at offset 86  (version ≥ 2)
// sCapHeight at offset 88 (version ≥ 2)

func parseOs2(_ data: Data, _ tables: [String: TableRecord]) -> (Int16?, Int16?) {
    guard let t = tables["OS/2"] else { return (nil, nil) }

    let off     = t.offset
    let version = ru16(data, off)

    guard version >= 2, t.length >= 90 else { return (nil, nil) }

    return (ri16(data, off + 86), ri16(data, off + 88))
}
