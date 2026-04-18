namespace CodingAdventures.Lzw.FSharp.Tests

open System
open System.Buffers.Binary
open System.Text
open CodingAdventures.Lzw.FSharp
open Xunit

module private Helpers =
    let bytes (value: string) = Encoding.UTF8.GetBytes(value)

type ConstantTests() =
    [<Fact>]
    member _.``Constants match CMP03``() =
        Assert.Equal(256, Lzw.CLEAR_CODE)
        Assert.Equal(257, Lzw.STOP_CODE)
        Assert.Equal(258, Lzw.INITIAL_NEXT_CODE)
        Assert.Equal(9, Lzw.INITIAL_CODE_SIZE)
        Assert.Equal(16, Lzw.MAX_CODE_SIZE)

type BitIoTests() =
    [<Fact>]
    member _.``Single 9-bit code round trips``() =
        let writer = BitWriter()
        writer.Write(256, 9)
        writer.Flush()

        let reader = BitReader(writer.ToArray())
        Assert.Equal(256, reader.Read(9))

    [<Fact>]
    member _.``Multiple 9-bit codes round trip``() =
        let writer = BitWriter()
        for code in [ Lzw.CLEAR_CODE; 65; 66; 258; Lzw.STOP_CODE ] do
            writer.Write(code, 9)

        writer.Flush()

        let reader = BitReader(writer.ToArray())
        Assert.Equal(Lzw.CLEAR_CODE, reader.Read(9))
        Assert.Equal(65, reader.Read(9))
        Assert.Equal(66, reader.Read(9))
        Assert.Equal(258, reader.Read(9))
        Assert.Equal(Lzw.STOP_CODE, reader.Read(9))

    [<Fact>]
    member _.``Exhausted reader throws``() =
        let reader = BitReader(Array.empty)
        Assert.Throws<InvalidOperationException>(fun () -> reader.Read(9) |> ignore) |> ignore

type EncodeDecodeCodeTests() =
    [<Fact>]
    member _.``Empty input encodes to CLEAR and STOP``() =
        let codes, originalLength = Lzw.EncodeCodes(Array.empty)
        Assert.Equal(0, originalLength)
        Assert.Equal<int list>([ Lzw.CLEAR_CODE; Lzw.STOP_CODE ], codes)

    [<Fact>]
    member _.``AB encodes to expected vector``() =
        let codes, _ = Lzw.EncodeCodes(Helpers.bytes "AB")
        Assert.Equal<int list>([ Lzw.CLEAR_CODE; 65; 66; Lzw.STOP_CODE ], codes)

    [<Fact>]
    member _.``ABABAB encodes to expected vector``() =
        let codes, _ = Lzw.EncodeCodes(Helpers.bytes "ABABAB")
        Assert.Equal<int list>([ Lzw.CLEAR_CODE; 65; 66; 258; 258; Lzw.STOP_CODE ], codes)

    [<Fact>]
    member _.``AAAAAAA encodes to tricky token vector``() =
        let codes, _ = Lzw.EncodeCodes(Helpers.bytes "AAAAAAA")
        Assert.Equal<int list>([ Lzw.CLEAR_CODE; 65; 258; 259; 65; Lzw.STOP_CODE ], codes)

    [<Fact>]
    member _.``ABABAB decodes from expected vector``() =
        let decoded = Lzw.DecodeCodes([ Lzw.CLEAR_CODE; 65; 66; 258; 258; Lzw.STOP_CODE ])
        Assert.Equal<byte array>(Helpers.bytes "ABABAB", decoded)

    [<Fact>]
    member _.``Tricky token vector decodes``() =
        let decoded = Lzw.DecodeCodes([ Lzw.CLEAR_CODE; 65; 258; 259; 65; Lzw.STOP_CODE ])
        Assert.Equal<byte array>(Helpers.bytes "AAAAAAA", decoded)

    [<Fact>]
    member _.``CLEAR resets dictionary``() =
        let decoded = Lzw.DecodeCodes([ Lzw.CLEAR_CODE; 65; Lzw.CLEAR_CODE; 66; Lzw.STOP_CODE ])
        Assert.Equal<byte array>(Helpers.bytes "AB", decoded)

    [<Fact>]
    member _.``Invalid code throws``() =
        let error =
            Assert.Throws<InvalidOperationException>(fun () ->
                Lzw.DecodeCodes([ Lzw.CLEAR_CODE; 9999; 65; Lzw.STOP_CODE ]) |> ignore)

        Assert.Contains("invalid LZW code", error.Message)

type PackUnpackTests() =
    [<Fact>]
    member _.``Header stores original length big endian``() =
        let packed = Lzw.PackCodes([ Lzw.CLEAR_CODE; Lzw.STOP_CODE ], 42)
        Assert.Equal(42u, BinaryPrimitives.ReadUInt32BigEndian(packed.AsSpan(0, 4)))

    [<Fact>]
    member _.``ABABAB codes round trip through wire format``() =
        let codes = [ Lzw.CLEAR_CODE; 65; 66; 258; 258; Lzw.STOP_CODE ]
        let packed = Lzw.PackCodes(codes, 6)
        let unpacked, originalLength = Lzw.UnpackCodes(packed)

        Assert.Equal(6, originalLength)
        Assert.Equal<int list>(codes, unpacked)

    [<Fact>]
    member _.``Truncated input does not crash unpack``() =
        let codes, originalLength = Lzw.UnpackCodes([| 0uy; 0uy |])
        Assert.Empty(codes)
        Assert.Equal(0, originalLength)

type CompressDecompressTests() =
    [<Theory>]
    [<InlineData("")>]
    [<InlineData("A")>]
    [<InlineData("AB")>]
    [<InlineData("ABABAB")>]
    [<InlineData("AAAAAAA")>]
    [<InlineData("AABABC")>]
    member _.``String vectors round trip``(value: string) =
        let input = Helpers.bytes value
        Assert.Equal<byte array>(input, Lzw.Decompress(Lzw.Compress input))

    [<Fact>]
    member _.``Repetitive text round trips``() =
        let input = Helpers.bytes(String.Concat(Array.replicate 20 "the quick brown fox jumps over the lazy dog "))
        Assert.Equal<byte array>(input, Lzw.Decompress(Lzw.Compress input))

    [<Fact>]
    member _.``Binary data round trips``() =
        let input = Array.init 1024 (fun index -> byte (index % 256))
        Assert.Equal<byte array>(input, Lzw.Decompress(Lzw.Compress input))

    [<Fact>]
    member _.``Header contains original length``() =
        let input = Helpers.bytes "hello world"
        let compressed = Lzw.Compress(input)
        Assert.Equal(uint32 input.Length, BinaryPrimitives.ReadUInt32BigEndian(compressed.AsSpan(0, 4)))
