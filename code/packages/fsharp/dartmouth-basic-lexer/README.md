# CodingAdventures.DartmouthBasicLexer.FSharp

A pure F# lexer for the original 1964 Dartmouth BASIC language. It embeds the
shared `dartmouth_basic.tokens` grammar and applies the language's contextual
line-label and `REM` rules after generic tokenization.

```fsharp
let tokens =
    DartmouthBasicLexer.TokenizeDartmouthBasic(
        "10 LET X = 5\n20 PRINT X\n30 END\n")
```

The normalized token contract matches the C# and Go ports. Tokenization is
fully in memory and requires no filesystem or network capability.
