namespace CodingAdventures.LatticeParser.Tests;

public sealed class LatticeParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("lattice-parser", new CodingAdventures.LatticeParser.LatticeParser().Ping());
    }
}