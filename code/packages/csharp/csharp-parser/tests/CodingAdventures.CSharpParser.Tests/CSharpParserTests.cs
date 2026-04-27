namespace CodingAdventures.CSharpParser.Tests;

public sealed class CSharpParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("csharp-parser", new CodingAdventures.CSharpParser.CSharpParser().Ping());
    }
}