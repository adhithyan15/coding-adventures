using CodingAdventures.Lexer;
using BasicLexer = CodingAdventures.DartmouthBasicLexer.DartmouthBasicLexer;

namespace CodingAdventures.DartmouthBasicLexer.Tests;

public sealed class DartmouthBasicLexerTests
{
    [Fact]
    public void TokenizesAndNormalizesAStatement()
    {
        var tokens = BasicLexer.TokenizeDartmouthBasic("10 let X = 1.5E3\r\n");

        Assert.Equal(
            ["LINE_NUM", "KEYWORD", "NAME", "EQ", "NUMBER", "NEWLINE", "EOF"],
            tokens.Select(token => token.EffectiveTypeName));
        Assert.Equal(["10", "LET", "x", "=", "1.5e3", "\\n", ""], tokens.Select(token => token.Value));
        Assert.Equal((1, 1), (tokens[0].Line, tokens[0].Column));
        Assert.Equal((2, 1), (tokens[^1].Line, tokens[^1].Column));
    }

    [Fact]
    public void RelabelsOnlyTheFirstNumberOnEachPhysicalLine()
    {
        var tokens = BasicLexer.TokenizeDartmouthBasic("30 GOTO 10\n40 PRINT 20\n");
        Assert.Equal(["LINE_NUM", "NUMBER", "LINE_NUM", "NUMBER"],
            tokens.Where(token => token.EffectiveTypeName is "LINE_NUM" or "NUMBER").Select(token => token.EffectiveTypeName));
    }

    [Fact]
    public void SuppressesRemarkBodyButRetainsItsNewline()
    {
        var tokens = BasicLexer.CreateDartmouthBasicLexer("10 rem GOTO 20 @ ignored\n20 END\n").Tokenize();
        Assert.Equal(
            ["LINE_NUM", "KEYWORD", "NEWLINE", "LINE_NUM", "KEYWORD", "NEWLINE", "EOF"],
            tokens.Select(token => token.EffectiveTypeName));
        Assert.Equal("REM", tokens[1].Value);
    }

    [Fact]
    public void PreservesStringCaseWithoutQuotes()
    {
        var tokens = BasicLexer.TokenizeDartmouthBasic("10 PRINT \"Hello, World!\"\n");
        var value = Assert.Single(tokens, token => token.EffectiveTypeName == "STRING").Value;
        Assert.Equal("Hello, World!", value);
    }

    [Fact]
    public void ClassifiesFunctionsNamesAndUnknownCharacters()
    {
        var tokens = BasicLexer.TokenizeDartmouthBasic("10 LET Result = SIN(FNA(X)) @\n");
        Assert.Contains(tokens, token => token.EffectiveTypeName == "BUILTIN_FN" && token.Value == "sin");
        Assert.Contains(tokens, token => token.EffectiveTypeName == "USER_FN" && token.Value == "fna");
        Assert.Contains(tokens, token => token.EffectiveTypeName == "NAME" && token.Value == "result");
        Assert.Contains(tokens, token => token.EffectiveTypeName == "UNKNOWN" && token.Value == "@");
    }

    [Theory]
    [InlineData("<=", "LE")]
    [InlineData(">=", "GE")]
    [InlineData("<>", "NE")]
    [InlineData("^", "CARET")]
    [InlineData(";", "SEMICOLON")]
    public void RecognizesOperators(string source, string expectedType)
    {
        Assert.Equal(expectedType, BasicLexer.TokenizeDartmouthBasic(source)[0].EffectiveTypeName);
    }

    [Fact]
    public void ABlankLineDoesNotRelabelTheFollowingExpressionNumberIncorrectly()
    {
        var tokens = BasicLexer.TokenizeDartmouthBasic("\n10 END\n");
        Assert.Equal("NEWLINE", tokens[0].EffectiveTypeName);
        Assert.Equal("LINE_NUM", tokens[1].EffectiveTypeName);
    }

    [Fact]
    public void RejectsNullSource() =>
        Assert.Throws<ArgumentNullException>(() => BasicLexer.CreateDartmouthBasicLexer(null!));
}
