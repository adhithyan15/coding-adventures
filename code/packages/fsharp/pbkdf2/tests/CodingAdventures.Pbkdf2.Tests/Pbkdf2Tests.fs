namespace CodingAdventures.Pbkdf2.Tests

open System
open Xunit
open CodingAdventures.Pbkdf2.FSharp

module Pbkdf2Tests =
    [<Fact>]
    let ``sha1 matches RFC 6070 vectors`` () =
        Assert.Equal(
            "0c60c80f961f0e71f3a9b524af6012062fe037a6",
            Pbkdf2.pbkdf2HmacSha1Hex "password"B "salt"B 1 20)
        Assert.Equal(
            "4b007901b765489abead49d926f721d065a429c1",
            Pbkdf2.pbkdf2HmacSha1Hex "password"B "salt"B 4096 20)
        Assert.Equal(
            "56fa6aa75548099dcc37d7f03425e0c3",
            Pbkdf2.pbkdf2HmacSha1Hex "pass\000word"B "sa\000lt"B 4096 16)

    [<Fact>]
    let ``sha256 matches known vector and can extend past one block`` () =
        let key = Pbkdf2.pbkdf2HmacSha256 "passwd"B "salt"B 1 64

        Assert.Equal(
            "55ac046e56e3089fec1691c22544b605f94185216dde0465e68b9d57c20dacbc49ca9cccf179b645991664b39d77ef317c71b845b1e30bd509112041d3a19783",
            Convert.ToHexString(key).ToLowerInvariant())

        Assert.Equal<byte array>(
            Pbkdf2.pbkdf2HmacSha256 "password"B "salt"B 1 32,
            (Pbkdf2.pbkdf2HmacSha256 "password"B "salt"B 1 64).[0..31])

    [<Fact>]
    let ``sha512 supports custom key lengths`` () =
        let full = Pbkdf2.pbkdf2HmacSha512 "secret"B "nacl"B 1 64
        let shortKey = Pbkdf2.pbkdf2HmacSha512 "secret"B "nacl"B 1 32

        Assert.Equal(64, full.Length)
        Assert.Equal<byte array>(shortKey, full.[0..31])
        Assert.Equal(128, (Pbkdf2.pbkdf2HmacSha512 "key"B "salt"B 1 128).Length)

    [<Fact>]
    let ``hex helpers match byte output`` () =
        Assert.Equal(
            Convert.ToHexString(Pbkdf2.pbkdf2HmacSha1 "password"B "salt"B 1 20).ToLowerInvariant(),
            Pbkdf2.pbkdf2HmacSha1Hex "password"B "salt"B 1 20)
        Assert.Equal(
            Convert.ToHexString(Pbkdf2.pbkdf2HmacSha256 "passwd"B "salt"B 1 32).ToLowerInvariant(),
            Pbkdf2.pbkdf2HmacSha256Hex "passwd"B "salt"B 1 32)
        Assert.Equal(
            Convert.ToHexString(Pbkdf2.pbkdf2HmacSha512 "secret"B "nacl"B 1 64).ToLowerInvariant(),
            Pbkdf2.pbkdf2HmacSha512Hex "secret"B "nacl"B 1 64)

    [<Fact>]
    let ``validation rejects invalid inputs`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Pbkdf2.pbkdf2HmacSha256 null "salt"B 1 32 |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> Pbkdf2.pbkdf2HmacSha256 "password"B null 1 32 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Pbkdf2.pbkdf2HmacSha256 [||] "salt"B 1 32 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Pbkdf2.pbkdf2HmacSha256 "pw"B "salt"B 0 32 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Pbkdf2.pbkdf2HmacSha256 "pw"B "salt"B 1 0 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Pbkdf2.pbkdf2HmacSha256 "pw"B "salt"B 1 ((1 <<< 20) + 1) |> ignore) |> ignore

    [<Fact>]
    let ``empty salt is allowed and empty password can be explicitly allowed`` () =
        Assert.Equal(32, (Pbkdf2.pbkdf2HmacSha256 "password"B [||] 1 32).Length)
        Assert.Equal(32, (Pbkdf2.pbkdf2HmacSha256AllowEmptyPassword [||] "salt"B 1 32).Length)

    [<Fact>]
    let ``salt password and iterations affect output`` () =
        let baseKey = Pbkdf2.pbkdf2HmacSha256 "password"B "salt"B 1 32

        Assert.NotEqual<byte array>(baseKey, Pbkdf2.pbkdf2HmacSha256 "password"B "salt2"B 1 32)
        Assert.NotEqual<byte array>(baseKey, Pbkdf2.pbkdf2HmacSha256 "password2"B "salt"B 1 32)
        Assert.NotEqual<byte array>(baseKey, Pbkdf2.pbkdf2HmacSha256 "password"B "salt"B 2 32)
