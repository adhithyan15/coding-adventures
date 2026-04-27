namespace CodingAdventures.StarlarkParser.Tests;

public sealed class StarlarkParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("starlark-parser", new CodingAdventures.StarlarkParser.StarlarkParser().Ping());
    }
}