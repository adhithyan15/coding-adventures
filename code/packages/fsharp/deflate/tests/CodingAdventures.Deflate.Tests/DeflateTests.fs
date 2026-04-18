namespace CodingAdventures.Deflate.FSharp.Tests

open System
open System.Buffers.Binary
open System.Text
open CodingAdventures.Deflate.FSharp
open Xunit

module private Helpers =
    let bytes (value: string) = Encoding.UTF8.GetBytes(value)

type DeflateTests() =
    [<Fact>]
    member _.``Empty input uses minimal header``() =
        let compressed = Deflate.Compress(Array.empty)
        Assert.Equal(12, compressed.Length)
        Assert.Equal(0u, BinaryPrimitives.ReadUInt32BigEndian(compressed.AsSpan(0, 4)))
        Assert.Equal(1us, BinaryPrimitives.ReadUInt16BigEndian(compressed.AsSpan(4, 2)))
        Assert.Equal(0us, BinaryPrimitives.ReadUInt16BigEndian(compressed.AsSpan(6, 2)))
        Assert.Equal<byte array>(Array.empty, Deflate.Decompress(compressed))

    [<Theory>]
    [<InlineData(0uy)>]
    [<InlineData(255uy)>]
    member _.``Single byte values round trip``(value: byte) =
        let data = [| value |]
        Assert.Equal<byte array>(data, Deflate.Decompress(Deflate.Compress(data)))

    [<Fact>]
    member _.``Literal-only example has no distance tree``() =
        let data = Helpers.bytes "AAABBC"
        let compressed = Deflate.Compress(data)
        Assert.Equal<byte array>(data, Deflate.Decompress(compressed))
        Assert.Equal(0us, BinaryPrimitives.ReadUInt16BigEndian(compressed.AsSpan(6, 2)))

    [<Fact>]
    member _.``Spec match example has distance tree``() =
        let data = Helpers.bytes "AABCBBABC"
        let compressed = Deflate.Compress(data)
        Assert.Equal(9u, BinaryPrimitives.ReadUInt32BigEndian(compressed.AsSpan(0, 4)))
        Assert.True(BinaryPrimitives.ReadUInt16BigEndian(compressed.AsSpan(6, 2)) > 0us)
        Assert.Equal<byte array>(data, Deflate.Decompress(compressed))

    [<Theory>]
    [<InlineData("AAAAAAA")>]
    [<InlineData("ABABABABABAB")>]
    [<InlineData("ABCABCABCABC")>]
    [<InlineData("hello hello hello world")>]
    [<InlineData("AABABC")>]
    member _.``Match-heavy examples round trip``(value: string) =
        let data = Helpers.bytes value
        Assert.Equal<byte array>(data, Deflate.Decompress(Deflate.Compress(data)))

    [<Fact>]
    member _.``Long repetitive text round trips``() =
        let data = Helpers.bytes(String.Concat(Array.replicate 10 "the quick brown fox jumps over the lazy dog "))
        Assert.Equal<byte array>(data, Deflate.Decompress(Deflate.Compress(data)))

    [<Fact>]
    member _.``Binary data round trips``() =
        let data = Array.init 1000 (fun index -> byte (index % 256))
        Assert.Equal<byte array>(data, Deflate.Decompress(Deflate.Compress(data)))

    [<Fact>]
    member _.``Repetitive data compresses below half``() =
        let baseBytes = Helpers.bytes "ABCABC"
        let data = Array.zeroCreate<byte> (baseBytes.Length * 100)
        for index in 0 .. 99 do
            Array.Copy(baseBytes, 0, data, index * baseBytes.Length, baseBytes.Length)

        let compressed = Deflate.Compress(data)
        Assert.True(float compressed.Length < float data.Length / 2.0)
