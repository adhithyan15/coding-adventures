namespace CodingAdventures.ExcelLexer.Tests;

public sealed class ExcelLexerTests
{
    [Fact]
    public void PingReturnsPackageName()
    {
        Assert.Equal("excel-lexer", new CodingAdventures.ExcelLexer.ExcelLexer().Ping());
    }
}