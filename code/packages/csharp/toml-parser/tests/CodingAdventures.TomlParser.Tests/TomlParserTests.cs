namespace CodingAdventures.TomlParser.Tests;

public sealed class TomlParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("toml-parser", new CodingAdventures.TomlParser.TomlParser().Ping());
    }
}