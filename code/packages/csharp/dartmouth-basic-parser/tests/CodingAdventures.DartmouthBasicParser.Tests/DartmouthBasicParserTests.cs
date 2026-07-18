using CodingAdventures.Parser;
using BasicLexer = CodingAdventures.DartmouthBasicLexer.DartmouthBasicLexer;
using BasicParser = CodingAdventures.DartmouthBasicParser.DartmouthBasicParser;

namespace CodingAdventures.DartmouthBasicParser.Tests;

public sealed class DartmouthBasicParserTests
{
    [Fact]
    public void ParsesTheCompleteStatementAndExpressionSurface()
    {
        const string source = """
            10 LET X = FNA(SIN(-X)) ^ 2 + A(1) / 3
            20 PRINT "HELLO", X;
            30 INPUT A, B
            40 IF X >= 0 THEN 100
            50 GOTO 100
            60 GOSUB 200
            70 RETURN
            80 FOR I = 1 TO 10 STEP 2
            90 NEXT I
            100 STOP
            110 REM THIS BODY IS IGNORED @@@
            120 READ A, B
            130 DATA 1, 2, 3
            140 RESTORE
            150 DIM A(10), B(2, 3)
            160 DEF FNA(T) = T * T
            170 END
            """ + "\n";

        var ast = BasicParser.ParseDartmouthBasic(source);
        var rules = Descendants(ast).Select(node => node.RuleName).ToHashSet();

        Assert.Equal("program", ast.RuleName);
        Assert.True(ast.DescendantCount() > 100);
        Assert.Contains("let_stmt", rules);
        Assert.Contains("print_stmt", rules);
        Assert.Contains("if_stmt", rules);
        Assert.Contains("for_stmt", rules);
        Assert.Contains("data_stmt", rules);
        Assert.Contains("dim_stmt", rules);
        Assert.Contains("def_stmt", rules);
    }

    [Fact]
    public void ConfiguredParserParsesBareAndEmptyPrograms()
    {
        Assert.Equal("program", BasicParser.CreateDartmouthBasicParser("10\n").Parse().RuleName);
        Assert.Equal("program", BasicParser.ParseDartmouthBasic(string.Empty).RuleName);
        Assert.Equal("program", BasicParser.ParseTokens(BasicLexer.TokenizeDartmouthBasic("20 END\n")).RuleName);
    }

    [Theory]
    [InlineData("10 LET X 5\n")]
    [InlineData("10 IF X > 0 100\n")]
    [InlineData("10 FOR I = 1\n")]
    [InlineData("10 END @\n")]
    public void RejectsMalformedOrUnconsumedInput(string source)
    {
        var error = Assert.Throws<ArgumentException>(() => BasicParser.ParseDartmouthBasic(source));
        Assert.Contains("Dartmouth BASIC parse failed", error.Message);
    }

    [Fact]
    public void RejectsNullSource() =>
        Assert.Multiple(
            () => Assert.Throws<ArgumentNullException>(() => BasicParser.CreateDartmouthBasicParser(null!)),
            () => Assert.Throws<ArgumentNullException>(() => BasicParser.ParseTokens(null!)));

    [Fact]
    public void TokenApiRequiresFinalEof()
    {
        var tokens = BasicLexer.TokenizeDartmouthBasic("20 END\n").SkipLast(1).ToArray();
        Assert.Throws<ArgumentException>(() => BasicParser.ParseTokens(tokens));
        Assert.Throws<ArgumentException>(() => BasicParser.ParseTokens([]));
    }

    private static IEnumerable<ASTNode> Descendants(ASTNode node)
    {
        yield return node;
        foreach (var child in node.Children.OfType<ASTNode>())
        {
            foreach (var descendant in Descendants(child))
            {
                yield return descendant;
            }
        }
    }
}
