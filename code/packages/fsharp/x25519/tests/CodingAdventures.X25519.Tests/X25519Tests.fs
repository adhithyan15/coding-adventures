namespace CodingAdventures.X25519.FSharp.Tests

open System
open Xunit
open CodingAdventures.X25519.FSharp

module X25519Tests =
    let private hex (value: string) = Convert.FromHexString value

    [<Theory>]
    [<InlineData(
        "a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4",
        "e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c",
        "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552")>]
    [<InlineData(
        "4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d",
        "e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493",
        "95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957")>]
    let ``x25519 matches RFC 7748 vectors`` scalar u expected =
        Assert.Equal<byte array>(hex expected, X25519.x25519 (hex scalar) (hex u))

    [<Fact>]
    let ``base multiplication matches Alice and Bob public keys`` () =
        Assert.Equal<byte array>(
            hex "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a",
            X25519.x25519Base (hex "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a"))

        Assert.Equal<byte array>(
            hex "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f",
            X25519.x25519Base (hex "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb"))

    [<Fact>]
    let ``both parties derive the RFC shared secret`` () =
        let alicePrivate = hex "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a"
        let bobPrivate = hex "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb"
        let alicePublic = X25519.x25519Base alicePrivate
        let bobPublic = X25519.x25519Base bobPrivate
        let expected = hex "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742"

        Assert.Equal<byte array>(expected, X25519.x25519 alicePrivate bobPublic)
        Assert.Equal<byte array>(expected, X25519.x25519 bobPrivate alicePublic)

    [<Fact>]
    let ``generateKeypair aliases base multiplication`` () =
        let privateKey = [| 0uy .. 31uy |]
        Assert.Equal<byte array>(X25519.x25519Base privateKey, X25519.generateKeypair privateKey)

    [<Fact>]
    let ``iterated RFC vector matches after one thousand rounds`` () =
        let mutable scalar = Array.zeroCreate<byte> 32
        let mutable u = Array.zeroCreate<byte> 32
        scalar[0] <- 9uy
        u[0] <- 9uy

        for _ = 1 to 1_000 do
            let next = X25519.x25519 scalar u
            u <- scalar
            scalar <- next

        Assert.Equal<byte array>(
            hex "684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51",
            scalar)

    [<Fact>]
    let ``u-coordinate high bit is masked`` () =
        let scalar = Array.create 32 0x42uy
        let canonical = X25519.basePoint ()
        let highBitSet = Array.copy canonical
        highBitSet[31] <- 0x80uy

        Assert.Equal<byte array>(
            X25519.x25519 scalar canonical,
            X25519.x25519 scalar highBitSet)

    [<Fact>]
    let ``null and wrong length inputs are rejected`` () =
        Assert.Throws<ArgumentNullException>(fun () -> X25519.x25519 null (Array.zeroCreate 32) |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () -> X25519.x25519 (Array.zeroCreate 32) null |> ignore)
        |> ignore

        Assert.Throws<ArgumentException>(fun () -> X25519.x25519 (Array.zeroCreate 31) (Array.zeroCreate 32) |> ignore)
        |> ignore

        Assert.Throws<ArgumentException>(fun () -> X25519.x25519 (Array.zeroCreate 32) (Array.zeroCreate 33) |> ignore)
        |> ignore

    [<Fact>]
    let ``low-order all-zero output is rejected`` () =
        Assert.Throws<InvalidOperationException>(fun () ->
            X25519.x25519 (Array.create 32 0x11uy) (Array.zeroCreate 32) |> ignore)
        |> ignore

    [<Fact>]
    let ``basePoint returns an independent copy`` () =
        let first = X25519.basePoint ()
        first[0] <- 0uy
        let second = X25519.basePoint ()

        Assert.Equal(32, second.Length)
        Assert.Equal(9uy, second[0])
        Assert.All(second[1..], fun value -> Assert.Equal(0uy, value))
