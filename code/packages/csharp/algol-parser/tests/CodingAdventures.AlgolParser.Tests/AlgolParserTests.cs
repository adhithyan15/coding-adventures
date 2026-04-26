namespace CodingAdventures.AlgolParser.Tests;

public sealed class AlgolParserTests
{
    [Fact]
    public void ParsesSample()
    {
        var ast = CodingAdventures.AlgolParser.AlgolParser.ParseAlgol("begin integer x; x := 42 end");

        Assert.Equal("program", ast.RuleName);
        Assert.True(ast.DescendantCount() > 0);
    }

    [Fact]
    public void DefaultVersionMatchesExplicitVersion()
    {
        Assert.Equal(
            CodingAdventures.AlgolParser.AlgolParser.ParseAlgol("begin integer x; x := 42 end").RuleName,
            CodingAdventures.AlgolParser.AlgolParser.ParseAlgol("begin integer x; x := 42 end", CodingAdventures.AlgolParser.AlgolParser.DefaultVersion).RuleName);
    }

    [Fact]
    public void RejectsUnknownVersion()
    {
        var error = Assert.Throws<ArgumentException>(() => CodingAdventures.AlgolParser.AlgolParser.ParseAlgol("begin integer x; x := 42 end", "algol68"));
        Assert.Contains("Unknown ALGOL version", error.Message);
    }
}
