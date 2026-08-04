namespace CodingAdventures.DartmouthBasicLexer.Tests

open System
open CodingAdventures.DartmouthBasicLexer.FSharp
open CodingAdventures.Lexer.FSharp
open Xunit

module DartmouthBasicLexerTests =
    let types (tokens: seq<Token>) = tokens |> Seq.map (fun token -> token.EffectiveTypeName) |> Seq.toArray
    let values (tokens: seq<Token>) = tokens |> Seq.map (fun token -> token.Value) |> Seq.toArray

    [<Fact>]
    let tokenizesAndNormalizesAStatement () =
        let tokens = DartmouthBasicLexer.TokenizeDartmouthBasic("10 let X = 1.5E3\r\n")
        Assert.Equal<string array>([| "LINE_NUM"; "KEYWORD"; "NAME"; "EQ"; "NUMBER"; "NEWLINE"; "EOF" |], types tokens)
        Assert.Equal<string array>([| "10"; "LET"; "x"; "="; "1.5e3"; "\\n"; "" |], values tokens)
        Assert.Equal((1, 1), (tokens.[0].Line, tokens.[0].Column))
        Assert.Equal((2, 1), (tokens.[tokens.Count - 1].Line, tokens.[tokens.Count - 1].Column))

    [<Fact>]
    let relabelsOnlyTheFirstNumberOnEachPhysicalLine () =
        let tokens = DartmouthBasicLexer.TokenizeDartmouthBasic("30 GOTO 10\n40 PRINT 20\n")
        let numberTypes =
            tokens
            |> Seq.filter (fun token -> token.EffectiveTypeName = "LINE_NUM" || token.EffectiveTypeName = "NUMBER")
            |> types
        Assert.Equal<string array>([| "LINE_NUM"; "NUMBER"; "LINE_NUM"; "NUMBER" |], numberTypes)

    [<Fact>]
    let suppressesRemarkBodyButRetainsItsNewline () =
        let tokens = DartmouthBasicLexer.CreateDartmouthBasicLexer("10 rem GOTO 20 @ ignored\n20 END\n").Tokenize()
        Assert.Equal<string array>([| "LINE_NUM"; "KEYWORD"; "NEWLINE"; "LINE_NUM"; "KEYWORD"; "NEWLINE"; "EOF" |], types tokens)
        Assert.Equal("REM", tokens.[1].Value)

    [<Fact>]
    let preservesStringCaseWithoutQuotes () =
        let tokens = DartmouthBasicLexer.TokenizeDartmouthBasic("10 PRINT \"Hello, World!\"\n")
        let stringToken = tokens |> Seq.find (fun token -> token.EffectiveTypeName = "STRING")
        Assert.Equal("Hello, World!", stringToken.Value)

    [<Fact>]
    let classifiesFunctionsNamesAndUnknownCharacters () =
        let tokens = DartmouthBasicLexer.TokenizeDartmouthBasic("10 LET Result = SIN(FNA(X)) @\n")
        Assert.Contains(tokens, fun token -> token.EffectiveTypeName = "BUILTIN_FN" && token.Value = "sin")
        Assert.Contains(tokens, fun token -> token.EffectiveTypeName = "USER_FN" && token.Value = "fna")
        Assert.Contains(tokens, fun token -> token.EffectiveTypeName = "NAME" && token.Value = "result")
        Assert.Contains(tokens, fun token -> token.EffectiveTypeName = "UNKNOWN" && token.Value = "@")

    [<Theory>]
    [<InlineData("<=", "LE")>]
    [<InlineData(">=", "GE")>]
    [<InlineData("<>", "NE")>]
    [<InlineData("^", "CARET")>]
    [<InlineData(";", "SEMICOLON")>]
    let recognizesOperators (source: string) (expectedType: string) =
        Assert.Equal(expectedType, DartmouthBasicLexer.TokenizeDartmouthBasic(source).[0].EffectiveTypeName)

    [<Fact>]
    let aBlankLineStillAllowsTheNextLineLabel () =
        let tokens = DartmouthBasicLexer.TokenizeDartmouthBasic("\n10 END\n")
        Assert.Equal("NEWLINE", tokens.[0].EffectiveTypeName)
        Assert.Equal("LINE_NUM", tokens.[1].EffectiveTypeName)

    [<Fact>]
    let rejectsNullSource () =
        Assert.Throws<ArgumentNullException>(fun () -> DartmouthBasicLexer.CreateDartmouthBasicLexer(null) |> ignore)
        |> ignore
