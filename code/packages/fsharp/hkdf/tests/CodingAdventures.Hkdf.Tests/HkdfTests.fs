namespace CodingAdventures.Hkdf.Tests

open System
open Xunit
open CodingAdventures.Hkdf.FSharp

module HkdfTests =
    [<Fact>]
    let ``sha256 matches RFC 5869 test case 1`` () =
        let ikm = Array.create 22 0x0buy
        let salt = Convert.FromHexString "000102030405060708090a0b0c"
        let info = Convert.FromHexString "f0f1f2f3f4f5f6f7f8f9"

        Assert.Equal(
            "077709362c2e32df0ddc3f0dc47bba6390b6c73bb50f9c3122ec844ad7c2b3e5",
            Convert.ToHexString(Hkdf.extractSha256 salt ikm).ToLowerInvariant())
        Assert.Equal(
            "3cb25f25faacd57a90434f64d0362f2a2d2d0a90cf1a5a4c5db02d56ecc4c5bf34007208d5b887185865",
            Convert.ToHexString(Hkdf.deriveSha256 salt ikm info 42).ToLowerInvariant())

    [<Fact>]
    let ``sha256 empty salt matches RFC 5869 test case 3`` () =
        let ikm = Array.create 22 0x0buy

        Assert.Equal(
            "19ef24a32c717b167f33a91d6f648bdf96596776afdb6377ac434c1c293ccb04",
            Convert.ToHexString(Hkdf.extract [||] ikm Sha256).ToLowerInvariant())
        Assert.Equal(
            "8da4e775a563c18f715f802a063c5a31b8a11f5c5ee1879ec3454e5f3c738d2d9d201395faa4b61a96c8",
            Convert.ToHexString(Hkdf.derive [||] ikm [||] 42 Sha256).ToLowerInvariant())

    [<Fact>]
    let ``expand supports bounds and info domain separation`` () =
        let prk = Array.create 32 0x01uy

        Assert.Equal(1, (Hkdf.expandSha256 prk [||] 1).Length)
        Assert.Equal(255 * 32, (Hkdf.expandSha256 prk [||] (255 * 32)).Length)
        Assert.NotEqual<byte array>(
            Hkdf.expandSha256 prk "purpose-a"B 32,
            Hkdf.expandSha256 prk "purpose-b"B 32)

    [<Fact>]
    let ``sha512 variant uses larger digest and bounds`` () =
        let ikm = Array.create 22 0x0buy
        let prk = Hkdf.extractSha512 [||] ikm

        Assert.Equal(64, prk.Length)
        Assert.Equal(64, (Hkdf.expandSha512 prk "info"B 64).Length)
        Assert.Equal(255 * 64, (Hkdf.expandSha512 (Array.create 64 0x01uy) [||] (255 * 64)).Length)

    [<Fact>]
    let ``derive equals manual extract then expand and default sha256 helpers`` () =
        let salt = "salt"B
        let ikm = "input keying material"B
        let info = "context"B

        let combined = Hkdf.derive salt ikm info 42 Sha256
        let manual = Hkdf.expand (Hkdf.extract salt ikm Sha256) info 42 Sha256
        Assert.Equal<byte array>(manual, combined)
        Assert.Equal<byte array>(Hkdf.extractSha256 salt ikm, Hkdf.extract salt ikm Sha256)

    [<Fact>]
    let ``validation rejects invalid inputs`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Hkdf.extract null "ikm"B Sha256 |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Hkdf.extract [||] null Sha256 |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Hkdf.expand null [||] 1 Sha256 |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Hkdf.expand [||] null 1 Sha256 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Hkdf.expand [||] [||] 0 Sha256 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Hkdf.expand (Array.create 32 0x01uy) [||] (255 * 32 + 1) Sha256 |> ignore) |> ignore
