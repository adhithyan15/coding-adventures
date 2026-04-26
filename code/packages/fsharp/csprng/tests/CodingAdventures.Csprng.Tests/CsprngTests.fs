namespace CodingAdventures.Csprng.Tests

open System
open Xunit
open CodingAdventures.Csprng.FSharp

module CsprngTests =
    [<Fact>]
    let ``random bytes returns requested length and entropy`` () =
        let bytes = Csprng.randomBytes 32

        Assert.Equal(32, bytes.Length)
        Assert.Contains<byte>(bytes, fun value -> value <> 0uy)

    [<Fact>]
    let ``fill random mutates buffer`` () =
        let buffer = Array.zeroCreate<byte> 32

        Csprng.fillRandom buffer

        Assert.Contains<byte>(buffer, fun value -> value <> 0uy)

    [<Fact>]
    let ``random integers return values`` () =
        Csprng.randomUInt32 () |> ignore
        Csprng.randomUInt64 () |> ignore

    [<Fact>]
    let ``validation rejects invalid requests`` () =
        Assert.Throws<ArgumentNullException>(fun () -> Csprng.fillRandom null) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Csprng.fillRandom [||]) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Csprng.randomBytes 0 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> Csprng.randomBytes -1 |> ignore) |> ignore
