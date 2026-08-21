namespace CodingAdventures.ImageCodecPng.FSharp.Tests

open System
open System.Buffers.Binary
open System.IO
open System.IO.Compression
open System.Text
open System.Text.Json
open CodingAdventures.ImageCodecPng.FSharp
open CodingAdventures.PixelContainer
open Xunit

module PortableConformanceTests =
    type private Chunk = { Type: string; Data: byte[] }

    let private fromHex (value: string) = Convert.FromHexString(value)

    let private findFixture () =
        let rec search (directory: DirectoryInfo) =
            if isNull directory then
                raise (FileNotFoundException("image-codec-png-v1 fixture not found"))
            let candidate = Path.Combine(directory.FullName, "code", "specs", "fixtures", "image-codec-png-v1", "cases.json")
            if File.Exists(candidate) then candidate else search directory.Parent
        search (DirectoryInfo(AppContext.BaseDirectory))

    let private fixturePixels (input: JsonElement) =
        let width = input.GetProperty("width").GetDouble()
        let height = input.GetProperty("height").GetDouble()
        if not (Double.IsFinite(width)) || not (Double.IsFinite(height)) ||
           Math.Truncate(width) <> width || Math.Truncate(height) <> height ||
           width < 0.0 || height < 0.0 || width > float Int32.MaxValue || height > float Int32.MaxValue then
            raise (PngError("invalid-image-dimensions"))
        let data = fromHex (input.GetProperty("rgba_hex").GetString())
        let expected = int64 width * int64 height * 4L
        if int64 data.LongLength <> expected then raise (PngError("invalid-pixel-data-length"))
        PixelContainer(int width, int height, data)

    let private decodeFixture (testCase: JsonElement) =
        let bytes = fromHex (testCase.GetProperty("png_hex").GetString())
        let mutable options = Unchecked.defaultof<JsonElement>
        if testCase.TryGetProperty("options", &options) then
            Png.decodePng bytes (Some(options.GetProperty("max_pixels").GetDouble()))
        else Png.decodePng bytes None

    let private parseChunks (encoded: byte[]) =
        let chunks = ResizeArray<Chunk>()
        let mutable offset = 8
        while offset < encoded.Length do
            let length = int (BinaryPrimitives.ReadUInt32BigEndian(ReadOnlySpan<byte>(encoded, offset, 4)))
            chunks.Add({ Type = Encoding.ASCII.GetString(encoded, offset + 4, 4)
                         Data = encoded[(offset + 8) .. (offset + 8 + length - 1)] })
            offset <- offset + length + 12
        chunks.ToArray()

    let private assertDecode (testCase: JsonElement) id =
        let decoded = decodeFixture testCase
        let expected = testCase.GetProperty("expected")
        Assert.Equal(expected.GetProperty("width").GetInt32(), decoded.Width)
        Assert.Equal(expected.GetProperty("height").GetInt32(), decoded.Height)
        Assert.True(decoded.Data = fromHex (expected.GetProperty("rgba_hex").GetString()), id)

    let private assertDecodeError (testCase: JsonElement) id =
        let error = Assert.Throws<PngError>(fun () -> decodeFixture testCase |> ignore)
        Assert.Equal(testCase.GetProperty("expected").GetProperty("error_id").GetString(), error.Code)
        Assert.Equal(error.Code, error.Message)
        Assert.True(error.Message.Length <= 40, id)

    let private assertEncode (testCase: JsonElement) id =
        let input = testCase.GetProperty("input")
        let pixels = fixturePixels input
        let encoded = Png.encodePng pixels
        let expected = testCase.GetProperty("expected")
        let chunks = parseChunks encoded
        let expectedTypes = expected.GetProperty("chunk_types").EnumerateArray() |> Seq.map (fun value -> value.GetString()) |> Seq.toArray
        Assert.Equal<string>(expectedTypes, chunks |> Array.map (fun chunk -> chunk.Type))
        Assert.Equal(expected.GetProperty("bit_depth").GetByte(), encoded[24])
        Assert.Equal(expected.GetProperty("colour_type").GetByte(), encoded[25])
        Assert.Equal(expected.GetProperty("interlace").GetByte(), encoded[28])

        let idat = chunks |> Array.filter (fun chunk -> chunk.Type = "IDAT") |> Array.collect (fun chunk -> chunk.Data)
        use source = new MemoryStream(idat)
        use zlib = new ZLibStream(source, CompressionMode.Decompress)
        use filtered = new MemoryStream()
        zlib.CopyTo(filtered)
        let filteredBytes = filtered.ToArray()
        let stride = pixels.Width * 4
        let actualFilters = Array.init pixels.Height (fun row -> filteredBytes[row * (stride + 1)])
        let expectedFilters = expected.GetProperty("filter_types").EnumerateArray() |> Seq.map (fun value -> value.GetByte()) |> Seq.toArray
        Assert.Equal<byte>(expectedFilters, actualFilters)
        let decoded = Png.decodePng encoded None
        Assert.Equal<byte>(pixels.Data, decoded.Data)
        Assert.Equal(source.Length, source.Position)
        Assert.True(filteredBytes.Length = pixels.Height * (stride + 1), id)

    let private assertEncodeError (testCase: JsonElement) id =
        let error = Assert.Throws<PngError>(fun () -> fixturePixels (testCase.GetProperty("input")) |> Png.encodePng |> ignore)
        Assert.Equal(testCase.GetProperty("expected").GetProperty("error_id").GetString(), error.Code)
        Assert.True(error.Message.Length <= 40, id)

    [<Fact>]
    let ``closed portable corpus passes through the public API`` () =
        use document = JsonDocument.Parse(File.ReadAllText(findFixture()))
        let root = document.RootElement
        Assert.Equal(1, root.GetProperty("schema_version").GetInt32())
        Assert.Equal("image-codec-png-v1", root.GetProperty("profile").GetString())
        Assert.Equal(Png.maxDimension, root.GetProperty("limits").GetProperty("max_dimension").GetInt32())
        Assert.Equal(Png.defaultMaxPixels, root.GetProperty("limits").GetProperty("default_max_pixels").GetInt32())
        let fixtureErrors = root.GetProperty("error_ids").EnumerateArray() |> Seq.map (fun value -> value.GetString()) |> Seq.toArray
        Assert.Equal<string>(Png.errorCodes(), fixtureErrors)

        let cases = root.GetProperty("cases").EnumerateArray() |> Seq.toArray
        Assert.Equal(85, cases.Length)
        for testCase in cases do
            let id = testCase.GetProperty("id").GetString()
            match testCase.GetProperty("operation").GetString() with
            | "decode" -> assertDecode testCase id
            | "decode-error" -> assertDecodeError testCase id
            | "encode" -> assertEncode testCase id
            | "encode-error" -> assertEncodeError testCase id
            | "adler32" ->
                Assert.Equal(
                    testCase.GetProperty("expected").GetProperty("adler32_hex").GetString(),
                    Png.adler32 (fromHex (testCase.GetProperty("input_hex").GetString())) |> fun value -> value.ToString("x8"))
            | operation -> Assert.Fail(id + ": unknown operation " + operation)
