namespace CodingAdventures.LatticeLexer.Tests;

public sealed class LatticeLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("lattice-lexer", new CodingAdventures.LatticeLexer.LatticeLexer().Ping());
    }
}