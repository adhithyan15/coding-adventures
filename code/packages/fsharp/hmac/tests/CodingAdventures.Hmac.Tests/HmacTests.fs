namespace CodingAdventures.Hmac.Tests

open System
open System.Text
open Xunit
open CodingAdventures.Hmac.FSharp
open CodingAdventures.Sha256.FSharp
open CodingAdventures.Sha512.FSharp

module HmacTests =
    [<Fact>]
    let ``hmac sha256 matches RFC 4231 vectors`` () =
        Assert.Equal(
            "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
            Hmac.hmacSha256Hex (Array.create 20 0x0buy) "Hi There"B)
        Assert.Equal(
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843",
            Hmac.hmacSha256Hex "Jefe"B (Encoding.ASCII.GetBytes "what do ya want for nothing?"))

    [<Fact>]
    let ``hmac sha512 matches RFC 4231 vectors`` () =
        Assert.Equal(
            "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854",
            Hmac.hmacSha512Hex (Array.create 20 0x0buy) "Hi There"B)
        Assert.Equal(
            "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737",
            Hmac.hmacSha512Hex "Jefe"B (Encoding.ASCII.GetBytes "what do ya want for nothing?"))

    [<Fact>]
    let ``legacy variants match RFC 2202 vectors`` () =
        Assert.Equal("9294727a3638bb1c13f48ef8158bfc9d", Hmac.hmacMd5Hex (Array.create 16 0x0buy) "Hi There"B)
        Assert.Equal("b617318655057264e28bc0b6fb378c8ef146be00", Hmac.hmacSha1Hex (Array.create 20 0x0buy) "Hi There"B)
        Assert.Equal("effcdf6ae5eb2fa2d27416d5f184df9c259a7c79", Hmac.hmacSha1Hex "Jefe"B (Encoding.ASCII.GetBytes "what do ya want for nothing?"))

    [<Fact>]
    let ``return lengths match hash families`` () =
        let key = "key"B
        let message = "message"B

        Assert.Equal(16, (Hmac.hmacMd5 key message).Length)
        Assert.Equal(20, (Hmac.hmacSha1 key message).Length)
        Assert.Equal(32, (Hmac.hmacSha256 key message).Length)
        Assert.Equal(64, (Hmac.hmacSha512 key message).Length)
        Assert.Equal(64, (Hmac.hmacSha256Hex key message).Length)
        Assert.Equal(128, (Hmac.hmacSha512Hex key message).Length)

    [<Fact>]
    let ``generic compute matches named variant`` () =
        let key = Array.create 100 0x01uy
        let message = "msg"B

        Assert.Equal<byte array>(Hmac.hmacSha256 key message, Hmac.compute Sha256.hash 64 key message)
        Assert.Equal<byte array>(Hmac.hmacSha512 key message, Hmac.compute Sha512.hash 128 key message)

    [<Fact>]
    let ``verify uses constant time comparison semantics`` () =
        let tag = Hmac.hmacSha256 "key"B "message"B

        Assert.True(Hmac.verify tag (Array.copy tag))
        Assert.False(Hmac.verify tag (Hmac.hmacSha256 "key2"B "message"B))
        Assert.False(Hmac.verify tag tag.[0 .. tag.Length - 2])

    [<Fact>]
    let ``empty key is rejected but empty message is allowed`` () =
        Assert.Throws<ArgumentException>(fun () -> Hmac.hmacSha256 [||] "message"B |> ignore) |> ignore
        Assert.Equal(32, (Hmac.computeAllowEmptyKey Sha256.hash 64 [||] "message"B).Length)
        Assert.Equal(32, (Hmac.hmacSha256 "key"B [||]).Length)

    [<Fact>]
    let ``null inputs are rejected`` () =
        let nullHash = Unchecked.defaultof<byte array -> byte array>
        Assert.Throws<ArgumentNullException>(fun () -> Hmac.compute nullHash 64 "key"B "message"B |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Hmac.hmacSha256 null "message"B |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Hmac.hmacSha256 "key"B null |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Hmac.verify null [||] |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Hmac.verify [||] null |> ignore) |> ignore

    [<Fact>]
    let ``invalid block size is rejected`` () =
        Assert.Throws<ArgumentException>(fun () -> Hmac.compute Sha256.hash 0 "key"B "message"B |> ignore)
        |> ignore
