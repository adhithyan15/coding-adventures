using CodingAdventures.GrammarTools;

namespace CodingAdventures.GrammarTools.Tests;

public class GrammarToolsTests
{
    [Fact]
    public void TokenGrammarParser_ParsesSections()
    {
        var grammar = TokenGrammarParser.Parse("""
            NUMBER = /[0-9]+/
            PLUS = "+"
            skip:
              WS = /[ \t]+/
            keywords:
              if
            reserved:
              class
            errors:
              BAD = /.+/
            """);

        Assert.Equal(2, grammar.Definitions.Count);
        Assert.Single(grammar.SkipDefinitions!);
        Assert.Single(grammar.Keywords);
        Assert.Single(grammar.ReservedKeywords!);
        Assert.Single(grammar.ErrorDefinitions!);
    }

    [Fact]
    public void ParserGrammarParser_ParsesCoreForms()
    {
        var grammar = ParserGrammarParser.Parse("""
            program = { statement } ;
            statement = NAME | NUMBER | [ STRING ] | &NAME NAME | !PLUS NUMBER | { NUMBER // COMMA } ;
            """);

        Assert.Equal(2, grammar.Rules.Count);
        Assert.Equal("program", grammar.Rules[0].Name);
        Assert.Equal("statement", grammar.Rules[1].Name);
    }
}
