namespace CodingAdventures.VerilogParser.Tests

open CodingAdventures.VerilogParser.FSharp
open Xunit

module VerilogParserTests =
    [<Fact>]
    let parsesSample () =
        let ast = VerilogParser.ParseVerilog("module top; endmodule")

        Assert.Equal("source_text", ast.RuleName)
        Assert.True(ast.DescendantCount() > 0)

    [<Fact>]
    let defaultVersionMatchesExplicitVersion () =
        Assert.Equal(
            VerilogParser.ParseVerilog("module top; endmodule").RuleName,
            VerilogParser.ParseVerilog("module top; endmodule", VerilogParser.DefaultVersion).RuleName)

    [<Fact>]
    let rejectsUnknownVersion () =
        let error = Assert.Throws<System.ArgumentException>(fun () -> VerilogParser.ParseVerilog("module top; endmodule", "2099") |> ignore)
        Assert.Contains("Unknown Verilog version", error.Message)
