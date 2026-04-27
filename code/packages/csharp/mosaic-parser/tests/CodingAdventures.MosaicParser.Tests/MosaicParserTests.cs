namespace CodingAdventures.MosaicParser.Tests;

public sealed class MosaicParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("mosaic-parser", new CodingAdventures.MosaicParser.MosaicParser().Ping());
    }
}