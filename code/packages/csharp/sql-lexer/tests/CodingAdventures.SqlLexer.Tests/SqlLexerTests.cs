namespace CodingAdventures.SqlLexer.Tests;

public sealed class SqlLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("sql-lexer", new CodingAdventures.SqlLexer.SqlLexer().Ping());
    }
}