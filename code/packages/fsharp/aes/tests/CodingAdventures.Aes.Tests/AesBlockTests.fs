namespace CodingAdventures.Aes.Tests

open System
open Xunit
open CodingAdventures.Aes.FSharp

module AesBlockTests =
    [<Theory>]
    [<InlineData("2b7e151628aed2a6abf7158809cf4f3c", "3243f6a8885a308d313198a2e0370734", "3925841d02dc09fbdc118597196a0b32")>]
    [<InlineData("000102030405060708090a0b0c0d0e0f", "00112233445566778899aabbccddeeff", "69c4e0d86a7b0430d8cdb78070b4c55a")>]
    [<InlineData("000102030405060708090a0b0c0d0e0f1011121314151617", "00112233445566778899aabbccddeeff", "dda97ca4864cdfe06eaf70a0ec0d7191")>]
    [<InlineData("603deb1015ca71be2b73aef0857d77811f352c073b6108d72d9810a30914dff4", "6bc1bee22e409f96e93d7e117393172a", "f3eed1bdb5d2a03c064b5a7e3db181f8")>]
    [<InlineData("000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f", "00112233445566778899aabbccddeeff", "8ea2b7ca516745bfeafc49904b496089")>]
    let ``encrypt and decrypt match known vectors`` (keyHex: string) (plainHex: string) (cipherHex: string) =
        let key = Convert.FromHexString keyHex
        let plaintext = Convert.FromHexString plainHex
        let ciphertext = Convert.FromHexString cipherHex

        Assert.Equal<byte array>(ciphertext, AesBlock.encryptBlock plaintext key)
        Assert.Equal<byte array>(plaintext, AesBlock.decryptBlock ciphertext key)

    [<Fact>]
    let ``round trips across key sizes and inputs`` () =
        for keyLength in [ 16; 24; 32 ] do
            let key = [| 0 .. keyLength - 1 |] |> Array.map byte
            let plaintext = [| 0 .. 15 |] |> Array.map (fun value -> byte (255 - value))

            Assert.Equal<byte array>(plaintext, AesBlock.decryptBlock (AesBlock.encryptBlock plaintext key) key)

    [<Fact>]
    let ``validation rejects invalid inputs`` () =
        let key = Array.zeroCreate<byte> 16
        let block = Array.zeroCreate<byte> 16

        Assert.Throws<ArgumentNullException>(fun () -> AesBlock.encryptBlock null key |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> AesBlock.decryptBlock block null |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> AesBlock.encryptBlock (Array.zeroCreate 15) key |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> AesBlock.decryptBlock (Array.zeroCreate 17) key |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> AesBlock.encryptBlock block (Array.zeroCreate 10) |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> AesBlock.decryptBlock block (Array.zeroCreate 20) |> ignore) |> ignore
