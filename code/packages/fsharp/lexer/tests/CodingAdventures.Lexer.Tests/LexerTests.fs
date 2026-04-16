namespace CodingAdventures.Lexer.FSharp.Tests

open CodingAdventures.Lexer.FSharp
open Xunit

module LexerTests =
    [<Fact>]
    let ``lexer wrapper tokenizes`` () =
        let grammar =
            Lexer.parseGrammar """
                NUMBER = /[0-9]+/
                PLUS = "+"
                skip:
                  WS = /[ \t]+/
                """

        let tokens = Lexer.tokenize "42 + 7" grammar
        Assert.Equal("NUMBER", tokens[0].TypeName)
