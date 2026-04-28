namespace CodingAdventures.RubyParser.Tests;

public sealed class RubyParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("ruby-parser", new CodingAdventures.RubyParser.RubyParser().Ping());
    }
}