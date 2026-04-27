namespace CodingAdventures.CaesarCipher.Tests

open Xunit
open CodingAdventures.CaesarCipher

type CaesarCipherTests() =
    [<Fact>]
    member _.``Encrypt and decrypt handle known examples``() =
        Assert.Equal("KHOOR", CaesarCipher.encrypt "HELLO" 3)
        Assert.Equal("khoor", CaesarCipher.encrypt "hello" 3)
        Assert.Equal("Khoor, Zruog!", CaesarCipher.encrypt "Hello, World!" 3)
        Assert.Equal("HELLO", CaesarCipher.decrypt "KHOOR" 3)
        Assert.Equal("Hello, World!", CaesarCipher.decrypt "Khoor, Zruog!" 3)

    [<Fact>]
    member _.``Encrypt respects wrapping and negative shifts``() =
        Assert.Equal("abc XYZ 123!", CaesarCipher.encrypt "abc XYZ 123!" 0)
        Assert.Equal("Wrap around test", CaesarCipher.encrypt "Wrap around test" 26)
        Assert.Equal("Double wrap", CaesarCipher.encrypt "Double wrap" 52)
        Assert.Equal("ZAB", CaesarCipher.encrypt "ABC" -1)
        Assert.Equal("ZAB", CaesarCipher.encrypt "ABC" -27)
        Assert.Equal("ABC", CaesarCipher.encrypt "XYZ" 3)

    [<Fact>]
    member _.``Round trips across many shifts``() =
        let original = "The Quick Brown Fox Jumps Over The Lazy Dog! 123"

        for shift in -30 .. 30 do
            let encrypted = CaesarCipher.encrypt original shift
            let decrypted = CaesarCipher.decrypt encrypted shift
            Assert.Equal(original, decrypted)

    [<Fact>]
    member _.``Rot13 is self inverse``() =
        let text = "The Quick Brown Fox! 123"
        Assert.Equal(text, CaesarCipher.rot13 (CaesarCipher.rot13 text))
        Assert.Equal(CaesarCipher.encrypt text 13, CaesarCipher.rot13 text)

    [<Fact>]
    member _.``Non ASCII characters pass through``() =
        let original = "Cafe\u0301 costs $3.50 \u2764"
        let encrypted = CaesarCipher.encrypt original 7
        let decrypted = CaesarCipher.decrypt encrypted 7
        Assert.Equal(original, decrypted)

    [<Fact>]
    member _.``Brute force returns every candidate``() =
        let results = CaesarCipher.bruteForce "KHOOR"
        Assert.Equal(25, List.length results)
        Assert.Equal(3, results[2].Shift)
        Assert.Equal("HELLO", results[2].Plaintext)

        for result in results do
            Assert.InRange(result.Shift, 1, 25)

    [<Fact>]
    member _.``Brute force handles empty and punctuation only input``() =
        let emptyResults = CaesarCipher.bruteForce ""
        Assert.Equal(25, List.length emptyResults)

        for result in emptyResults do
            Assert.Equal("", result.Plaintext)

        let punctuationResults = CaesarCipher.bruteForce "123!!!"
        for result in punctuationResults do
            Assert.Equal("123!!!", result.Plaintext)

    [<Fact>]
    member _.``Frequency analysis finds known shifts``() =
        let plaintext1 = "THE QUICK BROWN FOX JUMPS OVER THE LAZY DOG"
        let ciphertext1 = CaesarCipher.encrypt plaintext1 3
        let shift1, decoded1 = CaesarCipher.frequencyAnalysis ciphertext1
        Assert.Equal(3, shift1)
        Assert.Equal(plaintext1, decoded1)

        let plaintext2 =
            "IN CRYPTOGRAPHY A CAESAR CIPHER ALSO KNOWN AS SHIFT CIPHER IS ONE OF THE SIMPLEST AND MOST WIDELY KNOWN ENCRYPTION TECHNIQUES"

        let ciphertext2 = CaesarCipher.encrypt plaintext2 17
        let shift2, decoded2 = CaesarCipher.frequencyAnalysis ciphertext2
        Assert.Equal(17, shift2)
        Assert.Equal(plaintext2, decoded2)

    [<Fact>]
    member _.``Frequency analysis handles low signal inputs``() =
        let shift1, decoded1 = CaesarCipher.frequencyAnalysis ""
        Assert.InRange(shift1, 1, 25)
        Assert.Equal("", decoded1)

        let shift2, decoded2 = CaesarCipher.frequencyAnalysis "12345!@#$%"
        Assert.Equal(1, shift2)
        Assert.Equal("12345!@#$%", decoded2)

        let shift3, _ = CaesarCipher.frequencyAnalysis "EEEEEEEEEEE"
        Assert.InRange(shift3, 1, 25)

    [<Fact>]
    member _.``English frequencies match expectations``() =
        Assert.Equal(26, CaesarCipher.englishFrequencies.Length)
        Assert.InRange(Array.sum CaesarCipher.englishFrequencies, 0.99, 1.01)

        let eFrequency = CaesarCipher.englishFrequencies[4]

        for i in 0 .. CaesarCipher.englishFrequencies.Length - 1 do
            Assert.True(CaesarCipher.englishFrequencies[i] > 0.0)
            if i <> 4 then
                Assert.True(eFrequency > CaesarCipher.englishFrequencies[i])
