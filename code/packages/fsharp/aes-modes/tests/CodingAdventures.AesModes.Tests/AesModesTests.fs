namespace CodingAdventures.AesModes.Tests

open System
open Xunit
open CodingAdventures.AesModes.FSharp

module AesModesTests =
    let nistKey = Convert.FromHexString "2b7e151628aed2a6abf7158809cf4f3c"

    let nistPlaintext =
        Convert.FromHexString(
            "6bc1bee22e409f96e93d7e117393172a"
            + "ae2d8a571e03ac9c9eb76fac45af8e51"
            + "30c81c46a35ce411e5fbc1191a0a52ef"
            + "f69f2445df4f9b17ad2b417be66c3710"
        )

    let utf8 (text: string) = System.Text.Encoding.UTF8.GetBytes text

    [<Fact>]
    let ``pkcs7 pad and unpad round trips`` () =
        let padded = AesModes.pkcs7Pad (utf8 "hello")

        Assert.Equal(16, padded.Length)
        Assert.Equal(11uy, padded.[padded.Length - 1])
        Assert.Equal<byte array>(utf8 "hello", AesModes.pkcs7Unpad padded)
        Assert.Equal(32, (AesModes.pkcs7Pad (Array.zeroCreate<byte> 16)).Length)

    [<Fact>]
    let ``pkcs7 unpad rejects invalid padding`` () =
        Assert.Throws<ArgumentException>(fun () -> AesModes.pkcs7Unpad Array.empty<byte> |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> AesModes.pkcs7Unpad (utf8 "short") |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> AesModes.pkcs7Unpad (Convert.FromHexString "30313233343536373839616263646500") |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> AesModes.pkcs7Unpad (Convert.FromHexString "30313233343536373839616263030203") |> ignore) |> ignore

    [<Fact>]
    let ``xor bytes requires equal lengths`` () =
        let result = AesModes.xorBytes [| 0xffuy; 0x00uy |] [| 0xf0uy; 0xf0uy |]
        Assert.Equal<byte array>([| 0x0fuy; 0xf0uy |], result)
        Assert.Throws<ArgumentException>(fun () -> AesModes.xorBytes [| 1uy |] Array.empty<byte> |> ignore) |> ignore

    [<Fact>]
    let ``ecb matches nist blocks and round trips`` () =
        let ciphertext = AesModes.ecbEncrypt nistPlaintext nistKey

        Assert.Equal("3ad77bb40d7a3660a89ecaf32466ef97", Convert.ToHexString(ciphertext.[0..15]).ToLowerInvariant())
        Assert.Equal("f5d3d58503b9699de785895a96fdbaaf", Convert.ToHexString(ciphertext.[16..31]).ToLowerInvariant())
        Assert.Equal<byte array>(nistPlaintext, AesModes.ecbDecrypt ciphertext nistKey)

    [<Fact>]
    let ``ecb shows identical block leak`` () =
        let plaintext = Array.create 48 (byte 'A')
        let ciphertext = AesModes.ecbEncrypt plaintext nistKey

        Assert.Equal<byte array>(ciphertext.[0..15], ciphertext.[16..31])
        Assert.Equal<byte array>(ciphertext.[16..31], ciphertext.[32..47])
        Assert.Throws<ArgumentException>(fun () -> AesModes.ecbDecrypt (utf8 "short") nistKey |> ignore) |> ignore

    [<Fact>]
    let ``cbc matches nist blocks and round trips`` () =
        let iv = Convert.FromHexString "000102030405060708090a0b0c0d0e0f"
        let ciphertext = AesModes.cbcEncrypt nistPlaintext nistKey iv

        Assert.Equal("7649abac8119b246cee98e9b12e9197d", Convert.ToHexString(ciphertext.[0..15]).ToLowerInvariant())
        Assert.Equal("5086cb9b507219ee95db113a917678b2", Convert.ToHexString(ciphertext.[16..31]).ToLowerInvariant())
        Assert.Equal<byte array>(nistPlaintext, AesModes.cbcDecrypt ciphertext nistKey iv)

    [<Fact>]
    let ``cbc rejects invalid inputs`` () =
        Assert.Throws<ArgumentException>(fun () -> AesModes.cbcEncrypt (utf8 "test") nistKey (utf8 "short") |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> AesModes.cbcDecrypt (utf8 "short") nistKey (Array.zeroCreate<byte> 16) |> ignore) |> ignore

    [<Fact>]
    let ``ctr round trips without padding`` () =
        let nonce = Array.zeroCreate<byte> 12
        let plaintext = utf8 "CTR keeps the same length."
        let ciphertext = AesModes.ctrEncrypt plaintext nistKey nonce

        Assert.Equal(plaintext.Length, ciphertext.Length)
        Assert.Equal<byte array>(plaintext, AesModes.ctrDecrypt ciphertext nistKey nonce)
        Assert.Empty(AesModes.ctrEncrypt Array.empty<byte> nistKey nonce)
        Assert.Throws<ArgumentException>(fun () -> AesModes.ctrEncrypt plaintext nistKey (Array.zeroCreate<byte> 16) |> ignore) |> ignore

    [<Fact>]
    let ``gcm matches nist test case and round trips with aad`` () =
        let key = Convert.FromHexString "feffe9928665731c6d6a8f9467308308"
        let iv = Convert.FromHexString "cafebabefacedbaddecaf888"

        let plaintext =
            Convert.FromHexString(
                "d9313225f88406e5a55909c5aff5269a"
                + "86a7a9531534f7da2e4c303d8a318a72"
                + "1c3c0c95956809532fcf0e2449a6b525"
                + "b16aedf5aa0de657ba637b391aafd255"
            )

        let expectedCiphertext =
            Convert.FromHexString(
                "42831ec2217774244b7221b784d0d49c"
                + "e3aa212f2c02a4e035c17e2329aca12e"
                + "21d514b25466931c7d8f6a5aac84aa05"
                + "1ba30b396a0aac973d58e091473f5985"
            )

        let expectedTag = Convert.FromHexString "4d5c2af327cd64a62cf35abd2ba6fab4"
        let ciphertext, tag = AesModes.gcmEncrypt plaintext key iv null

        Assert.Equal<byte array>(expectedCiphertext, ciphertext)
        Assert.Equal<byte array>(expectedTag, tag)
        Assert.Equal<byte array>(plaintext, AesModes.gcmDecrypt ciphertext key iv null tag)

        let aad = utf8 "metadata"
        let roundTripCiphertext, roundTripTag = AesModes.gcmEncrypt (utf8 "secret") key iv aad
        Assert.Equal<byte array>(utf8 "secret", AesModes.gcmDecrypt roundTripCiphertext key iv aad roundTripTag)

    [<Fact>]
    let ``gcm rejects tampering and invalid lengths`` () =
        let key = Convert.FromHexString "feffe9928665731c6d6a8f9467308308"
        let iv = Convert.FromHexString "cafebabefacedbaddecaf888"
        let ciphertext, tag = AesModes.gcmEncrypt (utf8 "secret") key iv null

        ciphertext.[0] <- ciphertext.[0] ^^^ 1uy
        Assert.Throws<InvalidOperationException>(fun () -> AesModes.gcmDecrypt ciphertext key iv null tag |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> AesModes.gcmEncrypt (utf8 "test") key (Array.zeroCreate<byte> 16) null |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> AesModes.gcmDecrypt Array.empty<byte> key iv null (Array.zeroCreate<byte> 8) |> ignore) |> ignore
