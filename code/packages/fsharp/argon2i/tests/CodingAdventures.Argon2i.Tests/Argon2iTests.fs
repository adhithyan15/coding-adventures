namespace CodingAdventures.Argon2i.Tests

open System
open System.Text
open Xunit
open CodingAdventures.Argon2i.FSharp

module Argon2iTests =
    let private rfcPassword = Array.create 32 0x01uy
    let private rfcSalt = Array.create 16 0x02uy

    [<Fact>]
    let ``matches RFC 9106 vector`` () =
        let options =
            {
                Argon2iOptions.Default with
                    Key = Array.create 8 0x03uy
                    AssociatedData = Array.create 12 0x04uy
            }

        let tag = Argon2i.derive rfcPassword rfcSalt 3 32 4 32 options

        Assert.Equal(
            "c814d9d1dc7f37aa13f0d77f2494bda1c8de6b016dd388d29952a4c4672b6ce8",
            Convert.ToHexString(tag).ToLowerInvariant()
        )

    [<Fact>]
    let ``hex matches byte form`` () =
        let tag = Argon2i.deriveDefault rfcPassword rfcSalt 3 32 4 32
        let expected = Convert.ToHexString(tag).ToLowerInvariant()
        Assert.Equal(expected, Argon2i.deriveHexDefault rfcPassword rfcSalt 3 32 4 32)

    [<Theory>]
    [<InlineData(4)>]
    [<InlineData(16)>]
    [<InlineData(32)>]
    [<InlineData(64)>]
    [<InlineData(65)>]
    [<InlineData(128)>]
    let ``supports variable tag lengths`` tagLength =
        let tag = Argon2i.deriveDefault "password"B "saltsalt"B 1 8 1 tagLength
        Assert.Equal(tagLength, tag.Length)

    [<Fact>]
    let ``secret inputs bind the output`` () =
        let baseline = Argon2i.deriveDefault "password"B "saltsalt"B 1 8 1 32

        let withKey =
            Argon2i.derive
                "password"B
                "saltsalt"B
                1
                8
                1
                32
                { Argon2iOptions.Default with Key = "secret!!"B }

        let withAssociatedData =
            Argon2i.derive
                "password"B
                "saltsalt"B
                1
                8
                1
                32
                { Argon2iOptions.Default with AssociatedData = "context"B }

        Assert.NotEqual<byte array>(baseline, withKey)
        Assert.NotEqual<byte array>(baseline, withAssociatedData)

    [<Fact>]
    let ``password salt and pass count change output`` () =
        let baseline = Argon2i.deriveDefault "password"B "saltsalt"B 1 8 1 32
        Assert.NotEqual<byte array>(baseline, Argon2i.deriveDefault "password2"B "saltsalt"B 1 8 1 32)
        Assert.NotEqual<byte array>(baseline, Argon2i.deriveDefault "password"B "saltsal2"B 1 8 1 32)
        Assert.NotEqual<byte array>(baseline, Argon2i.deriveDefault "password"B "saltsalt"B 2 8 1 32)

    [<Fact>]
    let ``repeats deterministically across address block boundary`` () =
        let first = Argon2i.deriveDefault "password"B "saltsalt"B 1 520 1 16
        let second = Argon2i.deriveDefault "password"B "saltsalt"B 1 520 1 16
        Assert.Equal<byte array>(first, second)

    [<Fact>]
    let ``rejects null inputs`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Argon2i.deriveDefault null "saltsalt"B 1 8 1 32 |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () -> Argon2i.deriveDefault [||] null 1 8 1 32 |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () -> Argon2i.derive [||] "saltsalt"B 1 8 1 32 Unchecked.defaultof<_> |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () ->
            Argon2i.derive [||] "saltsalt"B 1 8 1 32 { Argon2iOptions.Default with Key = null }
            |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () ->
            Argon2i.derive [||] "saltsalt"B 1 8 1 32 { Argon2iOptions.Default with AssociatedData = null }
            |> ignore)
        |> ignore

    [<Fact>]
    let ``rejects invalid parameters`` () =
        let rejects action = Assert.Throws<ArgumentException>(Action action) |> ignore

        rejects (fun () -> Argon2i.deriveDefault [||] "short"B 1 8 1 32 |> ignore)
        rejects (fun () -> Argon2i.deriveDefault [||] "saltsalt"B 1 8 1 3 |> ignore)
        rejects (fun () -> Argon2i.deriveDefault [||] "saltsalt"B 1 8 0 32 |> ignore)
        rejects (fun () -> Argon2i.deriveDefault [||] "saltsalt"B 1 8 0x0100_0000 32 |> ignore)
        rejects (fun () -> Argon2i.deriveDefault [||] "saltsalt"B 1 7 1 32 |> ignore)
        rejects (fun () -> Argon2i.deriveDefault [||] "saltsalt"B 0 8 1 32 |> ignore)

        rejects (fun () ->
            Argon2i.derive [||] "saltsalt"B 1 8 1 32 { Argon2iOptions.Default with Version = 0x10u }
            |> ignore)
