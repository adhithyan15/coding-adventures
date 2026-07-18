namespace CodingAdventures.FontParser

open System
open System.Collections.Generic
open System.Text

/// Identifies why a font could not be parsed.
type FontErrorKind =
    | BufferTooShort
    | InvalidMagic
    | InvalidHeadMagic
    | TableNotFound

/// Raised when an OpenType or TrueType binary is malformed.
type FontParseException(kind: FontErrorKind, message: string) =
    inherit Exception(message)
    member _.Kind = kind

/// Global typographic metrics in font design units.
type FontMetrics =
    { UnitsPerEm: uint16
      Ascender: int16
      Descender: int16
      LineGap: int16
      XHeight: int16 option
      CapHeight: int16 option
      NumGlyphs: uint16
      FamilyName: string
      SubfamilyName: string }

/// Horizontal metrics for a single glyph.
type GlyphMetrics =
    { AdvanceWidth: uint16
      LeftSideBearing: int16 }

type internal TableRecord =
    { Offset: int
      Length: int }

/// An immutable handle to a parsed font binary.
type FontFile internal (data: byte array, tables: IReadOnlyDictionary<string, TableRecord>) =
    member internal _.Data = data
    member internal _.Tables = tables

/// Metrics-only OpenType and TrueType font parser.
[<RequireQualifiedAccess>]
module FontParser =
    let private trueTypeMagic = 0x00010000u
    let private openTypeMagic = 0x4F54544Fu
    let private headMagic = 0x5F0F3CF5u

    let private fail kind message = raise (FontParseException(kind, message))

    let private ensureRange (data: byte array) offset length =
        if offset < 0 || length < 0 || offset > data.Length - length then
            fail BufferTooShort $"read at offset {offset} with length {length} exceeds the font buffer"

    let private readU16 (data: byte array) offset =
        ensureRange data offset 2
        (uint16 data.[offset] <<< 8) ||| uint16 data.[offset + 1]

    let private readI16 data offset =
        let value = readU16 data offset |> int
        if value <= 32767 then int16 value else int16 (value - 65536)

    let private readU32 (data: byte array) offset =
        ensureRange data offset 4
        (uint32 data.[offset] <<< 24)
        ||| (uint32 data.[offset + 1] <<< 16)
        ||| (uint32 data.[offset + 2] <<< 8)
        ||| uint32 data.[offset + 3]

    let private checkedInt label (value: uint32) =
        if value > uint32 Int32.MaxValue then
            fail BufferTooShort $"{label} exceeds the supported buffer size"
        int value

    let private requireTable (tables: IReadOnlyDictionary<string, TableRecord>) tag =
        match tables.TryGetValue(tag) with
        | true, table -> table
        | false, _ -> fail TableNotFound $"required table '{tag}' was not found"

    /// Parse a complete font binary and retain an immutable copy.
    let load (bytes: byte array) =
        nullArgCheck "bytes" bytes |> ignore
        if bytes.Length < 12 then
            fail BufferTooShort "font buffer is too small to contain an sfnt header"

        let data = Array.copy bytes
        let magic = readU32 data 0
        if magic <> trueTypeMagic && magic <> openTypeMagic then
            fail InvalidMagic $"invalid sfnt version 0x{magic:X8}"

        let numTables = readU16 data 4 |> int
        let tables = Dictionary<string, TableRecord>(StringComparer.Ordinal)

        for index in 0 .. numTables - 1 do
            let recordOffset = 12 + index * 16
            ensureRange data recordOffset 16
            let tag = Encoding.ASCII.GetString(data, recordOffset, 4)
            let offset = readU32 data (recordOffset + 8) |> checkedInt "table offset"
            let length = readU32 data (recordOffset + 12) |> checkedInt "table length"
            ensureRange data offset length
            tables.[tag] <- { Offset = offset; Length = length }

        let readOnlyTables = tables :> IReadOnlyDictionary<string, TableRecord>
        for tag in [ "head"; "hhea"; "maxp"; "cmap"; "hmtx" ] do
            requireTable readOnlyTables tag |> ignore

        let head = requireTable readOnlyTables "head"
        if readU32 data (head.Offset + 12) <> headMagic then
            fail InvalidHeadMagic "invalid head.magicNumber"

        FontFile(data, readOnlyTables)

    let private readName (font: FontFile) nameId =
        match font.Tables.TryGetValue("name") with
        | false, _ -> None
        | true, name ->
            let data = font.Data
            let count = readU16 data (name.Offset + 2) |> int
            let stringBase = name.Offset + (readU16 data (name.Offset + 4) |> int)
            let mutable fallback: (int * int) option = None
            let mutable result: string option = None
            let mutable index = 0

            while index < count && result.IsNone do
                let record = name.Offset + 6 + index * 12
                let platform = readU16 data record
                let encoding = readU16 data (record + 2)
                let recordNameId = readU16 data (record + 6)

                if recordNameId = nameId then
                    let length = readU16 data (record + 8) |> int
                    let start = stringBase + (readU16 data (record + 10) |> int)
                    ensureRange data start length

                    if platform = 3us && (encoding = 1us || encoding = 10us) then
                        result <- Some(Encoding.BigEndianUnicode.GetString(data, start, length))
                    elif platform = 0us && fallback.IsNone then
                        fallback <- Some(start, length)

                index <- index + 1

            match result, fallback with
            | Some value, _ -> Some value
            | None, Some(start, length) -> Some(Encoding.BigEndianUnicode.GetString(data, start, length))
            | None, None -> None

    /// Return global font metrics, preferring OS/2 typographic values.
    let fontMetrics (font: FontFile) =
        nullArgCheck "font" font |> ignore
        let data = font.Data
        let head = requireTable font.Tables "head"
        let hhea = requireTable font.Tables "hhea"
        let maxp = requireTable font.Tables "maxp"
        let mutable ascender = readI16 data (hhea.Offset + 4)
        let mutable descender = readI16 data (hhea.Offset + 6)
        let mutable lineGap = readI16 data (hhea.Offset + 8)
        let mutable xHeight = None
        let mutable capHeight = None

        match font.Tables.TryGetValue("OS/2") with
        | true, os2 ->
            let version = readU16 data os2.Offset
            if os2.Length >= 74 then
                ascender <- readI16 data (os2.Offset + 68)
                descender <- readI16 data (os2.Offset + 70)
                lineGap <- readI16 data (os2.Offset + 72)
            if version >= 2us && os2.Length >= 90 then
                xHeight <- Some(readI16 data (os2.Offset + 86))
                capHeight <- Some(readI16 data (os2.Offset + 88))
        | false, _ -> ()

        { UnitsPerEm = readU16 data (head.Offset + 18)
          Ascender = ascender
          Descender = descender
          LineGap = lineGap
          XHeight = xHeight
          CapHeight = capHeight
          NumGlyphs = readU16 data (maxp.Offset + 4)
          FamilyName = readName font 1us |> Option.defaultValue "(unknown)"
          SubfamilyName = readName font 2us |> Option.defaultValue "(unknown)" }

    /// Map a BMP Unicode codepoint to a glyph identifier.
    let glyphId (font: FontFile) codepoint =
        nullArgCheck "font" font |> ignore
        if codepoint < 0 || codepoint > 0xFFFF then
            None
        else
            let data = font.Data
            let cmap = requireTable font.Tables "cmap"
            let numSubtables = readU16 data (cmap.Offset + 2) |> int
            let mutable selected: int option = None
            let mutable index = 0
            let mutable foundBest = false

            while index < numSubtables && not foundBest do
                let record = cmap.Offset + 4 + index * 8
                let platform = readU16 data record
                let encoding = readU16 data (record + 2)
                let relative = readU32 data (record + 4) |> checkedInt "cmap subtable offset"
                let absolute = cmap.Offset + relative

                if platform = 3us && encoding = 1us then
                    selected <- Some absolute
                    foundBest <- true
                elif platform = 0us && selected.IsNone then
                    selected <- Some absolute

                index <- index + 1

            match selected with
            | None -> None
            | Some subtable when readU16 data subtable <> 4us -> None
            | Some subtable ->
                let segmentCount = readU16 data (subtable + 6) |> int |> fun value -> value / 2
                let endCodes = subtable + 14
                let startCodes = subtable + 16 + segmentCount * 2
                let deltas = subtable + 16 + segmentCount * 4
                let rangeOffsets = subtable + 16 + segmentCount * 6
                let mutable lo = 0
                let mutable hi = segmentCount

                while lo < hi do
                    let mid = lo + (hi - lo) / 2
                    if int (readU16 data (endCodes + mid * 2)) < codepoint then
                        lo <- mid + 1
                    else
                        hi <- mid

                if lo >= segmentCount then
                    None
                else
                    let startCode = readU16 data (startCodes + lo * 2) |> int
                    let endCode = readU16 data (endCodes + lo * 2) |> int
                    if codepoint < startCode || codepoint > endCode then
                        None
                    else
                        let delta = readI16 data (deltas + lo * 2) |> int
                        let rangeAddress = rangeOffsets + lo * 2
                        let rangeOffset = readU16 data rangeAddress |> int
                        let glyph =
                            if rangeOffset = 0 then
                                (codepoint + delta) &&& 0xFFFF
                            else
                                let address = rangeAddress + rangeOffset + (codepoint - startCode) * 2
                                let raw = readU16 data address |> int
                                if raw = 0 then 0 else (raw + delta) &&& 0xFFFF
                        if glyph = 0 then None else Some(uint16 glyph)

    /// Return horizontal metrics for a glyph, or None when out of range.
    let glyphMetrics (font: FontFile) glyphIdentifier =
        nullArgCheck "font" font |> ignore
        let data = font.Data
        let maxp = requireTable font.Tables "maxp"
        let hhea = requireTable font.Tables "hhea"
        let hmtx = requireTable font.Tables "hmtx"
        let numGlyphs = readU16 data (maxp.Offset + 4) |> int
        let numHorizontalMetrics = readU16 data (hhea.Offset + 34) |> int

        if glyphIdentifier < 0 || glyphIdentifier >= numGlyphs || numHorizontalMetrics = 0 then
            None
        elif glyphIdentifier < numHorizontalMetrics then
            let offset = hmtx.Offset + glyphIdentifier * 4
            Some
                { AdvanceWidth = readU16 data offset
                  LeftSideBearing = readI16 data (offset + 2) }
        else
            let lastMetric = hmtx.Offset + (numHorizontalMetrics - 1) * 4
            let bearing = hmtx.Offset + numHorizontalMetrics * 4 + (glyphIdentifier - numHorizontalMetrics) * 2
            Some
                { AdvanceWidth = readU16 data lastMetric
                  LeftSideBearing = readI16 data bearing }

    /// Return a legacy kern format 0 adjustment, or zero when absent.
    let kerning (font: FontFile) leftGlyphId rightGlyphId =
        nullArgCheck "font" font |> ignore
        if leftGlyphId < 0 || leftGlyphId > 0xFFFF || rightGlyphId < 0 || rightGlyphId > 0xFFFF then
            0s
        else
            match font.Tables.TryGetValue("kern") with
            | false, _ -> 0s
            | true, kern ->
                let data = font.Data
                let tableEnd = kern.Offset + kern.Length
                let subtableCount = readU16 data (kern.Offset + 2) |> int
                let target = (uint32 leftGlyphId <<< 16) ||| uint32 rightGlyphId
                let mutable position = kern.Offset + 4
                let mutable table = 0
                let mutable result: int16 option = None

                while table < subtableCount && position + 6 <= tableEnd && result.IsNone do
                    let length = readU16 data (position + 2) |> int
                    if length < 6 || position + length > tableEnd then
                        table <- subtableCount
                    else
                        let format = readU16 data (position + 4) >>> 8
                        if format = 0us && length >= 14 then
                            let pairCount = readU16 data (position + 6) |> int
                            let pairs = position + 14
                            let rec search lo hi =
                                if lo >= hi then
                                    None
                                else
                                    let mid = lo + (hi - lo) / 2
                                    let pair = pairs + mid * 6
                                    if pair + 6 > position + length then
                                        None
                                    else
                                        let key = (uint32 (readU16 data pair) <<< 16) ||| uint32 (readU16 data (pair + 2))
                                        if key = target then Some(readI16 data (pair + 4))
                                        elif key < target then search (mid + 1) hi
                                        else search lo mid
                            result <- search 0 pairCount

                        position <- position + length
                        table <- table + 1

                result |> Option.defaultValue 0s
