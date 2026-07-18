namespace CodingAdventures.ChaCha20Poly1305.FSharp.Tests

open System
open System.Security.Cryptography
open System.Text
open Xunit
open CodingAdventures.ChaCha20Poly1305.FSharp

module ChaCha20Poly1305Tests =
    let private hex (value: string) = Convert.FromHexString value

    let private chachaKey =
        hex "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"

    let private chachaNonce = hex "000000000000004a00000000"

    let private plaintext =
        Encoding.ASCII.GetBytes(
            "Ladies and Gentlemen of the class of '99: If I could offer you only one tip for the future, sunscreen would be it.")

    let private chachaCiphertext =
        hex (
            "6e2e359a2568f98041ba0728dd0d6981e97e7aec1d4360c20a27afccfd9fae0b"
            + "f91b65c5524733ab8f593dabcd62b3571639d624e65152ab8f530c359f0861d8"
            + "07ca0dbf500d6a6156a38e088a22b65e52bc514d16ccf806818ce91ab7793736"
            + "5af90bbf74a35be6b40b8eedf2785e42874d"
        )

    let private aeadKey =
        hex "808182838485868788898a8b8c8d8e8f909192939495969798999a9b9c9d9e9f"

    let private aeadNonce = hex "070000004041424344454647"
    let private aeadAad = hex "50515253c0c1c2c3c4c5c6c7"

    let private aeadCiphertext =
        hex (
            "d31a8d34648e60db7b86afbc53ef7ec2a4aded51296e08fea9e2b5a736ee62d6"
            + "3dbea45e8ca9671282fafb69da92728b1a71de0a9e060b2905d6a5b67ecd3b36"
            + "92ddbd7f2d778b8c9803aee328091b58fab324e4fad675945585808b4831d7bc"
            + "3ff4def08e4b7a9de576d26586cec64b6116"
        )

    let private aeadTag = hex "1ae10b594f09e26a7e902ecbd0600691"

    [<Fact>]
    let ``block matches RFC 8439 section 2.3.2`` () =
        let block =
            ChaCha20Poly1305.chacha20Block
                chachaKey
                1u
                (hex "000000090000004a00000000")

        Assert.Equal(64, block.Length)
        Assert.Equal<byte array>(hex "10f1e7e4d13b5915500fdd1fa32071c4", block[..15])

    [<Fact>]
    let ``stream cipher matches RFC 8439 section 2.4.2`` () =
        Assert.Equal<byte array>(
            chachaCiphertext,
            ChaCha20Poly1305.chacha20Encrypt plaintext chachaKey chachaNonce 1u)

    [<Fact>]
    let ``stream cipher is symmetric across multiple blocks`` () =
        let data = Array.init 512 (fun index -> byte (index &&& 0xff))
        let encrypted = ChaCha20Poly1305.chacha20Encrypt data chachaKey chachaNonce 7u

        Assert.Equal<byte array>(
            data,
            ChaCha20Poly1305.chacha20Encrypt encrypted chachaKey chachaNonce 7u)

        Assert.Empty(ChaCha20Poly1305.chacha20Encrypt Array.empty chachaKey chachaNonce 0u)

    [<Fact>]
    let ``Poly1305 matches RFC 8439 section 2.5.2`` () =
        Assert.Equal<byte array>(
            hex "a8061dc1305136c6c22b8baf0c0127a9",
            ChaCha20Poly1305.poly1305Mac
                (Encoding.ASCII.GetBytes("Cryptographic Forum Research Group"))
                (hex "85d6be7857556d337f4452fe42d506a80103808afb0db2fd4abff6af4149f51b"))

    [<Fact>]
    let ``Poly1305 handles empty and partial blocks`` () =
        let key = Array.init 32 byte
        Assert.Equal(16, (ChaCha20Poly1305.poly1305Mac Array.empty key).Length)

        Assert.NotEqual<byte array>(
            ChaCha20Poly1305.poly1305Mac [| 0uy |] key,
            ChaCha20Poly1305.poly1305Mac [| 0uy; 0uy |] key)

    [<Fact>]
    let ``AEAD encrypt matches RFC 8439 section 2.8.2`` () =
        let result = ChaCha20Poly1305.aeadEncrypt plaintext aeadKey aeadNonce aeadAad
        Assert.Equal<byte array>(aeadCiphertext, result.Ciphertext)
        Assert.Equal<byte array>(aeadTag, result.Tag)

    [<Fact>]
    let ``AEAD decrypt matches RFC 8439 section 2.8.2`` () =
        Assert.Equal<byte array>(
            plaintext,
            ChaCha20Poly1305.aeadDecrypt aeadCiphertext aeadKey aeadNonce aeadAad aeadTag)

    [<Fact>]
    let ``AEAD round trips empty and large plaintexts`` () =
        let key = Array.init 32 byte
        let nonce = Array.init 12 byte

        for input in [| Array.empty<byte>; Array.create 1024 0x41uy |] do
            let result = ChaCha20Poly1305.aeadEncrypt input key nonce null

            Assert.Equal<byte array>(
                input,
                ChaCha20Poly1305.aeadDecrypt result.Ciphertext key nonce null result.Tag)

    [<Fact>]
    let ``tampered ciphertext is rejected`` () =
        let result =
            ChaCha20Poly1305.aeadEncrypt
                (Encoding.ASCII.GetBytes("secret"))
                aeadKey
                aeadNonce
                aeadAad

        result.Ciphertext[0] <- result.Ciphertext[0] ^^^ 1uy

        Assert.Throws<CryptographicException>(fun () ->
            ChaCha20Poly1305.aeadDecrypt
                result.Ciphertext aeadKey aeadNonce aeadAad result.Tag
            |> ignore)
        |> ignore

    [<Fact>]
    let ``tampered tag and wrong AAD are rejected`` () =
        let result =
            ChaCha20Poly1305.aeadEncrypt
                (Encoding.ASCII.GetBytes("secret"))
                aeadKey
                aeadNonce
                aeadAad

        let badTag = Array.copy result.Tag
        badTag[badTag.Length - 1] <- badTag[badTag.Length - 1] ^^^ 1uy

        Assert.Throws<CryptographicException>(fun () ->
            ChaCha20Poly1305.aeadDecrypt result.Ciphertext aeadKey aeadNonce aeadAad badTag
            |> ignore)
        |> ignore

        Assert.Throws<CryptographicException>(fun () ->
            ChaCha20Poly1305.aeadDecrypt result.Ciphertext aeadKey aeadNonce [| 1uy; 2uy; 3uy |] result.Tag
            |> ignore)
        |> ignore

    [<Fact>]
    let ``invalid key lengths are rejected`` () =
        Assert.Throws<ArgumentNullException>(fun () ->
            ChaCha20Poly1305.chacha20Encrypt Array.empty null chachaNonce 0u |> ignore)
        |> ignore

        Assert.Throws<ArgumentException>(fun () ->
            ChaCha20Poly1305.chacha20Encrypt Array.empty (Array.zeroCreate 31) chachaNonce 0u
            |> ignore)
        |> ignore

        Assert.Throws<ArgumentException>(fun () ->
            ChaCha20Poly1305.poly1305Mac Array.empty (Array.zeroCreate 33) |> ignore)
        |> ignore

    [<Fact>]
    let ``invalid nonce and tag lengths are rejected`` () =
        Assert.Throws<ArgumentException>(fun () ->
            ChaCha20Poly1305.chacha20Encrypt Array.empty chachaKey (Array.zeroCreate 11) 0u
            |> ignore)
        |> ignore

        Assert.Throws<ArgumentException>(fun () ->
            ChaCha20Poly1305.aeadDecrypt Array.empty chachaKey chachaNonce Array.empty (Array.zeroCreate 15)
            |> ignore)
        |> ignore

    [<Fact>]
    let ``null data inputs are rejected`` () =
        Assert.Throws<ArgumentNullException>(fun () ->
            ChaCha20Poly1305.chacha20Encrypt null chachaKey chachaNonce 0u |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () ->
            ChaCha20Poly1305.poly1305Mac null chachaKey |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () ->
            ChaCha20Poly1305.aeadEncrypt null chachaKey chachaNonce Array.empty |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () ->
            ChaCha20Poly1305.aeadDecrypt null chachaKey chachaNonce Array.empty (Array.zeroCreate 16)
            |> ignore)
        |> ignore
