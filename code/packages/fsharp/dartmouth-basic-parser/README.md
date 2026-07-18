# CodingAdventures.DartmouthBasicParser.FSharp

A pure F# parser for the original 1964 Dartmouth BASIC language. It composes
the native `DartmouthBasicLexer` with the generic grammar parser and embeds the
shared `dartmouth_basic.grammar` resource.

```fsharp
let ast =
    DartmouthBasicParser.ParseDartmouthBasic(
        "10 LET X = 5\n20 PRINT X\n30 END\n")
printfn "%s" ast.RuleName // program
```

The configured-parser API supports two-stage use, while `ParseTokens` accepts
an existing lexer stream. Complete token-stream consumption rejects malformed
or incomplete statements. Parsing is fully in memory and needs no filesystem
or network capability.
