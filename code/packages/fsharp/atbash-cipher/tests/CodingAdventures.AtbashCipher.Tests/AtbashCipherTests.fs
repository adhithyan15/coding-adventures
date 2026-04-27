namespace CodingAdventures.AtbashCipher.Tests

open Xunit
open CodingAdventures.AtbashCipher

type AtbashCipherTests() =
    [<Fact>]
    member _.``Encrypt matches classic examples``() =
        Assert.Equal("SVOOL", AtbashCipher.encrypt "HELLO")
        Assert.Equal("svool", AtbashCipher.encrypt "hello")
        Assert.Equal("Svool, Dliow! 123", AtbashCipher.encrypt "Hello, World! 123")

    [<Fact>]
    member _.``Encrypt preserves case and passthrough characters``() =
        Assert.Equal("ZYX", AtbashCipher.encrypt "ABC")
        Assert.Equal("zyx", AtbashCipher.encrypt "abc")
        Assert.Equal("ZyXwVu", AtbashCipher.encrypt "AbCdEf")
        Assert.Equal("12345", AtbashCipher.encrypt "12345")
        Assert.Equal("!@#$%", AtbashCipher.encrypt "!@#$%")
        Assert.Equal("Z\nY\tX", AtbashCipher.encrypt "A\nB\tC")

    [<Fact>]
    member _.``Encrypt transforms the full alphabet``() =
        Assert.Equal("ZYXWVUTSRQPONMLKJIHGFEDCBA", AtbashCipher.encrypt "ABCDEFGHIJKLMNOPQRSTUVWXYZ")
        Assert.Equal("zyxwvutsrqponmlkjihgfedcba", AtbashCipher.encrypt "abcdefghijklmnopqrstuvwxyz")

    [<Fact>]
    member _.``Cipher is self inverse``() =
        let text = "The quick brown fox jumps over the lazy dog! 42"
        Assert.Equal(text, AtbashCipher.encrypt (AtbashCipher.encrypt text))
        Assert.Equal(text, AtbashCipher.decrypt (AtbashCipher.encrypt text))
        Assert.Equal(AtbashCipher.encrypt text, AtbashCipher.decrypt text)

    [<Fact>]
    member _.``Edge cases remain stable``() =
        Assert.Equal("", AtbashCipher.encrypt "")
        Assert.Equal("5", AtbashCipher.encrypt "5")
        Assert.Equal("Z", AtbashCipher.encrypt "A")
        Assert.Equal("A", AtbashCipher.encrypt "Z")
        Assert.Equal("N", AtbashCipher.encrypt "M")
        Assert.Equal("M", AtbashCipher.encrypt "N")

    [<Fact>]
    member _.``No letter maps to itself``() =
        for offset in 0 .. 25 do
            let upper = string (char (int 'A' + offset))
            let lower = string (char (int 'a' + offset))
            Assert.NotEqual<string>(upper, AtbashCipher.encrypt upper)
            Assert.NotEqual<string>(lower, AtbashCipher.encrypt lower)
