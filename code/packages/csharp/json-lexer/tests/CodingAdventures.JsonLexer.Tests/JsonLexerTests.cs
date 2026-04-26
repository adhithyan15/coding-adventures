namespace CodingAdventures.JsonLexer.Tests;

public sealed class JsonLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("json-lexer", new CodingAdventures.JsonLexer.JsonLexer().Ping());
    }
}