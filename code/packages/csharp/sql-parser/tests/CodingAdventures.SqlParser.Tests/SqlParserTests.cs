namespace CodingAdventures.SqlParser.Tests;

public sealed class SqlParserTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("sql-parser", new CodingAdventures.SqlParser.SqlParser().Ping());
    }
}