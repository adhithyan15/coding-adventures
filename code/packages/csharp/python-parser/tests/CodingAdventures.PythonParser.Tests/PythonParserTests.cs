namespace CodingAdventures.PythonParser.Tests;

public sealed class PythonParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("python-parser", new CodingAdventures.PythonParser.PythonParser().Ping());
    }
}