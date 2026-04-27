namespace CodingAdventures.RubyLexer.Tests;

public sealed class RubyLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("ruby-lexer", new CodingAdventures.RubyLexer.RubyLexer().Ping());
    }
}