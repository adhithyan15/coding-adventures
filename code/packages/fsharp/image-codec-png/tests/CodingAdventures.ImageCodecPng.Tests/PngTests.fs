namespace CodingAdventures.ImageCodecPng.FSharp.Tests

open System
open System.Buffers.Binary
open System.IO
open System.IO.Compression
open System.Text
open CodingAdventures.ImageCodecPng.FSharp
open CodingAdventures.PixelContainer
open CodingAdventures.Zip.FSharp
open Xunit

module PngTests =
    let private assertCode expected (action: unit -> unit) =
        let error = Assert.Throws<PngError>(Action action)
        Assert.Equal(expected, error.Code)
        Assert.Equal(expected, error.Message)
        Assert.Empty(error.Data)

    let private insertAfterIhdr (png: byte[]) (chunkType: string) validCrc =
        let ihdrEnd = 8 + 12 + 13
        let typeBytes = Encoding.ASCII.GetBytes(chunkType)
        let chunk = Array.zeroCreate<byte> 12
        Array.Copy(typeBytes, 0, chunk, 4, 4)
        let crc = RawRfc1951.crc32 typeBytes 0u
        BinaryPrimitives.WriteUInt32BigEndian(chunk.AsSpan(8), if validCrc then crc else crc ^^^ 1u)
        Array.concat [ png[.. ihdrEnd - 1]; chunk; png[ihdrEnd..] ]

    [<Fact>]
    let ``codec implements the image contract and round trips`` () =
        let codec: IImageCodec = PngCodec()
        Assert.Equal("image/png", codec.MimeType)
        let pixels = PixelContainer(2, 1, [| 1uy; 2uy; 3uy; 4uy; 250uy; 240uy; 230uy; 220uy |])
        let decoded = codec.Decode(codec.Encode(pixels))
        Assert.Equal(2, decoded.Width)
        Assert.Equal(1, decoded.Height)
        Assert.Equal<byte>(pixels.Data, decoded.Data)

        let limited: IImageCodec = PngCodec(2.0)
        Assert.Equal<byte>(pixels.Data, limited.Decode(limited.Encode(pixels)).Data)

    [<Fact>]
    let ``caller pixel limit must be a positive lowering integer`` () =
        let invalid =
            [| 0.0; -1.0; 1.5; float Png.defaultMaxPixels + 1.0
               Double.NaN; Double.PositiveInfinity; Double.NegativeInfinity |]
        for value in invalid do
            assertCode "invalid-max-pixels" (fun () -> PngCodec(value) |> ignore)
            assertCode "invalid-max-pixels" (fun () -> Png.decodePng [||] (Some value) |> ignore)

    [<Fact>]
    let ``error taxonomy is closed and copied`` () =
        let first = Png.errorCodes()
        let second = Png.errorCodes()
        Assert.Equal(29, first.Length)
        first[0] <- "changed"
        Assert.Equal("invalid-max-pixels", second[0])

    [<Fact>]
    let ``encoder rejects typed shape and resource failures`` () =
        assertCode "invalid-image-dimensions" (fun () -> Png.encodePng (Unchecked.defaultof<PixelContainer>) |> ignore)
        assertCode "invalid-image-dimensions" (fun () -> Png.encodePng (PixelContainer(0, 1)) |> ignore)
        assertCode "invalid-image-dimensions" (fun () -> Png.encodePng (PixelContainer(Png.maxDimension + 1, 1)) |> ignore)

    [<Fact>]
    let ``APNG names are rejected after ordinary CRC validation`` () =
        let encoded = Png.encodePng (PixelContainer(1, 1))
        for chunkType in [| "acTL"; "fcTL"; "fdAT" |] do
            assertCode "unsupported-feature" (fun () -> Png.decodePng (insertAfterIhdr encoded chunkType true) None |> ignore)
            assertCode "chunk-crc-mismatch" (fun () -> Png.decodePng (insertAfterIhdr encoded chunkType false) None |> ignore)

    [<Fact>]
    let ``Adler32 matches the published vector and zlib trailer`` () =
        Assert.Equal(0x11e60398u, Png.adler32 (Encoding.ASCII.GetBytes("Wikipedia")))
        let data = Array.init 6000 (fun index -> byte (index * 31))
        use stream = new MemoryStream()
        do
            use zlib = new ZLibStream(stream, CompressionLevel.SmallestSize, true)
            zlib.Write(data)
        let wrapped = stream.ToArray()
        Assert.Equal(Png.adler32 data, BinaryPrimitives.ReadUInt32BigEndian(ReadOnlySpan<byte>(wrapped, wrapped.Length - 4, 4)))
