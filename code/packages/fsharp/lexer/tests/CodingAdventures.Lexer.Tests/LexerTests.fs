namespace CodingAdventures.Lexer.FSharp.Tests

open CodingAdventures.Lexer.FSharp
open Xunit

module LexerTests =
    [<Fact>]
    let ``lexer tokenizes grammar driven input`` () =
        let grammar =
            Lexer.parseGrammar """
                NUMBER = /[0-9]+/
                PLUS = "+"
                skip:
                  WS = /[ \t]+/
                """

        let tokens = Lexer.tokenize "42 + 7" grammar
        Assert.Equal("NUMBER", tokens[0].TypeName)
        Assert.Equal("PLUS", tokens[1].TypeName)
        Assert.Equal("NUMBER", tokens[2].TypeName)

    [<Fact>]
    let ``lexer promotes keywords and preserves newline flags`` () =
        let grammar =
            Lexer.parseGrammar """
                NAME = /[a-z]+/
                skip:
                  WS = /[ \t]+/
                  NL = /\n/
                keywords:
                  if
                """

        let tokens = Lexer.tokenize "if\nname" grammar
        Assert.Equal(TokenType.Keyword, tokens[0].Type)
        Assert.True(tokens[1].HasFlag(Token.FlagPrecededByNewline))
