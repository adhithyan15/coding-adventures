namespace CodingAdventures.Lz77.FSharp.Tests

open System
open System.Text
open CodingAdventures.Lz77.FSharp
open Xunit

type Lz77Tests() =
    let encodeText (value: string) = Encoding.UTF8.GetBytes value
    let decodeText (value: byte array) = Encoding.UTF8.GetString value

    [<Fact>]
    member _.``empty input round trips as no tokens``() =
        Assert.Empty(Lz77.Encode [||])
        Assert.Empty(Lz77.Decode [])

    [<Fact>]
    member _.``all identical bytes use overlap backreference``() =
        let tokens = Lz77.Encode(encodeText "AAAAAAA")

        Assert.Equal(2, List.length tokens)
        Assert.Equal({ Offset = 0; Length = 0; NextChar = byte 'A' }, List.item 0 tokens)
        Assert.Equal(1, (List.item 1 tokens).Offset)
        Assert.Equal(5, (List.item 1 tokens).Length)
        Assert.Equal(byte 'A', (List.item 1 tokens).NextChar)
        Assert.Equal("AAAAAAA", decodeText (Lz77.Decode tokens))

    [<Fact>]
    member _.``repeated pair uses single backreference``() =
        let tokens = Lz77.Encode(encodeText "ABABABAB")

        Assert.Equal(3, List.length tokens)
        Assert.Equal({ Offset = 0; Length = 0; NextChar = byte 'A' }, List.item 0 tokens)
        Assert.Equal({ Offset = 0; Length = 0; NextChar = byte 'B' }, List.item 1 tokens)
        Assert.Equal(2, (List.item 2 tokens).Offset)
        Assert.Equal(5, (List.item 2 tokens).Length)
        Assert.Equal(byte 'B', (List.item 2 tokens).NextChar)
        Assert.Equal("ABABABAB", decodeText (Lz77.Decode tokens))

    [<Fact>]
    member _.``AABCBBABC default min match leaves all literals``() =
        let tokens = Lz77.Encode(encodeText "AABCBBABC")

        Assert.Equal(9, List.length tokens)
        Assert.All(tokens, fun token ->
            Assert.Equal(0, token.Offset)
            Assert.Equal(0, token.Length))

    [<Theory>]
    [<InlineData("")>]
    [<InlineData("A")>]
    [<InlineData("ABCDE")>]
    [<InlineData("hello world")>]
    [<InlineData("ABABABABAB")>]
    member _.``compress and decompress round trip known inputs``(value: string) =
        let data = encodeText value
        Assert.Equal<byte array>(data, Lz77.Decompress(Lz77.Compress data))

    [<Fact>]
    member _.``decode uses initial buffer for streaming style backreferences``() =
        let result = Lz77.Decode([ { Offset = 2; Length = 3; NextChar = byte 'Z' } ], [| byte 'A'; byte 'B' |])
        Assert.Equal("ABABAZ", decodeText result)

    [<Fact>]
    member _.``encode respects window and max match limits``() =
        let data = Array.create 1000 (byte 'A')
        let tokens = Lz77.Encode(data, windowSize = 100, maxMatch = 50)

        Assert.All(tokens, fun token ->
            Assert.InRange(token.Offset, 0, 100)
            Assert.InRange(token.Length, 0, 50))

    [<Fact>]
    member _.``serialise and deserialise are inverse for teaching format``() =
        let tokens =
            [ { Offset = 0; Length = 0; NextChar = byte 'A' }
              { Offset = 2; Length = 5; NextChar = byte 'B' }
              { Offset = 1; Length = 3; NextChar = byte 'C' } ]

        let serialised = Lz77.SerialiseTokens(tokens)
        let deserialised = Lz77.DeserialiseTokens serialised

        Assert.Equal<Lz77Token list>(tokens, deserialised)

    [<Fact>]
    member _.``decode rejects offsets before the output buffer``() =
        let error =
            Assert.Throws<InvalidOperationException>(fun () ->
                Lz77.Decode([ { Offset = 4; Length = 1; NextChar = byte 'A' } ]) |> ignore)

        Assert.Contains("before the output buffer", error.Message)
