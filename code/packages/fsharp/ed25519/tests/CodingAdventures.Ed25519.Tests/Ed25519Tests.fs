namespace CodingAdventures.Ed25519.FSharp.Tests

open System
open Xunit
open CodingAdventures.Ed25519.FSharp

module Ed25519Tests =
    let private hex (value: string) = Convert.FromHexString value

    [<Theory>]
    [<InlineData(
        "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60",
        "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a",
        "",
        "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e065224901555fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b")>]
    [<InlineData(
        "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb",
        "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c",
        "72",
        "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00")>]
    [<InlineData(
        "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7",
        "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025",
        "af82",
        "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a")>]
    let ``matches RFC 8032 vectors`` seedHex publicKeyHex messageHex signatureHex =
        let seed = hex seedHex
        let message = hex messageHex
        let expectedPublicKey = hex publicKeyHex
        let expectedSignature = hex signatureHex
        let publicKey, secretKey = Ed25519.generateKeypair seed

        Assert.Equal<byte array>(expectedPublicKey, publicKey)
        Assert.Equal<byte array>(Array.append seed publicKey, secretKey)
        Assert.Equal<byte array>(expectedSignature, Ed25519.sign message secretKey)
        Assert.True(Ed25519.verify message expectedSignature publicKey)

    [<Fact>]
    let ``key generation and signing are deterministic`` () =
        let seed = [| 0uy .. 31uy |]
        let message = Text.Encoding.UTF8.GetBytes "deterministic"
        let firstPublic, firstSecret = Ed25519.generateKeypair seed
        let secondPublic, secondSecret = Ed25519.generateKeypair seed

        Assert.Equal<byte array>(firstPublic, secondPublic)
        Assert.Equal<byte array>(firstSecret, secondSecret)
        Assert.Equal<byte array>(Ed25519.sign message firstSecret, Ed25519.sign message firstSecret)

    [<Fact>]
    let ``verification rejects tampering wrong messages and wrong keys`` () =
        let seed = [| 0uy .. 31uy |]
        let otherSeed = [| 32uy .. 63uy |]
        let publicKey, secretKey = Ed25519.generateKeypair seed
        let otherPublicKey, _ = Ed25519.generateKeypair otherSeed
        let message = Text.Encoding.UTF8.GetBytes "hello"
        let signature = Ed25519.sign message secretKey
        let tamperedR = Array.copy signature
        tamperedR[0] <- tamperedR[0] ^^^ 1uy
        let tamperedS = Array.copy signature
        tamperedS[32] <- tamperedS[32] ^^^ 1uy

        Assert.False(Ed25519.verify (Text.Encoding.UTF8.GetBytes "world") signature publicKey)
        Assert.False(Ed25519.verify message signature otherPublicKey)
        Assert.False(Ed25519.verify message tamperedR publicKey)
        Assert.False(Ed25519.verify message tamperedS publicKey)

    [<Fact>]
    let ``verification rejects malformed scalars points and lengths`` () =
        let publicKey, secretKey = Ed25519.generateKeypair (Array.zeroCreate 32)
        let message = Text.Encoding.UTF8.GetBytes "hello"
        let signature = Ed25519.sign message secretKey
        let outOfRangeS = Array.copy signature

        Array.Copy(
            hex "edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010",
            0,
            outOfRangeS,
            32,
            32)

        let invalidR = Array.copy signature
        Array.Copy(Array.create 32 0xffuy, invalidR, 32)
        let negativeZeroR = Array.copy signature
        Array.Clear(negativeZeroR, 0, 32)
        negativeZeroR[0] <- 1uy
        negativeZeroR[31] <- 0x80uy

        Assert.False(Ed25519.verify message outOfRangeS publicKey)
        Assert.False(Ed25519.verify message invalidR publicKey)
        Assert.False(Ed25519.verify message negativeZeroR publicKey)
        Assert.False(Ed25519.verify message signature (Array.create 32 0xffuy))
        Assert.False(Ed25519.verify message null publicKey)
        Assert.False(Ed25519.verify message (Array.zeroCreate 63) publicKey)
        Assert.False(Ed25519.verify message signature null)
        Assert.False(Ed25519.verify message signature (Array.zeroCreate 31))

    [<Fact>]
    let ``public inputs are validated and secret key must match its seed`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Ed25519.generateKeypair null |> ignore)
        |> ignore

        Assert.Throws<ArgumentException>(fun () -> Ed25519.generateKeypair (Array.zeroCreate 31) |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () -> Ed25519.sign null (Array.zeroCreate 64) |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () -> Ed25519.sign [||] null |> ignore)
        |> ignore

        Assert.Throws<ArgumentException>(fun () -> Ed25519.sign [||] (Array.zeroCreate 63) |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () -> Ed25519.verify null (Array.zeroCreate 64) (Array.zeroCreate 32) |> ignore)
        |> ignore

        let _, secretKey = Ed25519.generateKeypair (Array.zeroCreate 32)
        secretKey[63] <- secretKey[63] ^^^ 1uy

        Assert.Throws<ArgumentException>(fun () -> Ed25519.sign [||] secretKey |> ignore)
        |> ignore
