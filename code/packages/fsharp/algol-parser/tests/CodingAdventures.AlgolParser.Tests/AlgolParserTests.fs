namespace CodingAdventures.AlgolParser.Tests

open CodingAdventures.AlgolParser.FSharp
open Xunit

module AlgolParserTests =
    [<Fact>]
    let parsesSample () =
        let ast = AlgolParser.ParseAlgol("begin integer x; x := 42 end")

        Assert.Equal("program", ast.RuleName)
        Assert.True(ast.DescendantCount() > 0)

    [<Fact>]
    let defaultVersionMatchesExplicitVersion () =
        Assert.Equal(
            AlgolParser.ParseAlgol("begin integer x; x := 42 end").RuleName,
            AlgolParser.ParseAlgol("begin integer x; x := 42 end", AlgolParser.DefaultVersion).RuleName)

    [<Fact>]
    let rejectsUnknownVersion () =
        let error = Assert.Throws<System.ArgumentException>(fun () -> AlgolParser.ParseAlgol("begin integer x; x := 42 end", "algol68") |> ignore)
        Assert.Contains("Unknown ALGOL version", error.Message)
