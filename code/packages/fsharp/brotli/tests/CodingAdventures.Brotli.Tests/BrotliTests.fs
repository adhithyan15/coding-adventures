namespace CodingAdventures.Brotli.FSharp.Tests

open System
open System.Buffers.Binary
open System.Text
open CodingAdventures.Brotli.FSharp
open Xunit

module private Helpers =
    let bytes (value: string) = Encoding.UTF8.GetBytes(value)

type BrotliTests() =
    [<Fact>]
    member _.``Empty input uses minimal encoding``() =
        let compressed = Brotli.Compress(Array.empty)
        Assert.Equal(13, compressed.Length)
        Assert.Equal(0u, BinaryPrimitives.ReadUInt32BigEndian(compressed.AsSpan(0, 4)))
        Assert.Equal(1uy, compressed[4])
        Assert.Equal(0uy, compressed[5])
        Assert.Equal<byte array>(Array.empty, Brotli.Decompress(compressed))

    [<Theory>]
    [<InlineData(0x42uy)>]
    [<InlineData(0x00uy)>]
    [<InlineData(0xFFuy)>]
    member _.``Single byte values round trip``(value: byte) =
        let data = [| value |]
        Assert.Equal<byte array>(data, Brotli.Decompress(Brotli.Compress(data)))

    [<Fact>]
    member _.``Repeated A data compresses well``() =
        let data = Array.create 1024 (byte 'A')
        let compressed = Brotli.Compress(data)

        Assert.Equal<byte array>(data, Brotli.Decompress(compressed))
        Assert.True(float compressed.Length < float data.Length / 2.0)
        Assert.True(compressed[5] > 0uy)

    [<Fact>]
    member _.``English prose round trips``() =
        let passage = "The quick brown fox jumps over the lazy dog. Pack my box with five dozen liquor jugs. "
        let data = Helpers.bytes(String.Concat(Array.replicate 16 passage))
        let compressed = Brotli.Compress(data)

        Assert.Equal<byte array>(data, Brotli.Decompress(compressed))
        Assert.True(float compressed.Length < float data.Length * 0.8)

    [<Fact>]
    member _.``Binary data round trips``() =
        let data = Array.zeroCreate<byte> 512
        let mutable state = 0xDEADBEEFu
        for index in 0 .. data.Length - 1 do
            state <- (state >>> 1) ^^^ (if (state &&& 1u) = 0u then 0u else 0xEDB88320u)
            data[index] <- byte (state &&& 0xFFu)

        Assert.Equal<byte array>(data, Brotli.Decompress(Brotli.Compress(data)))

    [<Fact>]
    member _.``Context transitions round trip``() =
        let data = Helpers.bytes "abc123ABCabc"
        let compressed = Brotli.Compress(data)

        Assert.Equal<byte array>(data, Brotli.Decompress(compressed))
        Assert.True(compressed[6] > 0uy)
        Assert.True(compressed[7] > 0uy)
        Assert.True(compressed[8] > 0uy)

    [<Fact>]
    member _.``Long-distance match round trips``() =
        let marker = Helpers.bytes "XYZABCDEFG"
        let filler = Array.create 4200 (byte 'B')
        let data = Array.zeroCreate<byte> (marker.Length + filler.Length + marker.Length)
        Array.Copy(marker, 0, data, 0, marker.Length)
        Array.Copy(filler, 0, data, marker.Length, filler.Length)
        Array.Copy(marker, 0, data, marker.Length + filler.Length, marker.Length)

        let compressed = Brotli.Compress(data)
        Assert.Equal<byte array>(data, Brotli.Decompress(compressed))
        Assert.True(compressed[5] > 0uy)

    [<Fact>]
    member _.``Compression is deterministic``() =
        let data = Helpers.bytes "The quick brown fox jumps over the lazy dog. The quick brown fox jumps over the lazy dog."
        let a = Brotli.Compress(data)
        let b = Brotli.Compress(data)
        Assert.Equal<byte array>(a, b)

    [<Fact>]
    member _.``Manual payload for single A decompresses``() =
        let payload =
            [|
                0x00uy; 0x00uy; 0x00uy; 0x01uy
                0x01uy
                0x00uy
                0x01uy
                0x00uy
                0x00uy
                0x00uy
                0x3Fuy; 0x01uy
                0x00uy; 0x41uy; 0x01uy
                0x00uy
            |]

        Assert.Equal<byte array>(Helpers.bytes "A", Brotli.Decompress(payload))

    [<Fact>]
    member _.``Manual payload for empty input decompresses``() =
        let payload =
            [|
                0x00uy; 0x00uy; 0x00uy; 0x00uy
                0x01uy
                0x00uy
                0x00uy
                0x00uy
                0x00uy
                0x00uy
                0x3Fuy; 0x01uy
                0x00uy
            |]

        Assert.Equal<byte array>(Array.empty, Brotli.Decompress(payload))

    [<Fact>]
    member _.``Header stores original length``() =
        let data = Helpers.bytes "Hello, Brotli!"
        let compressed = Brotli.Compress(data)
        Assert.Equal(uint32 data.Length, BinaryPrimitives.ReadUInt32BigEndian(compressed.AsSpan(0, 4)))

    [<Fact>]
    member _.``ICC sentinel always present``() =
        let compressed = Brotli.Compress(Helpers.bytes "test")
        Assert.True(compressed[4] > 0uy)

    [<Fact>]
    member _.``All distinct byte values round trip``() =
        let data = Array.init 256 byte
        Assert.Equal<byte array>(data, Brotli.Decompress(Brotli.Compress(data)))
