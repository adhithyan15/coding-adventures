namespace CodingAdventures.PythonLexer.Tests;

public sealed class PythonLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("python-lexer", new CodingAdventures.PythonLexer.PythonLexer().Ping());
    }
}