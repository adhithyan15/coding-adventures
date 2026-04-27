namespace CodingAdventures.ExcelParser.Tests;

public sealed class ExcelParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("excel-parser", new CodingAdventures.ExcelParser.ExcelParser().Ping());
    }
}