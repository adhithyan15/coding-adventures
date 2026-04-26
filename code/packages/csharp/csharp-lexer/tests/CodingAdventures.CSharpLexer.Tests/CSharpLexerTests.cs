namespace CodingAdventures.CSharpLexer.Tests;

public sealed class CSharpLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("csharp-lexer", new CodingAdventures.CSharpLexer.CSharpLexer().Ping());
    }
}