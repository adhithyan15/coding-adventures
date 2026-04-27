namespace CodingAdventures.JsonParser.Tests;

public sealed class JsonParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("json-parser", new CodingAdventures.JsonParser.JsonParser().Ping());
    }
}