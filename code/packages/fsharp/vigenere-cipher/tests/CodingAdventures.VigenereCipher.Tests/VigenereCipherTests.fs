namespace CodingAdventures.VigenereCipher.Tests

open System
open Xunit
open CodingAdventures.VigenereCipher

type VigenereCipherTests() =
    let longEnglishText =
        "THE ART OF ENCIPHERING AND DECIPHERING MESSAGES HAS A LONG AND STORIED " +
        "HISTORY STRETCHING BACK TO ANCIENT TIMES. THE SPARTANS USED THE SCYTALE, " +
        "JULIUS CAESAR USED A SIMPLE SHIFT CIPHER, AND DURING THE RENAISSANCE THE " +
        "VIGENERE CIPHER WAS CONSIDERED UNBREAKABLE FOR NEARLY THREE HUNDRED YEARS. " +
        "CHARLES BABBAGE BROKE THE CIPHER IN THE EIGHTEEN FIFTIES USING THE INDEX."

    [<Fact>]
    member _.``Encrypt and decrypt known vectors``() =
        Assert.Equal("LXFOPVEFRNHR", VigenereCipher.encrypt "ATTACKATDAWN" "LEMON")
        Assert.Equal("ATTACKATDAWN", VigenereCipher.decrypt "LXFOPVEFRNHR" "LEMON")
        Assert.Equal("Rijvs, Uyvjn!", VigenereCipher.encrypt "Hello, World!" "key")
        Assert.Equal("Hello, World!", VigenereCipher.decrypt "Rijvs, Uyvjn!" "key")

    [<Fact>]
    member _.``Encrypt preserves case and punctuation``() =
        Assert.Equal("A B", VigenereCipher.encrypt "A A" "AB")
        Assert.Equal("123 !@#", VigenereCipher.encrypt "123 !@#" "key")

        let original = "Cafe\u0301 costs $3.50"
        let encrypted = VigenereCipher.encrypt original "KEY"
        Assert.Equal(original, VigenereCipher.decrypt encrypted "KEY")
        Assert.Equal("Hj", VigenereCipher.encrypt "Hi" "ABCDEFGHIJ")

    [<Fact>]
    member _.``Round trips across keys``() =
        let texts =
            [|
                "ATTACKATDAWN"
                "Hello, World!"
                "The quick brown fox jumps over the lazy dog."
                "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
                ""
            |]

        let keys = [| "KEY"; "LEMON"; "A"; "SECRETKEY"; "MiXeD" |]

        for text in texts do
            for key in keys do
                let encrypted = VigenereCipher.encrypt text key
                Assert.Equal(text, VigenereCipher.decrypt encrypted key)

    [<Fact>]
    member _.``Invalid keys are rejected``() =
        Assert.Throws<ArgumentException>(fun () -> VigenereCipher.encrypt "hello" "" |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> VigenereCipher.encrypt "hello" "key1" |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> VigenereCipher.decrypt "hello" "ke y" |> ignore) |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> VigenereCipher.encrypt "hello" null |> ignore) |> ignore
        Assert.Equal("", VigenereCipher.findKey "ABC" 0)

    [<Fact>]
    member _.``Finds key lengths``() =
        Assert.Equal(1, VigenereCipher.findKeyLengthDefault "A")
        Assert.Equal(5, VigenereCipher.findKeyLengthDefault (VigenereCipher.encrypt longEnglishText "LEMON"))
        Assert.Equal(3, VigenereCipher.findKeyLengthDefault (VigenereCipher.encrypt longEnglishText "KEY"))

    [<Fact>]
    member _.``Recovers keys``() =
        Assert.Equal("LEMON", VigenereCipher.findKey (VigenereCipher.encrypt longEnglishText "LEMON") 5)
        Assert.Equal("KEY", VigenereCipher.findKey (VigenereCipher.encrypt longEnglishText "KEY") 3)
        let lowSignalKey = VigenereCipher.findKey "A" 2
        Assert.Equal(2, lowSignalKey.Length)
        Assert.EndsWith("A", lowSignalKey)

    [<Fact>]
    member _.``Break cipher recovers key and plaintext``() =
        let ciphertext = VigenereCipher.encrypt longEnglishText "LEMON"
        let result = VigenereCipher.breakCipher ciphertext
        Assert.Equal("LEMON", result.Key)
        Assert.Equal(longEnglishText, result.Plaintext)

    [<Fact>]
    member _.``English frequencies match expectations``() =
        Assert.Equal(26, VigenereCipher.englishFrequencies.Length)
        Assert.InRange(Array.sum VigenereCipher.englishFrequencies, 0.99, 1.01)
        Assert.True(VigenereCipher.englishFrequencies[4] > VigenereCipher.englishFrequencies[25])

    [<Fact>]
    member _.``CR03 analysis limits and ordering``() =
        let atLimit = String.replicate 8192 "😀"
        let overLimit = atLimit + "😀"
        Assert.Equal(1, VigenereCipher.findKeyLength atLimit 40)
        Assert.Throws<ArgumentException>(fun () -> VigenereCipher.findKeyLength overLimit 20 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> VigenereCipher.findKeyLength overLimit 41 |> ignore) |> ignore
        Assert.Equal("", VigenereCipher.findKey overLimit 0)
        Assert.Throws<ArgumentException>(fun () -> VigenereCipher.findKey overLimit 1 |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> VigenereCipher.findKey overLimit 41 |> ignore) |> ignore

    [<Fact>]
    member _.``CR03 uses ASCII only and preserves requested key length``() =
        let plaintext = "Hello, 😀Wörld!"
        let ciphertext = "Rijvs, 😀Uöbpb!"
        Assert.Equal(ciphertext, VigenereCipher.encrypt plaintext "kEy")
        Assert.Equal(plaintext, VigenereCipher.decrypt ciphertext "kEy")
        Assert.Throws<ArgumentException>(fun () -> VigenereCipher.decrypt "" "KÉY" |> ignore) |> ignore
        Assert.Throws<ArgumentException>(fun () -> VigenereCipher.decrypt "" "KſY" |> ignore) |> ignore
        Assert.Equal(2, VigenereCipher.findKeyLength "AéA😀AЖAéABB" 4)
        Assert.Equal(String.replicate 40 "A", VigenereCipher.findKey "Eé😀Ж" 40)
