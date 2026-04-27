namespace CodingAdventures.VhdlParser.Tests

open CodingAdventures.VhdlParser.FSharp
open Xunit

module VhdlParserTests =
    [<Fact>]
    let parsesSample () =
        let ast = VhdlParser.ParseVhdl("entity top is end entity top;")

        Assert.Equal("design_file", ast.RuleName)
        Assert.True(ast.DescendantCount() > 0)

    [<Fact>]
    let defaultVersionMatchesExplicitVersion () =
        Assert.Equal(
            VhdlParser.ParseVhdl("entity top is end entity top;").RuleName,
            VhdlParser.ParseVhdl("entity top is end entity top;", VhdlParser.DefaultVersion).RuleName)

    [<Fact>]
    let rejectsUnknownVersion () =
        let error = Assert.Throws<System.ArgumentException>(fun () -> VhdlParser.ParseVhdl("entity top is end entity top;", "2099") |> ignore)
        Assert.Contains("Unknown VHDL version", error.Message)
