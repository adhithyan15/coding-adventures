namespace CodingAdventures.JavaParser.Tests;

public sealed class JavaParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("java-parser", new CodingAdventures.JavaParser.JavaParser().Ping());
    }
}