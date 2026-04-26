namespace CodingAdventures.JavaScriptLexer.Tests;

public sealed class JavaScriptLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("javascript-lexer", new CodingAdventures.JavaScriptLexer.JavaScriptLexer().Ping());
    }
}