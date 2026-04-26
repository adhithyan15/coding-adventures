namespace CodingAdventures.JavaLexer.Tests;

public sealed class JavaLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("java-lexer", new CodingAdventures.JavaLexer.JavaLexer().Ping());
    }
}