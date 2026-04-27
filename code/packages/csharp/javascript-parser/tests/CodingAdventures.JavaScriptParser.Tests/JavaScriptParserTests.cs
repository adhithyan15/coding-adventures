namespace CodingAdventures.JavaScriptParser.Tests;

public sealed class JavaScriptParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("javascript-parser", new CodingAdventures.JavaScriptParser.JavaScriptParser().Ping());
    }
}