namespace CodingAdventures.ScytaleCipher.Tests

open System
open Xunit
open CodingAdventures.ScytaleCipher

type ScytaleCipherTests() =
    [<Fact>]
    member _.``encrypt matches reference examples``() =
        Assert.Equal("HLWLEOODL R ", ScytaleCipher.encrypt "HELLO WORLD" 3)
        Assert.Equal("ACEBDF", ScytaleCipher.encrypt "ABCDEF" 2)
        Assert.Equal("ADBECF", ScytaleCipher.encrypt "ABCDEF" 3)
        Assert.Equal("ABCD", ScytaleCipher.encrypt "ABCD" 4)
        Assert.Equal(String.Empty, ScytaleCipher.encrypt String.Empty 2)

    [<Fact>]
    member _.``decrypt strips padding and matches reference examples``() =
        Assert.Equal("HELLO WORLD", ScytaleCipher.decrypt "HLWLEOODL R " 3)
        Assert.Equal("ABCDEF", ScytaleCipher.decrypt "ACEBDF" 2)
        Assert.Equal("HELLO", ScytaleCipher.decrypt (ScytaleCipher.encrypt "HELLO" 3) 3)
        Assert.Equal(String.Empty, ScytaleCipher.decrypt String.Empty 2)

    [<Fact>]
    member _.``round trips across valid keys``() =
        let text = "The quick brown fox jumps over the lazy dog!"

        for key in 2 .. text.Length / 2 do
            Assert.Equal(text, ScytaleCipher.decrypt (ScytaleCipher.encrypt text key) key)

    [<Fact>]
    member _.``brute force returns candidate plaintexts``() =
        let ciphertext = ScytaleCipher.encrypt "HELLO WORLD" 3
        let results = ScytaleCipher.bruteForce ciphertext

        Assert.Contains(results, fun result -> result.Key = 3 && result.Text = "HELLO WORLD")
        Assert.Equal<int list>([ 2; 3; 4; 5 ], ScytaleCipher.bruteForce "ABCDEFGHIJ" |> List.map _.Key)
        Assert.Empty(ScytaleCipher.bruteForce "ABC")

    [<Fact>]
    member _.``invalid inputs throw``() =
        Assert.Throws<ArgumentNullException>(fun () -> ScytaleCipher.encrypt null 2 |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> ScytaleCipher.decrypt null 2 |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> ScytaleCipher.bruteForce null |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> ScytaleCipher.encrypt "HELLO" 1 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> ScytaleCipher.decrypt "HI" 3 |> ignore) |> ignore
