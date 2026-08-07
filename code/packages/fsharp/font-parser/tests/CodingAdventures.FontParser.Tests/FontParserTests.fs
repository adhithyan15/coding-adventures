namespace CodingAdventures.FontParser.Tests

open System
open System.IO
open System.Text
open CodingAdventures.FontParser
open Xunit

module private Fixture =
    let writeU16 (bytes: byte array) offset (value: int) =
        bytes.[offset] <- byte (value >>> 8)
        bytes.[offset + 1] <- byte value

    let writeI16 bytes offset (value: int16) =
        writeU16 bytes offset (int value &&& 0xFFFF)

    let writeU32 (bytes: byte array) offset (value: uint32) =
        bytes.[offset] <- byte (value >>> 24)
        bytes.[offset + 1] <- byte (value >>> 16)
        bytes.[offset + 2] <- byte (value >>> 8)
        bytes.[offset + 3] <- byte value

    let readU16 (bytes: byte array) offset =
        (int bytes.[offset] <<< 8) ||| int bytes.[offset + 1]

    let readU32 (bytes: byte array) offset =
        (uint32 bytes.[offset] <<< 24)
        ||| (uint32 bytes.[offset + 1] <<< 16)
        ||| (uint32 bytes.[offset + 2] <<< 8)
        ||| uint32 bytes.[offset + 3]

    let writeTable bytes index (tag: string) offset length =
        let record = 12 + index * 16
        Encoding.ASCII.GetBytes(tag).CopyTo(bytes, record)
        writeU32 bytes (record + 8) (uint32 offset)
        writeU32 bytes (record + 12) (uint32 length)

    let findTable bytes (tag: string) =
        seq { 0 .. readU16 bytes 4 - 1 }
        |> Seq.tryPick (fun index ->
            let record = 12 + index * 16
            if Encoding.ASCII.GetString(bytes, record, 4) = tag then
                Some(int (readU32 bytes (record + 8)))
            else
                None)
        |> Option.defaultWith (fun () -> invalidOp $"Missing table {tag}")

    let buildSyntheticFont (inputPairs: (uint16 * uint16 * int16) list) =
        let pairs = inputPairs |> List.sortBy (fun (left, right, _) -> (uint32 left <<< 16) ||| uint32 right)
        let tableCount = 6
        let headLength = 54
        let hheaLength = 36
        let maxpLength = 6
        let cmapLength = 36
        let hmtxLength = 14
        let kernLength = 18 + pairs.Length * 6
        let directoryLength = 12 + tableCount * 16
        let head = directoryLength
        let hhea = head + headLength
        let maxp = hhea + hheaLength
        let cmap = maxp + maxpLength
        let hmtx = cmap + cmapLength
        let kern = hmtx + hmtxLength
        let bytes = Array.zeroCreate<byte> (kern + kernLength)

        writeU32 bytes 0 0x00010000u
        writeU16 bytes 4 tableCount
        writeTable bytes 0 "cmap" cmap cmapLength
        writeTable bytes 1 "head" head headLength
        writeTable bytes 2 "hhea" hhea hheaLength
        writeTable bytes 3 "hmtx" hmtx hmtxLength
        writeTable bytes 4 "kern" kern kernLength
        writeTable bytes 5 "maxp" maxp maxpLength

        writeU32 bytes head 0x00010000u
        writeU32 bytes (head + 12) 0x5F0F3CF5u
        writeU16 bytes (head + 18) 1000

        writeU32 bytes hhea 0x00010000u
        writeI16 bytes (hhea + 4) 800s
        writeI16 bytes (hhea + 6) -200s
        writeI16 bytes (hhea + 8) 10s
        writeU16 bytes (hhea + 34) 2

        writeU32 bytes maxp 0x00005000u
        writeU16 bytes (maxp + 4) 5

        writeU16 bytes (cmap + 2) 1
        writeU16 bytes (cmap + 4) 3
        writeU16 bytes (cmap + 6) 1
        writeU32 bytes (cmap + 8) 12u
        writeU16 bytes (cmap + 12) 4
        writeU16 bytes (cmap + 14) 24
        writeU16 bytes (cmap + 18) 2
        writeU16 bytes (cmap + 20) 2
        writeU16 bytes (cmap + 26) 0xFFFF
        writeU16 bytes (cmap + 28) 0
        writeU16 bytes (cmap + 30) 0xFFFF
        writeI16 bytes (cmap + 32) 1s
        writeU16 bytes (cmap + 34) 0

        writeU16 bytes hmtx 600
        writeI16 bytes (hmtx + 2) 10s
        writeU16 bytes (hmtx + 4) 700
        writeI16 bytes (hmtx + 6) 20s
        writeI16 bytes (hmtx + 8) 30s
        writeI16 bytes (hmtx + 10) 40s
        writeI16 bytes (hmtx + 12) 50s

        writeU16 bytes kern 0
        writeU16 bytes (kern + 2) 1
        writeU16 bytes (kern + 4) 0
        writeU16 bytes (kern + 6) (kernLength - 4)
        writeU16 bytes (kern + 8) 1
        writeU16 bytes (kern + 10) pairs.Length

        pairs
        |> List.iteri (fun index (left, right, value) ->
            let offset = kern + 18 + index * 6
            writeU16 bytes offset (int left)
            writeU16 bytes (offset + 2) (int right)
            writeI16 bytes (offset + 4) value)

        bytes

    let interBytes () =
        File.ReadAllBytes(Path.Combine(AppContext.BaseDirectory, "Fixtures", "Inter-Regular.ttf"))

module FontParserTests =
    [<Fact>]
    let ``load rejects null and short buffers`` () =
        Assert.Throws<ArgumentNullException>(fun () -> FontParser.load null |> ignore) |> ignore
        let error = Assert.Throws<FontParseException>(fun () -> FontParser.load [||] |> ignore)
        Assert.Equal(FontErrorKind.BufferTooShort, error.Kind)

    [<Fact>]
    let ``load rejects invalid magic and missing tables`` () =
        let invalid = Array.zeroCreate<byte> 12
        invalid.[0] <- 0xDEuy
        let magicError = Assert.Throws<FontParseException>(fun () -> FontParser.load invalid |> ignore)
        Assert.Equal(FontErrorKind.InvalidMagic, magicError.Kind)

        let emptyDirectory = Array.zeroCreate<byte> 12
        Fixture.writeU32 emptyDirectory 0 0x00010000u
        let tableError = Assert.Throws<FontParseException>(fun () -> FontParser.load emptyDirectory |> ignore)
        Assert.Equal(FontErrorKind.TableNotFound, tableError.Kind)

    [<Fact>]
    let ``load rejects truncated directories and bad head magic`` () =
        let truncated = Array.zeroCreate<byte> 12
        Fixture.writeU32 truncated 0 0x00010000u
        Fixture.writeU16 truncated 4 1
        let shortError = Assert.Throws<FontParseException>(fun () -> FontParser.load truncated |> ignore)
        Assert.Equal(FontErrorKind.BufferTooShort, shortError.Kind)

        let badHead = Fixture.buildSyntheticFont []
        let head = Fixture.findTable badHead "head"
        Fixture.writeU32 badHead (head + 12) 0u
        let headError = Assert.Throws<FontParseException>(fun () -> FontParser.load badHead |> ignore)
        Assert.Equal(FontErrorKind.InvalidHeadMagic, headError.Kind)

    [<Fact>]
    let ``load owns an immutable copy`` () =
        let bytes = Fixture.buildSyntheticFont []
        let font = FontParser.load bytes
        bytes.[0] <- 0xFFuy
        Assert.Equal(1000us, (FontParser.fontMetrics font).UnitsPerEm)

    [<Fact>]
    let ``Inter exposes global metrics and names`` () =
        let metrics = Fixture.interBytes () |> FontParser.load |> FontParser.fontMetrics
        Assert.Equal(2048us, metrics.UnitsPerEm)
        Assert.Equal("Inter", metrics.FamilyName)
        Assert.Equal("Regular", metrics.SubfamilyName)
        Assert.True(metrics.Ascender > 0s)
        Assert.True(metrics.Descender <= 0s)
        Assert.True(metrics.NumGlyphs > 100us)
        Assert.True(metrics.XHeight |> Option.exists (fun value -> value > 0s))
        Assert.True(metrics.CapHeight |> Option.exists (fun value -> value > 0s))

    [<Fact>]
    let ``synthetic font uses hhea fallback and unknown names`` () =
        let metrics = Fixture.buildSyntheticFont [] |> FontParser.load |> FontParser.fontMetrics
        Assert.Equal(1000us, metrics.UnitsPerEm)
        Assert.Equal(800s, metrics.Ascender)
        Assert.Equal(-200s, metrics.Descender)
        Assert.Equal(10s, metrics.LineGap)
        Assert.Equal(None, metrics.XHeight)
        Assert.Equal(None, metrics.CapHeight)
        Assert.Equal("(unknown)", metrics.FamilyName)
        Assert.Equal("(unknown)", metrics.SubfamilyName)

    [<Fact>]
    let ``glyph lookup handles mapped unmapped and out of BMP values`` () =
        let font = Fixture.interBytes () |> FontParser.load
        let a = FontParser.glyphId font (int 'A')
        let v = FontParser.glyphId font (int 'V')
        Assert.True(a.IsSome)
        Assert.True(v.IsSome)
        Assert.NotEqual(a, v)
        Assert.True((FontParser.glyphId font (int ' ')).IsSome)
        Assert.Equal(None, FontParser.glyphId font -1)
        Assert.Equal(None, FontParser.glyphId font 0x10000)
        Assert.Equal(None, Fixture.buildSyntheticFont [] |> FontParser.load |> fun value -> FontParser.glyphId value 0xFFFF)

    [<Fact>]
    let ``glyph lookup ignores unsupported cmap formats`` () =
        let bytes = Fixture.buildSyntheticFont []
        let cmap = Fixture.findTable bytes "cmap"
        Fixture.writeU16 bytes (cmap + 12) 12
        Assert.Equal(None, bytes |> FontParser.load |> fun font -> FontParser.glyphId font (int 'A'))

    [<Fact>]
    let ``glyph metrics support full and shared advance records`` () =
        let font = Fixture.buildSyntheticFont [] |> FontParser.load
        Assert.Equal(Some { AdvanceWidth = 600us; LeftSideBearing = 10s }, FontParser.glyphMetrics font 0)
        Assert.Equal(Some { AdvanceWidth = 700us; LeftSideBearing = 20s }, FontParser.glyphMetrics font 1)
        Assert.Equal(Some { AdvanceWidth = 700us; LeftSideBearing = 40s }, FontParser.glyphMetrics font 3)
        Assert.Equal(None, FontParser.glyphMetrics font -1)
        Assert.Equal(None, FontParser.glyphMetrics font 5)

        let inter = Fixture.interBytes () |> FontParser.load
        let glyph = FontParser.glyphId inter (int 'A') |> Option.get |> int
        Assert.True((FontParser.glyphMetrics inter glyph |> Option.get).AdvanceWidth > 0us)

    [<Fact>]
    let ``kerning reads sorted format zero pairs`` () =
        let font = Fixture.buildSyntheticFont [ 1us, 2us, -140s; 3us, 4us, 80s ] |> FontParser.load
        Assert.Equal(-140s, FontParser.kerning font 1 2)
        Assert.Equal(80s, FontParser.kerning font 3 4)
        Assert.Equal(0s, FontParser.kerning font 1 4)
        Assert.Equal(0s, FontParser.kerning font 2 1)
        Assert.Equal(0s, FontParser.kerning font -1 2)

    [<Fact>]
    let ``kerning defaults to zero when table is absent`` () =
        let font = Fixture.interBytes () |> FontParser.load
        let a = FontParser.glyphId font (int 'A') |> Option.get |> int
        let v = FontParser.glyphId font (int 'V') |> Option.get |> int
        Assert.Equal(0s, FontParser.kerning font a v)
