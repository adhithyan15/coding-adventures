namespace CodingAdventures.ReedSolomon.FSharp.Tests

open System.Text
open CodingAdventures.Gf256
open CodingAdventures.ReedSolomon.FSharp
open Xunit

module private Helpers =
    let bytes (value: string) = Encoding.UTF8.GetBytes(value)

    let corrupt (codeword: byte array) (positions: int list) (mask: byte) =
        let clone = Array.copy codeword

        for position in positions do
            clone[position] <- clone[position] ^^^ mask

        clone

type BuildGeneratorTests() =
    [<Fact>]
    member _.``nCheck two matches known vector``() =
        Assert.Equal<byte array>([| 8uy; 6uy; 1uy |], ReedSolomon.BuildGenerator(2))

    [<Fact>]
    member _.``generator is monic and has expected degree``() =
        let generator = ReedSolomon.BuildGenerator(8)
        Assert.Equal(9, generator.Length)
        Assert.Equal(1uy, generator[generator.Length - 1])

    [<Fact>]
    member _.``generator roots are consecutive alpha powers``() =
        let nCheck = 4
        let generator = ReedSolomon.BuildGenerator(nCheck)

        for powerIndex in 1 .. nCheck do
            let root = Gf256.power 2uy powerIndex
            let mutable value = 0uy

            for index in generator.Length - 1 .. -1 .. 0 do
                value <- Gf256.add (Gf256.multiply value root) generator[index]

            Assert.Equal(0uy, value)

    [<Fact>]
    member _.``odd nCheck throws``() =
        Assert.Throws<InvalidInputError>(fun () -> ReedSolomon.BuildGenerator(3) |> ignore) |> ignore

type EncodeAndSyndromeTests() =
    [<Fact>]
    member _.``encode is systematic``() =
        let message = Helpers.bytes "hello RS"
        let codeword = ReedSolomon.Encode(message, 4)
        Assert.Equal<byte array>(message, codeword[.. message.Length - 1])

    [<Fact>]
    member _.``valid codeword has zero syndromes``() =
        let codeword = ReedSolomon.Encode(Helpers.bytes "syndromes", 6)
        Assert.All<byte>(ReedSolomon.Syndromes(codeword, 6), fun value -> Assert.Equal(0uy, value))

    [<Fact>]
    member _.``empty message encodes to zero syndrome codeword``() =
        let codeword = ReedSolomon.Encode(Array.empty, 4)
        Assert.Equal(4, codeword.Length)
        Assert.All<byte>(ReedSolomon.Syndromes(codeword, 4), fun value -> Assert.Equal(0uy, value))

    [<Fact>]
    member _.``max length codeword is accepted``() =
        let codeword = ReedSolomon.Encode([| 0x42uy |], 254)
        Assert.Equal(255, codeword.Length)
        Assert.All<byte>(ReedSolomon.Syndromes(codeword, 254), fun value -> Assert.Equal(0uy, value))

    [<Fact>]
    member _.``oversized codeword throws``() =
        let message = Array.zeroCreate<byte> 240
        let error = Assert.Throws<InvalidInputError>(fun () -> ReedSolomon.Encode(message, 20) |> ignore)
        Assert.Contains("exceeds GF(256) block size limit", error.Message)

type DecodeTests() =
    [<Fact>]
    member _.``decode returns original on clean codeword``() =
        let message = Helpers.bytes "Reed-Solomon coding is beautiful"
        let recovered = ReedSolomon.Decode(ReedSolomon.Encode(message, 8), 8)
        Assert.Equal<byte array>(message, recovered)

    [<Fact>]
    member _.``t1 corrects single error``() =
        let message = Helpers.bytes "abc"
        let codeword = Helpers.corrupt (ReedSolomon.Encode(message, 2)) [ 1 ] 0x5Auy
        Assert.Equal<byte array>(message, ReedSolomon.Decode(codeword, 2))

    [<Fact>]
    member _.``t2 corrects two errors``() =
        let message = Helpers.bytes "four check bytes"
        let codeword = Helpers.corrupt (ReedSolomon.Encode(message, 4)) [ 0; 5 ] 0xAAuy
        Assert.Equal<byte array>(message, ReedSolomon.Decode(codeword, 4))

    [<Fact>]
    member _.``t4 corrects four errors``() =
        let message = Helpers.bytes "eight check bytes give t=4"
        let codeword = ReedSolomon.Encode(message, 8)
        codeword[0] <- codeword[0] ^^^ 0xFFuy
        codeword[3] <- codeword[3] ^^^ 0xAAuy
        codeword[10] <- codeword[10] ^^^ 0x55uy
        codeword[14] <- codeword[14] ^^^ 0x0Fuy
        Assert.Equal<byte array>(message, ReedSolomon.Decode(codeword, 8))

    [<Fact>]
    member _.``errors in check bytes are corrected``() =
        let message = Helpers.bytes "check byte error"
        let codeword = ReedSolomon.Encode(message, 4)
        codeword[message.Length] <- codeword[message.Length] ^^^ 0x33uy
        Assert.Equal<byte array>(message, ReedSolomon.Decode(codeword, 4))

    [<Fact>]
    member _.``too many errors throws``() =
        let message = Helpers.bytes "too many errors"
        let codeword = Helpers.corrupt (ReedSolomon.Encode(message, 4)) [ 0; 2; 4 ] 0x77uy
        Assert.Throws<TooManyErrorsError>(fun () -> ReedSolomon.Decode(codeword, 4) |> ignore) |> ignore

    [<Fact>]
    member _.``too short received throws``() =
        Assert.Throws<InvalidInputError>(fun () -> ReedSolomon.Decode([| 1uy; 2uy; 3uy |], 4) |> ignore) |> ignore

type ErrorLocatorTests() =
    [<Fact>]
    member _.``error locator for clean codeword is one``() =
        let lambda = ReedSolomon.ErrorLocator(ReedSolomon.Syndromes(ReedSolomon.Encode(Helpers.bytes "clean", 4), 4))
        Assert.Equal<byte array>([| 1uy |], lambda)

    [<Fact>]
    member _.``error locator degree matches correctable errors``() =
        let message = Helpers.bytes "locator"
        let codeword = Helpers.corrupt (ReedSolomon.Encode(message, 4)) [ 0; 5 ] 0x12uy
        let lambda = ReedSolomon.ErrorLocator(ReedSolomon.Syndromes(codeword, 4))
        Assert.Equal(3, lambda.Length)
        Assert.Equal(1uy, lambda[0])

type ValidationTests() =
    [<Fact>]
    member _.``decode rejects odd nCheck``() =
        Assert.Throws<InvalidInputError>(fun () -> ReedSolomon.Decode(Helpers.bytes "abc", 3) |> ignore) |> ignore

    [<Fact>]
    member _.``encode rejects zero nCheck``() =
        Assert.Throws<InvalidInputError>(fun () -> ReedSolomon.Encode(Helpers.bytes "abc", 0) |> ignore) |> ignore

    [<Fact>]
    member _.``single byte round trips``() =
        let recovered = ReedSolomon.Decode(ReedSolomon.Encode([| 0x42uy |], 2), 2)
        Assert.Equal<byte array>([| 0x42uy |], recovered)

    [<Fact>]
    member _.``binary payload round trips``() =
        let message = Array.init 50 (fun index -> byte ((index * 37 + 13) &&& 0xFF))
        Assert.Equal<byte array>(message, ReedSolomon.Decode(ReedSolomon.Encode(message, 10), 10))
