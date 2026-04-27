namespace CodingAdventures.Lzss.FSharp.Tests

open System
open System.Buffers.Binary
open System.Text
open CodingAdventures.Lzss.FSharp
open Xunit

type LzssTests() =
    let encodeText (value: string) = Encoding.UTF8.GetBytes value

    [<Fact>]
    member _.``empty input produces no tokens``() =
        Assert.Empty(Lzss.Encode [||])

    [<Fact>]
    member _.``single byte produces literal``() =
        Assert.Equal<LzssToken list>([ Literal(byte 'A') ], Lzss.Encode(encodeText "A"))

    [<Fact>]
    member _.``AABCBBABC matches spec vector tail``() =
        let tokens = Lzss.Encode(encodeText "AABCBBABC")
        Assert.Equal(7, List.length tokens)
        Assert.Equal(Match(5, 3), List.last tokens)

    [<Fact>]
    member _.``ABABAB uses match token``() =
        Assert.Equal<LzssToken list>(
            [ Literal(byte 'A'); Literal(byte 'B'); Match(2, 4) ],
            Lzss.Encode(encodeText "ABABAB"))

    [<Fact>]
    member _.``all same bytes use self-referential match``() =
        Assert.Equal<LzssToken list>(
            [ Literal(byte 'A'); Match(1, 6) ],
            Lzss.Encode(encodeText "AAAAAAA"))

    [<Fact>]
    member _.``decode handles overlapping matches``() =
        let output = Lzss.Decode([ Literal(byte 'A'); Match(1, 6) ], 7)
        Assert.Equal<byte array>(encodeText "AAAAAAA", output)

    [<Theory>]
    [<InlineData("")>]
    [<InlineData("A")>]
    [<InlineData("ABCDE")>]
    [<InlineData("AAAAAAA")>]
    [<InlineData("ABABAB")>]
    [<InlineData("AABCBBABC")>]
    [<InlineData("hello world")>]
    member _.``compress and decompress round trip ascii inputs``(value: string) =
        let data = encodeText value
        Assert.Equal<byte array>(data, Lzss.Decompress(Lzss.Compress data))

    [<Fact>]
    member _.``binary and repeated data round trip``() =
        let data = Array.init 300 (fun index -> byte (index % 3))
        Assert.Equal<byte array>(data, Lzss.Decompress(Lzss.Compress data))

    [<Fact>]
    member _.``serialise and deserialise are symmetric``() =
        let tokens = [ Literal(byte 'A'); Literal(byte 'B'); Match(2, 4) ]
        let bytes = Lzss.SerialiseTokens(tokens, 6)
        let recovered, originalLength = Lzss.DeserialiseTokens bytes

        Assert.Equal(6, originalLength)
        Assert.Equal<LzssToken list>(tokens, recovered)

    [<Fact>]
    member _.``deserialise caps crafted large block count``() =
        let bad = Array.zeroCreate<byte> 16
        BinaryPrimitives.WriteUInt32BigEndian(bad.AsSpan(4, 4), 0x40000000u)
        let output = Lzss.Decompress bad
        Assert.NotNull output

    [<Fact>]
    member _.``decode rejects offsets before output buffer``() =
        let error = Assert.Throws<InvalidOperationException>(fun () -> Lzss.Decode([ Match(4, 1) ]) |> ignore)
        Assert.Contains("before the output buffer", error.Message)
