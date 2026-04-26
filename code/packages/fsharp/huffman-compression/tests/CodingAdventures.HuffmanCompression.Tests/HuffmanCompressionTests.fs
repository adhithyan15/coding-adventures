namespace CodingAdventures.HuffmanCompression.FSharp.Tests

open System
open System.Buffers.Binary
open System.Text
open CodingAdventures.HuffmanCompression.FSharp
open Xunit

type HuffmanCompressionTests() =
    let encodeText (value: string) = Encoding.UTF8.GetBytes value

    let repeatBytes (source: byte array) times =
        let output = Array.zeroCreate<byte> (source.Length * times)
        for index in 0 .. times - 1 do
            Array.Copy(source, 0, output, index * source.Length, source.Length)
        output

    [<Theory>]
    [<InlineData("AAABBC")>]
    [<InlineData("hello world")>]
    [<InlineData("Lorem ipsum dolor sit amet, consectetur adipiscing elit.")>]
    [<InlineData("ABCABCABCABC")>]
    [<InlineData("line\nline\nline\n")>]
    member _.``compress and decompress round trip text inputs``(value: string) =
        let data = encodeText value
        Assert.Equal<byte array>(data, HuffmanCompression.Decompress(HuffmanCompression.Compress data))

    [<Fact>]
    member _.``compress and decompress round trip all byte values``() =
        let data = [| for value in 0 .. 255 -> byte value |]
        Assert.Equal<byte array>(data, HuffmanCompression.Decompress(HuffmanCompression.Compress data))

    [<Fact>]
    member _.``compress and decompress round trip repeated all byte values``() =
        let data = repeatBytes [| for value in 0 .. 255 -> byte value |] 10
        Assert.Equal<byte array>(data, HuffmanCompression.Decompress(HuffmanCompression.Compress data))

    [<Fact>]
    member _.``compress and decompress round trip binary data``() =
        let data = [| 0uy; 1uy; 2uy; 3uy; 255uy; 254uy; 253uy; 128uy; 64uy; 32uy; 0uy; 255uy |]
        Assert.Equal<byte array>(data, HuffmanCompression.Decompress(HuffmanCompression.Compress data))

    [<Theory>]
    [<InlineData(0)>]
    [<InlineData(65)>]
    [<InlineData(127)>]
    [<InlineData(255)>]
    member _.``single symbol inputs round trip``(symbol: int) =
        let data = Array.create 50 (byte symbol)
        Assert.Equal<byte array>(data, HuffmanCompression.Decompress(HuffmanCompression.Compress data))

    [<Fact>]
    member _.``empty and null compress produce header only``() =
        Assert.Equal<byte array>(Array.zeroCreate 8, HuffmanCompression.Compress [||])
        Assert.Equal<byte array>(Array.zeroCreate 8, HuffmanCompression.Compress null)

    [<Fact>]
    member _.``empty and short decompress return empty``() =
        Assert.Empty(HuffmanCompression.Decompress [||])
        Assert.Empty(HuffmanCompression.Decompress null)
        Assert.Empty(HuffmanCompression.Decompress [| 0uy; 0uy; 0uy; 0uy |])
        Assert.Empty(HuffmanCompression.Decompress(HuffmanCompression.Compress [||]))

    [<Fact>]
    member _.``AAABBC matches exact CMP04 wire bytes``() =
        let result = HuffmanCompression.Compress(Encoding.ASCII.GetBytes "AAABBC")

        Assert.Equal(6u, BinaryPrimitives.ReadUInt32BigEndian(result.AsSpan(0, 4)))
        Assert.Equal(3u, BinaryPrimitives.ReadUInt32BigEndian(result.AsSpan(4, 4)))
        Assert.Equal<byte array>(
            [| 0x00uy; 0x00uy; 0x00uy; 0x06uy
               0x00uy; 0x00uy; 0x00uy; 0x03uy
               0x41uy; 0x01uy
               0x42uy; 0x02uy
               0x43uy; 0x02uy
               0xA8uy; 0x01uy |],
            result)

    [<Theory>]
    [<InlineData(1)>]
    [<InlineData(5)>]
    [<InlineData(100)>]
    [<InlineData(1000)>]
    member _.``wire format stores original length``(length: int) =
        let data = Array.create length (byte 'A')
        let compressed = HuffmanCompression.Compress data
        Assert.Equal(uint32 length, BinaryPrimitives.ReadUInt32BigEndian(compressed.AsSpan(0, 4)))

    [<Fact>]
    member _.``wire format stores sorted code lengths``() =
        let result = HuffmanCompression.Compress(Encoding.ASCII.GetBytes "AAABBC")
        let symbolCount = int (BinaryPrimitives.ReadUInt32BigEndian(result.AsSpan(4, 4)))
        let mutable previousLength = 0
        let mutable previousSymbol = -1

        for index in 0 .. symbolCount - 1 do
            let symbol = int result[8 + (index * 2)]
            let length = int result[8 + (index * 2) + 1]
            Assert.True(length > previousLength || (length = previousLength && symbol > previousSymbol))
            previousLength <- length
            previousSymbol <- symbol

    [<Fact>]
    member _.``single byte input uses one bit code``() =
        let result = HuffmanCompression.Compress(Encoding.ASCII.GetBytes "A")

        Assert.Equal(1u, BinaryPrimitives.ReadUInt32BigEndian(result.AsSpan(0, 4)))
        Assert.Equal(1u, BinaryPrimitives.ReadUInt32BigEndian(result.AsSpan(4, 4)))
        Assert.Equal(byte 'A', result[8])
        Assert.Equal(1uy, result[9])
        Assert.Equal(0uy, result[10])
        Assert.Equal(11, result.Length)

    [<Fact>]
    member _.``compressible inputs shrink``() =
        let data = Array.append (Array.create 900 (byte 'A')) (Array.create 100 (byte 'B'))
        let compressed = HuffmanCompression.Compress data
        Assert.True(compressed.Length < data.Length)

    [<Fact>]
    member _.``uniform small input can be larger than original``() =
        let data = [| for value in 0 .. 255 -> byte value |]
        let compressed = HuffmanCompression.Compress data
        Assert.True(compressed.Length > data.Length)

    [<Fact>]
    member _.``compression is deterministic``() =
        let data = Encoding.ASCII.GetBytes "the quick brown fox jumps over the lazy dog"
        Assert.Equal<byte array>(HuffmanCompression.Compress data, HuffmanCompression.Compress data)

    [<Fact>]
    member _.``decompress throws when bit stream is exhausted``() =
        let truncated = Array.zeroCreate<byte> 11
        BinaryPrimitives.WriteUInt32BigEndian(truncated.AsSpan(0, 4), 100u)
        BinaryPrimitives.WriteUInt32BigEndian(truncated.AsSpan(4, 4), 1u)
        truncated[8] <- byte 'A'
        truncated[9] <- 1uy

        let error = Assert.Throws<InvalidOperationException>(fun () -> HuffmanCompression.Decompress truncated |> ignore)
        Assert.Contains("exhausted", error.Message)

    [<Fact>]
    member _.``decompress throws when table is truncated``() =
        let truncated = Array.zeroCreate<byte> 9
        BinaryPrimitives.WriteUInt32BigEndian(truncated.AsSpan(0, 4), 1u)
        BinaryPrimitives.WriteUInt32BigEndian(truncated.AsSpan(4, 4), 1u)
        truncated[8] <- byte 'A'

        let error = Assert.Throws<InvalidOperationException>(fun () -> HuffmanCompression.Decompress truncated |> ignore)
        Assert.Contains("code-length table", error.Message)

    [<Fact>]
    member _.``decompress rejects zero length codes``() =
        let malformed = Array.zeroCreate<byte> 11
        BinaryPrimitives.WriteUInt32BigEndian(malformed.AsSpan(0, 4), 1u)
        BinaryPrimitives.WriteUInt32BigEndian(malformed.AsSpan(4, 4), 1u)
        malformed[8] <- byte 'A'

        let error = Assert.Throws<InvalidOperationException>(fun () -> HuffmanCompression.Decompress malformed |> ignore)
        Assert.Contains("positive", error.Message)
