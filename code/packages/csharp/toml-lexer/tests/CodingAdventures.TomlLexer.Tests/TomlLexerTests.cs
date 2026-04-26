namespace CodingAdventures.TomlLexer.Tests;

public sealed class TomlLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("toml-lexer", new CodingAdventures.TomlLexer.TomlLexer().Ping());
    }
}