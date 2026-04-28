namespace CodingAdventures.MosaicLexer.Tests;

public sealed class MosaicLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("mosaic-lexer", new CodingAdventures.MosaicLexer.MosaicLexer().Ping());
    }
}