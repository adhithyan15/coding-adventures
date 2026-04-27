namespace CodingAdventures.Blake2b.Tests

open System
open System.Text
open Xunit
open CodingAdventures.Blake2b.FSharp

module Blake2bTests =
    let private bytesFromRange startInclusive endExclusive =
        [| startInclusive .. endExclusive - 1 |]
        |> Array.map (fun value -> byte (value &&& 0xff))

    [<Theory>]
    [<InlineData("", "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce")>]
    [<InlineData("abc", "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d17d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923")>]
    [<InlineData("The quick brown fox jumps over the lazy dog", "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918")>]
    let ``hash matches canonical vectors`` (input: string) (expected: string) =
        let data = Encoding.ASCII.GetBytes input

        Assert.Equal(Blake2b.MaxDigestLength, (Blake2b.hash data).Length)
        Assert.Equal(expected, Blake2b.hashHex data)

    [<Fact>]
    let ``truncated digest matches vector`` () =
        let options = Blake2bOptions.Default.WithDigestSize(32)

        Assert.Equal(
            "0e5751c026e543b2e8ab2eb06099daa1d1e5df47778f7787faab45cdf12fe3a8",
            Blake2b.hashHexWithOptions options [||])

    [<Fact>]
    let ``keyed long vector matches reference`` () =
        let key = bytesFromRange 1 65
        let data = bytesFromRange 0 256

        Assert.Equal(
            "402fa70e35f026c9bfc1202805e931b995647fe479e1701ad8b7203cddad5927ee7950b898a5a8229443d93963e4f6f27136b2b56f6845ab18f59bc130db8bf3",
            Blake2b.hashHexWithOptions (Blake2bOptions.Default.WithKey key) data)

    [<Theory>]
    [<InlineData(0, "786a02f742015903c6c6fd852552d272912f4740e15847618a86e217f71f5419d25e1031afee585313896444934eb04b903a685b1448b755d56f701afe9be2ce")>]
    [<InlineData(1, "4fe4da61bcc756071b226843361d74944c72245d23e8245ea678c13fdcd7fe2ae529cf999ad99cc24f7a73416a18ba53e76c0afef83b16a568b12fbfc1a2674d")>]
    [<InlineData(63, "70b2a0e6daecac22c7a2df82c06e3fc0b4c66bd5ef8098e4ed54e723b393d79ef3bceba079a01a14c6ef2ae2ed1171df1662cd14ef38e6f77b01c7f48144dd09")>]
    [<InlineData(64, "3db7bb5c40745f0c975ac6bb8578f590e2cd2cc1fc6d13533ef725325c9fddff5cca24e7a591a0f6032a24fad0e09f6df873c4ff314628391f78df7f09cb7ed7")>]
    [<InlineData(65, "149c114a3e8c6e06bafee27c9d0de0e39ef28294fa0d9f81876dcceb10bb41101e256593587e46b844819ed7ded90d56c0843df06c95d1695c3de635cd7a888e")>]
    [<InlineData(127, "71546bbf9110ad184cc60f2eb120fcfd9b4dbbca7a7f1270045b8a23a6a4f4330f65c1f030dd2f5fabc6c57617242c37cf427bd90407fac5b9deffd3ae888c39")>]
    [<InlineData(128, "2d9e329f42afa3601d646692b81c13e87fcaff5bf15972e9813d7373cb6d181f9599f4d513d4af4fd6ebd37497aceb29aba5ee23ed764d8510b552bd088814fb")>]
    [<InlineData(129, "47889df9eb4d717afc5019df5c6a83df00a0b8677395e078cd5778ace0f338a618e68b7d9afb065d9e6a01ccd31d109447e7fae771c3ee3e105709194122ba2b")>]
    [<InlineData(255, "1a5199ac66a00e8a87ad1c7fbad30b33137dd8312bf6d98602dacf8f40ea2cb623a7fbc63e5a6bfa434d337ae7da5ca1a52502a215a3fe0297a151be85d88789")>]
    [<InlineData(256, "91019c558584980249ca43eceed27e19f1c3c24161b93eed1eee2a6a774f60bf8a81b43750870bee1698feac9c5336ae4d5c842e7ead159bf3916387e8ded9ae")>]
    [<InlineData(257, "9f1975efca45e7b74b020975d4d2c22802906ed8bfefca51ac497bd23147fc8f303890d8e5471ab6caaa02362e831a9e8d3435279912ccd4842c7806b096c348")>]
    [<InlineData(1024, "eddc3f3af9392eff065b359ce5f2b28f71e9f3a3a50e60ec27787b9fa623094d17b046c1dfce89bc5cdfc951b95a9a9c05fb8cc2361c905db01dd237fe56efb3")>]
    [<InlineData(4096, "31404c9c7ed64c59112579f300f2afef181ee6283c3918bf026c4ed4bcde0697a7834f3a3410396622ef3d4f432602528a689498141c184cc2063554ba688dc7")>]
    [<InlineData(9999, "b4a5808e65d7424b517bde11e04075a09b1343148e3ab2c8b13ff35c542e0a2beff6309ecc54b59ac046f6d65a9e3680c6372a033607709c95d5fd8070be6069")>]
    let ``block boundary vectors match reference`` (size: int) (expected: string) =
        let data =
            [| 0 .. size - 1 |]
            |> Array.map (fun index -> byte ((index * 7 + 3) &&& 0xff))

        Assert.Equal(expected, Blake2b.hashHex data)

    [<Theory>]
    [<InlineData(1, "b5")>]
    [<InlineData(16, "249df9a49f517ddcd37f5c897620ec73")>]
    [<InlineData(20, "3c523ed102ab45a37d54f5610d5a983162fde84f")>]
    [<InlineData(32, "01718cec35cd3d796dd00020e0bfecb473ad23457d063b75eff29c0ffa2e58a9")>]
    [<InlineData(48, "b7c81b228b6bd912930e8f0b5387989691c1cee1e65aade4da3b86a3c9f678fc8018f6ed9e2906720c8d2a3aeda9c03d")>]
    [<InlineData(64, "a8add4bdddfd93e4877d2746e62817b116364a1fa7bc148d95090bc7333b3673f82401cf7aa2e4cb1ecd90296e3f14cb5413f8ed77be73045b13914cdcd6a918")>]
    let ``variable digest sizes match reference`` (digestSize: int) (expected: string) =
        let data = Encoding.ASCII.GetBytes "The quick brown fox jumps over the lazy dog"
        let options = Blake2bOptions.Default.WithDigestSize(digestSize)
        let digest = Blake2b.hashWithOptions options data

        Assert.Equal(digestSize, digest.Length)
        Assert.Equal(expected, Convert.ToHexString(digest).ToLowerInvariant())

    [<Theory>]
    [<InlineData(1, "affd4e429aa2fb18da276f6ecff16f7d048769cacefe1a7ac75184448e082422")>]
    [<InlineData(16, "5f8510d05dac42e8b6fc542af93f349d41ae4ebaf5cecae4af43fae54c7ca618")>]
    [<InlineData(32, "88a78036d5890e91b5e3d70ba4738d2be302b76e0857d8ee029dc56dfa04fe67")>]
    [<InlineData(64, "df7eab2ec9135ab8c58f48c288cdc873bac245a7fa46ca9f047cab672bd1eabb")>]
    let ``keyed variants match reference`` (keyLength: int) (expected: string) =
        let key = bytesFromRange 1 (keyLength + 1)
        let options = Blake2bOptions.Default.WithDigestSize(32).WithKey(key)

        Assert.Equal(expected, Blake2b.hashHexWithOptions options "secret message body"B)

    [<Fact>]
    let ``salt and personal match reference`` () =
        let options =
            Blake2bOptions.Default
                .WithSalt(bytesFromRange 0 16)
                .WithPersonal(bytesFromRange 16 32)

        Assert.Equal(
            "a2185d648fc63f3d363871a76360330c9b238af5466a20f94bb64d363289b95da0453438eea300cd6f31521274ec001011fa29e91a603fabf00f2b454e30bf3d",
            Blake2b.hashHexWithOptions options "parameterized hash"B)

    [<Fact>]
    let ``streaming matches one shot across chunking`` () =
        let data = bytesFromRange 0 200
        let options = Blake2bOptions.Default.WithDigestSize(32)
        let hasher = Blake2bHasher(options)

        for value in data do
            hasher.Update([| value |]) |> ignore

        Assert.Equal<byte array>(Blake2b.hashWithOptions options data, hasher.Digest())

    [<Fact>]
    let ``streaming handles exact block then more`` () =
        let data = bytesFromRange 0 132
        let hasher = Blake2bHasher()

        hasher.Update(data[0..127]) |> ignore
        hasher.Update(data[128..]) |> ignore

        Assert.Equal<byte array>(Blake2b.hash data, hasher.Digest())

    [<Fact>]
    let ``digest is nondestructive and update can continue`` () =
        let options = Blake2bOptions.Default.WithDigestSize(32)
        let hasher = Blake2bHasher(options)

        let result = hasher.Update("hello "B)
        let first = hasher.HexDigest()
        let second = hasher.HexDigest()
        hasher.Update("world"B) |> ignore

        Assert.Same(hasher, result)
        Assert.Equal(first, second)
        Assert.Equal(Blake2b.hashHexWithOptions options "hello world"B, hasher.HexDigest())

    [<Fact>]
    let ``copy is independent`` () =
        let hasher = Blake2bHasher()
        hasher.Update("prefix "B) |> ignore

        let copy = hasher.Copy()
        hasher.Update("path A"B) |> ignore
        copy.Update("path B"B) |> ignore

        Assert.Equal<byte array>(Blake2b.hash "prefix path A"B, hasher.Digest())
        Assert.Equal<byte array>(Blake2b.hash "prefix path B"B, copy.Digest())

    [<Fact>]
    let ``rejects invalid options and nulls`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Blake2b.hash null |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Blake2bHasher().Update(null) |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> Blake2b.hashWithOptions (Blake2bOptions.Default.WithDigestSize(0)) [||] |> ignore) |> ignore
        Assert.Throws<ArgumentOutOfRangeException>(fun () -> Blake2b.hashWithOptions (Blake2bOptions.Default.WithDigestSize(65)) [||] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Blake2b.hashWithOptions (Blake2bOptions.Default.WithKey(Array.zeroCreate 65)) [||] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Blake2b.hashWithOptions (Blake2bOptions.Default.WithSalt(Array.zeroCreate 8)) [||] |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Blake2b.hashWithOptions (Blake2bOptions.Default.WithPersonal(Array.zeroCreate 20)) [||] |> ignore) |> ignore

    [<Fact>]
    let ``accepts maximum key length`` () =
        let digest = Blake2b.hashWithOptions (Blake2bOptions.Default.WithKey(Array.create 64 (byte 'k'))) "x"B

        Assert.Equal(Blake2b.MaxDigestLength, digest.Length)
