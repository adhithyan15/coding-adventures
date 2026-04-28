namespace CodingAdventures.VhdlParser.Tests;

public sealed class VhdlParserTests
{
    [Fact]
    public void ParsesSample()
    {
        var ast = CodingAdventures.VhdlParser.VhdlParser.ParseVhdl("entity top is end entity top;");

        Assert.Equal("design_file", ast.RuleName);
        Assert.True(ast.DescendantCount() > 0);
    }

    [Fact]
    public void DefaultVersionMatchesExplicitVersion()
    {
        Assert.Equal(
            CodingAdventures.VhdlParser.VhdlParser.ParseVhdl("entity top is end entity top;").RuleName,
            CodingAdventures.VhdlParser.VhdlParser.ParseVhdl("entity top is end entity top;", CodingAdventures.VhdlParser.VhdlParser.DefaultVersion).RuleName);
    }

    [Fact]
    public void RejectsUnknownVersion()
    {
        var error = Assert.Throws<ArgumentException>(() => CodingAdventures.VhdlParser.VhdlParser.ParseVhdl("entity top is end entity top;", "2099"));
        Assert.Contains("Unknown VHDL version", error.Message);
    }
}
