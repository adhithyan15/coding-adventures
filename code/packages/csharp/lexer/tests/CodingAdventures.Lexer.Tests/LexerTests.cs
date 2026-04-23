using CodingAdventures.GrammarTools;
using CodingAdventures.Lexer;

namespace CodingAdventures.Lexer.Tests;

public class LexerTests
{
    [Fact]
    public void GrammarLexer_TokenizesSimpleExpression()
    {
        var grammar = TokenGrammarParser.Parse("""
            NUMBER = /[0-9]+/
            PLUS = "+"
            skip:
              WS = /[ \t]+/
            """);

        var lexer = new GrammarLexer(grammar);
        var tokens = lexer.Tokenize("42 + 7");

        Assert.Equal("NUMBER", tokens[0].TypeName);
        Assert.Equal("PLUS", tokens[1].TypeName);
        Assert.Equal("NUMBER", tokens[2].TypeName);
        Assert.Equal("EOF", tokens[^1].TypeName);
    }
}
