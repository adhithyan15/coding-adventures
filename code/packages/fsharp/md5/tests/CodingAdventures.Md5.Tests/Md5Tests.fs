namespace CodingAdventures.Md5.Tests

open System
open System.Linq
open System.Text
open Xunit
open CodingAdventures.Md5

type Md5Tests() =
    let encode (value: string) = Encoding.UTF8.GetBytes(value)
    let hex value = Md5.hexString (encode value)

    [<Fact>]
    member _.``version and hex utilities match expected output``() =
        Assert.Equal("0.1.0", Md5.VERSION)
        Assert.Equal(String.Empty, Md5.toHex [||])
        Assert.Equal("00", Md5.toHex [| 0x00uy |])
        Assert.Equal("ff", Md5.toHex [| 0xffuy |])
        Assert.Equal("d41d8cd9", Md5.toHex [| 0xd4uy; 0x1duy; 0x8cuy; 0xd9uy |])

    [<Fact>]
    member _.``rfc1321 vectors all match``() =
        Assert.Equal("d41d8cd98f00b204e9800998ecf8427e", hex "")
        Assert.Equal("0cc175b9c0f1b6a831c399e269772661", hex "a")
        Assert.Equal("900150983cd24fb0d6963f7d28e17f72", hex "abc")
        Assert.Equal("f96b697d7cb7938d525a2f31aaf161d0", hex "message digest")
        Assert.Equal("c3fcd3d76192e4007dfb496cca67e13b", hex "abcdefghijklmnopqrstuvwxyz")

        Assert.Equal(
            "d174ab98d277d9f5a5611c2c9f419d9f",
            hex "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
        )

        Assert.Equal(
            "57edf4a22be3c955ac49da2e2107b67a",
            hex "12345678901234567890123456789012345678901234567890123456789012345678901234567890"
        )

    [<Fact>]
    member _.``known digests and little-endian checks match``() =
        Assert.Equal("9e107d9d372bb6826bd81d3542a419d6", hex "The quick brown fox jumps over the lazy dog")
        Assert.Equal("e4d909c290d0fb1ca068ffaddf22cbd0", hex "The quick brown fox jumps over the lazy dog.")
        Assert.Equal("93b885adfe0da089cdf634904fd59f71", Md5.hexString [| 0x00uy |])
        Assert.Equal("00594fd4f42ba43fc1ca0427a0576295", Md5.hexString [| 0xffuy |])

        let emptyDigest = Md5.sumMd5 [||]
        Assert.Equal(16, emptyDigest.Length)
        Assert.Equal(0xd4uy, emptyDigest[0])
        Assert.Equal(0x1duy, emptyDigest[1])

    [<Fact>]
    member _.``one-shot digest always returns sixteen bytes and lowercase hex``() =
        Assert.Equal(16, (Md5.sumMd5 (encode "")).Length)
        Assert.Equal(16, (Md5.sumMd5 (encode "abc")).Length)
        Assert.Equal(16, (Md5.sumMd5 (Array.zeroCreate<byte> 1000)).Length)

        let digestHex = Md5.hexString (encode "hello world")
        Assert.Equal(32, digestHex.Length)
        Assert.Matches("^[0-9a-f]{32}$", digestHex)
        Assert.Equal(digestHex, Md5.toHex (Md5.sumMd5 (encode "hello world")))

    [<Fact>]
    member _.``block boundary lengths remain stable``() =
        for length in [| 55; 56; 63; 64; 65; 128 |] do
            let data = Array.create length (byte 'a')
            let oneShot = Md5.hexString data

            let hasher = Md5.Md5Hasher()
            let split = min 13 data.Length
            hasher.Update(data[.. split - 1]) |> ignore
            hasher.Update(data[split..]) |> ignore

            Assert.Equal(oneShot, hasher.HexDigest())

    [<Fact>]
    member _.``streaming hasher matches one-shot across arbitrary chunks``() =
        let data = Enumerable.Range(0, 256).Select(byte).ToArray()
        let expected = Md5.hexString data

        let hasher = Md5.Md5Hasher()
        hasher.Update(data[..6]) |> ignore
        hasher.Update(data[7..110]) |> ignore
        hasher.Update(data[111..191]) |> ignore
        hasher.Update(data[192..]) |> ignore

        Assert.Equal(expected, hasher.HexDigest())
        Assert.Equal(expected, Md5.toHex (hasher.Digest()))

    [<Fact>]
    member _.``digest is non-destructive and hasher can continue after digesting``() =
        let hasher = Md5.Md5Hasher()
        hasher.Update(encode "hello") |> ignore

        let first = hasher.HexDigest()
        Assert.Equal(first, hasher.HexDigest())
        Assert.Equal(Md5.hexString (encode "hello"), first)

        hasher.Update(encode " world") |> ignore
        Assert.Equal(Md5.hexString (encode "hello world"), hasher.HexDigest())

    [<Fact>]
    member _.``copy creates independent streaming states``() =
        let original = Md5.Md5Hasher()
        original.Update(encode "ab") |> ignore

        let copy = original.Copy()
        copy.Update(encode "c") |> ignore
        original.Update(encode "x") |> ignore

        Assert.Equal(Md5.hexString (encode "abc"), copy.HexDigest())
        Assert.Equal(Md5.hexString (encode "abx"), original.HexDigest())
