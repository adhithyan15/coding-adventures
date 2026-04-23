using CodingAdventures.GrammarTools;
using CodingAdventures.Lexer;
using CodingAdventures.Parser;

namespace CodingAdventures.Parser.Tests;

public class ParserTests
{
    [Fact]
    public void GrammarParser_ParsesSequence()
    {
        var grammar = ParserGrammarParser.Parse("assign = NAME EQUALS NUMBER ;");
        var parser = new GrammarParser(grammar);
        var tokens = new List<Token>
        {
            new(TokenType.Grammar, "x", 1, 1, "NAME"),
            new(TokenType.Grammar, "=", 1, 2, "EQUALS"),
            new(TokenType.Grammar, "42", 1, 3, "NUMBER"),
            new(TokenType.EOF, string.Empty, 1, 5, "EOF"),
        };

        var ast = parser.Parse(tokens);
        Assert.Equal("assign", ast.RuleName);
        Assert.Equal(3, ast.Children.Count);
    }
}
