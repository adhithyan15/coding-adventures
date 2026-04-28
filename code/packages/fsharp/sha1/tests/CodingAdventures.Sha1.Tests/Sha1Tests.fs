namespace CodingAdventures.Sha1.Tests

open System
open System.Text
open Xunit
open CodingAdventures.Sha1.FSharp

module Sha1Tests =
    [<Theory>]
    [<InlineData("", "da39a3ee5e6b4b0d3255bfef95601890afd80709")>]
    [<InlineData("abc", "a9993e364706816aba3e25717850c26c9cd0d89d")>]
    [<InlineData("abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq", "84983e441c3bd26ebaae4aa1f95129e5e54670f1")>]
    let ``hash matches FIPS vectors`` (input: string) (expected: string) =
        let data = Encoding.ASCII.GetBytes input

        Assert.Equal(Sha1.DigestLength, (Sha1.hash data).Length)
        Assert.Equal(expected, Sha1.hashHex data)

    [<Fact>]
    let ``hash handles large input`` () =
        let data = Array.create 1_000_000 (byte 'a')

        Assert.Equal("34aa973cd4c4daa4f61eeb2bdbad27316534016f", Sha1.hashHex data)

    [<Fact>]
    let ``hex digest is lowercase`` () =
        let hex = Sha1.hashHex (Encoding.ASCII.GetBytes "abc")

        Assert.Equal(40, hex.Length)
        Assert.Equal(hex.ToLowerInvariant(), hex)

    [<Fact>]
    let ``hash rejects null`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Sha1.hash null |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Sha1.hashHex null |> ignore) |> ignore

    [<Fact>]
    let ``streaming matches one shot across chunking`` () =
        let data = [| 0 .. 199 |] |> Array.map byte

        for chunkSize in [ 1; 7; 13; 32; 63; 64; 65; 100; 200 ] do
            let hasher = Sha1Hasher()
            for offset in 0 .. chunkSize .. data.Length - 1 do
                data.[offset .. min (offset + chunkSize - 1) (data.Length - 1)]
                |> hasher.Update
                |> ignore

            Assert.Equal<byte array>(Sha1.hash data, hasher.Digest())

    [<Fact>]
    let ``streaming digest is nondestructive and chainable`` () =
        let hasher = Sha1Hasher()

        let result =
            hasher
                .Update("a"B)
                .Update("b"B)
                .Update("c"B)

        Assert.Same(hasher, result)
        Assert.Equal<byte array>(hasher.Digest(), hasher.Digest())
        Assert.Equal(Sha1.hashHex "abc"B, hasher.HexDigest())

    [<Fact>]
    let ``streaming can continue after digest`` () =
        let hasher = Sha1Hasher()

        hasher.Update("ab"B) |> ignore
        hasher.Digest() |> ignore
        hasher.Update("c"B) |> ignore

        Assert.Equal<byte array>(Sha1.hash "abc"B, hasher.Digest())

    [<Fact>]
    let ``copy is independent`` () =
        let baseHasher = Sha1Hasher()
        baseHasher.Update("ab"B) |> ignore

        let copy = baseHasher.Copy()
        copy.Update("c"B) |> ignore
        baseHasher.Update("x"B) |> ignore

        Assert.Equal<byte array>(Sha1.hash "abc"B, copy.Digest())
        Assert.Equal<byte array>(Sha1.hash "abx"B, baseHasher.Digest())

    [<Fact>]
    let ``update rejects null`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Sha1Hasher().Update(null) |> ignore) |> ignore
