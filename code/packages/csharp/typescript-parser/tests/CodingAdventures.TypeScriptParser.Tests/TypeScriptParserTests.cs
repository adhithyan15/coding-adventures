namespace CodingAdventures.TypeScriptParser.Tests;

public sealed class TypeScriptParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("typescript-parser", new CodingAdventures.TypeScriptParser.TypeScriptParser().Ping());
    }
}