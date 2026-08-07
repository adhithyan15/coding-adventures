namespace CodingAdventures.Argon2id.Tests

open System
open System.Text
open Xunit
open CodingAdventures.Argon2id.FSharp

module Argon2idTests =
    let private rfcPassword = Array.create 32 0x01uy
    let private rfcSalt = Array.create 16 0x02uy

    [<Fact>]
    let ``matches RFC 9106 vector`` () =
        let options =
            {
                Argon2idOptions.Default with
                    Key = Array.create 8 0x03uy
                    AssociatedData = Array.create 12 0x04uy
            }

        let tag = Argon2id.derive rfcPassword rfcSalt 3 32 4 32 options

        Assert.Equal(
            "0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659",
            Convert.ToHexString(tag).ToLowerInvariant()
        )

    [<Fact>]
    let ``hex matches byte form`` () =
        let tag = Argon2id.deriveDefault rfcPassword rfcSalt 3 32 4 32
        let expected = Convert.ToHexString(tag).ToLowerInvariant()
        Assert.Equal(expected, Argon2id.deriveHexDefault rfcPassword rfcSalt 3 32 4 32)

    [<Theory>]
    [<InlineData(4)>]
    [<InlineData(16)>]
    [<InlineData(32)>]
    [<InlineData(64)>]
    [<InlineData(65)>]
    [<InlineData(128)>]
    let ``supports variable tag lengths`` tagLength =
        let tag = Argon2id.deriveDefault "password"B "saltsalt"B 1 8 1 tagLength
        Assert.Equal(tagLength, tag.Length)

    [<Fact>]
    let ``secret inputs bind the output`` () =
        let baseline = Argon2id.deriveDefault "password"B "saltsalt"B 1 8 1 32

        let withKey =
            Argon2id.derive
                "password"B
                "saltsalt"B
                1
                8
                1
                32
                { Argon2idOptions.Default with Key = "secret!!"B }

        let withAssociatedData =
            Argon2id.derive
                "password"B
                "saltsalt"B
                1
                8
                1
                32
                { Argon2idOptions.Default with AssociatedData = "context"B }

        Assert.NotEqual<byte array>(baseline, withKey)
        Assert.NotEqual<byte array>(baseline, withAssociatedData)

    [<Fact>]
    let ``password salt and pass count change output`` () =
        let baseline = Argon2id.deriveDefault "password"B "saltsalt"B 1 8 1 32
        Assert.NotEqual<byte array>(baseline, Argon2id.deriveDefault "password2"B "saltsalt"B 1 8 1 32)
        Assert.NotEqual<byte array>(baseline, Argon2id.deriveDefault "password"B "saltsal2"B 1 8 1 32)
        Assert.NotEqual<byte array>(baseline, Argon2id.deriveDefault "password"B "saltsalt"B 2 8 1 32)

    [<Fact>]
    let ``repeats deterministically across hybrid address boundary`` () =
        let first = Argon2id.deriveDefault "password"B "saltsalt"B 1 520 1 16
        let second = Argon2id.deriveDefault "password"B "saltsalt"B 1 520 1 16
        Assert.Equal<byte array>(first, second)

    [<Fact>]
    let ``rejects null inputs`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Argon2id.deriveDefault null "saltsalt"B 1 8 1 32 |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () -> Argon2id.deriveDefault [||] null 1 8 1 32 |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () -> Argon2id.derive [||] "saltsalt"B 1 8 1 32 Unchecked.defaultof<_> |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () ->
            Argon2id.derive [||] "saltsalt"B 1 8 1 32 { Argon2idOptions.Default with Key = null }
            |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () ->
            Argon2id.derive [||] "saltsalt"B 1 8 1 32 { Argon2idOptions.Default with AssociatedData = null }
            |> ignore)
        |> ignore

    [<Fact>]
    let ``rejects invalid parameters`` () =
        let rejects action = Assert.Throws<ArgumentException>(Action action) |> ignore

        rejects (fun () -> Argon2id.deriveDefault [||] "short"B 1 8 1 32 |> ignore)
        rejects (fun () -> Argon2id.deriveDefault [||] "saltsalt"B 1 8 1 3 |> ignore)
        rejects (fun () -> Argon2id.deriveDefault [||] "saltsalt"B 1 8 0 32 |> ignore)
        rejects (fun () -> Argon2id.deriveDefault [||] "saltsalt"B 1 8 0x0100_0000 32 |> ignore)
        rejects (fun () -> Argon2id.deriveDefault [||] "saltsalt"B 1 7 1 32 |> ignore)
        rejects (fun () -> Argon2id.deriveDefault [||] "saltsalt"B 0 8 1 32 |> ignore)

        rejects (fun () ->
            Argon2id.derive [||] "saltsalt"B 1 8 1 32 { Argon2idOptions.Default with Version = 0x10u }
            |> ignore)
