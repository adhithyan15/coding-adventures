namespace CodingAdventures.Lz78.FSharp.Tests

open System
open System.Text
open CodingAdventures.Lz78.FSharp
open Xunit

type Lz78Tests() =
    let encodeText (value: string) = Encoding.UTF8.GetBytes value
    let decodeText (value: byte array) = Encoding.UTF8.GetString value

    [<Fact>]
    member _.``empty input produces no tokens``() =
        Assert.Empty(Lz78.Encode [||])
        Assert.Empty(Lz78.Decode([], 0))

    [<Fact>]
    member _.``single byte produces single literal token``() =
        let tokens = Lz78.Encode(encodeText "A")
        Assert.Equal<Lz78Token list>([ { DictIndex = 0; NextChar = byte 'A' } ], tokens)
        Assert.Equal("A", decodeText (Lz78.Decode(tokens, 1)))

    [<Fact>]
    member _.``AABCBBABC matches spec vector``() =
        let tokens = Lz78.Encode(encodeText "AABCBBABC")

        Assert.Equal<Lz78Token list>(
            [ { DictIndex = 0; NextChar = byte 'A' }
              { DictIndex = 1; NextChar = byte 'B' }
              { DictIndex = 0; NextChar = byte 'C' }
              { DictIndex = 0; NextChar = byte 'B' }
              { DictIndex = 4; NextChar = byte 'A' }
              { DictIndex = 4; NextChar = byte 'C' } ],
            tokens)

        Assert.Equal("AABCBBABC", decodeText (Lz78.Decompress(Lz78.Compress(encodeText "AABCBBABC"))))

    [<Fact>]
    member _.``ABABAB uses flush token``() =
        let tokens = Lz78.Encode(encodeText "ABABAB")
        Assert.Equal<Lz78Token list>(
            [ { DictIndex = 0; NextChar = byte 'A' }
              { DictIndex = 0; NextChar = byte 'B' }
              { DictIndex = 1; NextChar = byte 'B' }
              { DictIndex = 3; NextChar = 0uy } ],
            tokens)

    [<Theory>]
    [<InlineData("")>]
    [<InlineData("A")>]
    [<InlineData("ABCDE")>]
    [<InlineData("AAAAAAA")>]
    [<InlineData("ABABABAB")>]
    [<InlineData("hello world")>]
    member _.``compress and decompress round trip ascii inputs``(value: string) =
        let data = encodeText value
        Assert.Equal<byte array>(data, Lz78.Decompress(Lz78.Compress data))

    [<Fact>]
    member _.``binary inputs round trip``() =
        let data = [| 0uy; 0uy; 0uy; 255uy; 255uy; 0uy; 1uy; 2uy; 0uy; 1uy; 2uy |]
        Assert.Equal<byte array>(data, Lz78.Decompress(Lz78.Compress data))

    [<Fact>]
    member _.``max dictionary size is respected``() =
        let tokens = Lz78.Encode(encodeText "ABCABCABCABCABC", maxDictSize = 10)
        Assert.All(tokens, fun token -> Assert.InRange(token.DictIndex, 0, 9))

    [<Fact>]
    member _.``max dictionary size one forces all literals``() =
        let tokens = Lz78.Encode(encodeText "AAAA", maxDictSize = 1)
        Assert.All(tokens, fun token -> Assert.Equal(0, token.DictIndex))

    [<Fact>]
    member _.``serialise and deserialise are symmetric``() =
        let tokens =
            [ { DictIndex = 0; NextChar = byte 'A' }
              { DictIndex = 1; NextChar = byte 'B' } ]

        let serialised = Lz78.SerialiseTokens(tokens, 3)
        let deserialised, originalLength = Lz78.DeserialiseTokens serialised

        Assert.Equal(3, originalLength)
        Assert.Equal<Lz78Token list>(tokens, deserialised)

    [<Fact>]
    member _.``decode rejects unknown dictionary index``() =
        let error = Assert.Throws<InvalidOperationException>(fun () -> Lz78.Decode([ { DictIndex = 1; NextChar = byte 'A' } ]) |> ignore)
        Assert.Contains("does not exist", error.Message)
