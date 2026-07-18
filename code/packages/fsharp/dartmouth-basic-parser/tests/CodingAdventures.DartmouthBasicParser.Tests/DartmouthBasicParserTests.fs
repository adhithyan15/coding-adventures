namespace CodingAdventures.DartmouthBasicParser.Tests

open System
open CodingAdventures.DartmouthBasicParser.FSharp
open CodingAdventures.DartmouthBasicLexer.FSharp
open CodingAdventures.Parser.FSharp
open Xunit

module DartmouthBasicParserTests =
    let rec descendants (node: ASTNode) =
        seq {
            yield node
            for child in node.Children do
                match child with
                | :? ASTNode as nested -> yield! descendants nested
                | _ -> ()
        }

    [<Fact>]
    let parsesTheCompleteStatementAndExpressionSurface () =
        let source =
            "10 LET X = FNA(SIN(-X)) ^ 2 + A(1) / 3\n" +
            "20 PRINT \"HELLO\", X;\n" +
            "30 INPUT A, B\n" +
            "40 IF X >= 0 THEN 100\n" +
            "50 GOTO 100\n" +
            "60 GOSUB 200\n" +
            "70 RETURN\n" +
            "80 FOR I = 1 TO 10 STEP 2\n" +
            "90 NEXT I\n" +
            "100 STOP\n" +
            "110 REM THIS BODY IS IGNORED @@@\n" +
            "120 READ A, B\n" +
            "130 DATA 1, 2, 3\n" +
            "140 RESTORE\n" +
            "150 DIM A(10), B(2, 3)\n" +
            "160 DEF FNA(T) = T * T\n" +
            "170 END\n"

        let ast = DartmouthBasicParser.ParseDartmouthBasic(source)
        let rules = descendants ast |> Seq.map (fun node -> node.RuleName) |> Set.ofSeq

        Assert.Equal("program", ast.RuleName)
        Assert.True(ast.DescendantCount() > 100)
        Assert.Contains("let_stmt", rules)
        Assert.Contains("print_stmt", rules)
        Assert.Contains("if_stmt", rules)
        Assert.Contains("for_stmt", rules)
        Assert.Contains("data_stmt", rules)
        Assert.Contains("dim_stmt", rules)
        Assert.Contains("def_stmt", rules)

    [<Fact>]
    let configuredParserParsesBareAndEmptyPrograms () =
        Assert.Equal("program", DartmouthBasicParser.CreateDartmouthBasicParser("10\n").Parse().RuleName)
        Assert.Equal("program", DartmouthBasicParser.ParseDartmouthBasic(String.Empty).RuleName)
        Assert.Equal("program", DartmouthBasicParser.ParseTokens(DartmouthBasicLexer.TokenizeDartmouthBasic("20 END\n")).RuleName)

    [<Theory>]
    [<InlineData("10 LET X 5\n")>]
    [<InlineData("10 IF X > 0 100\n")>]
    [<InlineData("10 FOR I = 1\n")>]
    [<InlineData("10 END @\n")>]
    let rejectsMalformedOrUnconsumedInput (source: string) =
        let error = Assert.Throws<ArgumentException>(fun () -> DartmouthBasicParser.ParseDartmouthBasic(source) |> ignore)
        Assert.Contains("Dartmouth BASIC parse failed", error.Message)

    [<Fact>]
    let rejectsNullSource () =
        Assert.Throws<ArgumentNullException>(fun () -> DartmouthBasicParser.CreateDartmouthBasicParser(null) |> ignore)
        |> ignore
        Assert.Throws<ArgumentNullException>(fun () -> DartmouthBasicParser.ParseTokens(null) |> ignore)
        |> ignore

    [<Fact>]
    let tokenApiRequiresFinalEof () =
        let tokens =
            DartmouthBasicLexer.TokenizeDartmouthBasic("20 END\n")
            |> Seq.take 3
            |> Seq.toArray
        Assert.Throws<ArgumentException>(fun () -> DartmouthBasicParser.ParseTokens(tokens) |> ignore)
        |> ignore
        Assert.Throws<ArgumentException>(fun () -> DartmouthBasicParser.ParseTokens(Array.empty) |> ignore)
        |> ignore
