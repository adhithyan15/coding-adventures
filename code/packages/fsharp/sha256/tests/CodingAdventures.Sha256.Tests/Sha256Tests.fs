namespace CodingAdventures.Sha256.Tests

open System
open System.Text
open Xunit
open CodingAdventures.Sha256.FSharp

module Sha256Tests =
    [<Theory>]
    [<InlineData("", "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")>]
    [<InlineData("abc", "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad")>]
    [<InlineData("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1")>]
    let ``hash matches FIPS vectors`` (input: string) (expected: string) =
        let data = Encoding.ASCII.GetBytes input

        Assert.Equal(Sha256.DigestLength, (Sha256.hash data).Length)
        Assert.Equal(expected, Sha256.hashHex data)

    [<Fact>]
    let ``hash handles large input`` () =
        let data = Array.create 1_000_000 (byte 'a')

        Assert.Equal(
            "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0",
            Sha256.hashHex data)

    [<Fact>]
    let ``hex digest is lowercase`` () =
        let hex = Sha256.hashHex (Encoding.ASCII.GetBytes "abc")

        Assert.Equal(64, hex.Length)
        Assert.Equal(hex.ToLowerInvariant(), hex)

    [<Fact>]
    let ``hash rejects null`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Sha256.hash null |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Sha256.hashHex null |> ignore) |> ignore

    [<Fact>]
    let ``streaming matches one shot across chunking`` () =
        let data = [| 0 .. 199 |] |> Array.map byte

        for chunkSize in [ 1; 7; 13; 32; 63; 64; 65; 100; 200 ] do
            let hasher = Sha256Hasher()
            for offset in 0 .. chunkSize .. data.Length - 1 do
                data.[offset .. min (offset + chunkSize - 1) (data.Length - 1)]
                |> hasher.Update
                |> ignore

            Assert.Equal<byte array>(Sha256.hash data, hasher.Digest())

    [<Fact>]
    let ``streaming digest is nondestructive and chainable`` () =
        let hasher = Sha256Hasher()

        let result =
            hasher
                .Update("a"B)
                .Update("b"B)
                .Update("c"B)

        Assert.Same(hasher, result)
        Assert.Equal<byte array>(hasher.Digest(), hasher.Digest())
        Assert.Equal(Sha256.hashHex "abc"B, hasher.HexDigest())

    [<Fact>]
    let ``streaming can continue after digest`` () =
        let hasher = Sha256Hasher()

        hasher.Update("ab"B) |> ignore
        hasher.Digest() |> ignore
        hasher.Update("c"B) |> ignore

        Assert.Equal<byte array>(Sha256.hash "abc"B, hasher.Digest())

    [<Fact>]
    let ``copy is independent`` () =
        let baseHasher = Sha256Hasher()
        baseHasher.Update("ab"B) |> ignore

        let copy = baseHasher.Copy()
        copy.Update("c"B) |> ignore
        baseHasher.Update("x"B) |> ignore

        Assert.Equal<byte array>(Sha256.hash "abc"B, copy.Digest())
        Assert.Equal<byte array>(Sha256.hash "abx"B, baseHasher.Digest())

    [<Fact>]
    let ``update rejects null`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Sha256Hasher().Update(null) |> ignore) |> ignore
