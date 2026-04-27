namespace CodingAdventures.VerilogParser.Tests;

public sealed class VerilogParserTests
{
    [Fact]
    public void ParsesSample()
    {
        var ast = CodingAdventures.VerilogParser.VerilogParser.ParseVerilog("module top; endmodule");

        Assert.Equal("source_text", ast.RuleName);
        Assert.True(ast.DescendantCount() > 0);
    }

    [Fact]
    public void DefaultVersionMatchesExplicitVersion()
    {
        Assert.Equal(
            CodingAdventures.VerilogParser.VerilogParser.ParseVerilog("module top; endmodule").RuleName,
            CodingAdventures.VerilogParser.VerilogParser.ParseVerilog("module top; endmodule", CodingAdventures.VerilogParser.VerilogParser.DefaultVersion).RuleName);
    }

    [Fact]
    public void RejectsUnknownVersion()
    {
        var error = Assert.Throws<ArgumentException>(() => CodingAdventures.VerilogParser.VerilogParser.ParseVerilog("module top; endmodule", "2099"));
        Assert.Contains("Unknown Verilog version", error.Message);
    }
}
