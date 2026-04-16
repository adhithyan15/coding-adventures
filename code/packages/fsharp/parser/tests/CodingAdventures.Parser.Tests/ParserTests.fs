namespace CodingAdventures.Parser.FSharp.Tests

open System.Collections.Generic
open CodingAdventures.Lexer
open CodingAdventures.Parser.FSharp
open Xunit

module ParserTests =
    [<Fact>]
    let ``parser wrapper parses`` () =
        let grammar = Parser.parseGrammar "assign = NAME EQUALS NUMBER ;"
        let tokens =
            [|
                Token(TokenType.Grammar, "x", 1, 1, "NAME")
                Token(TokenType.Grammar, "=", 1, 2, "EQUALS")
                Token(TokenType.Grammar, "42", 1, 3, "NUMBER")
                Token(TokenType.EOF, "", 1, 5, "EOF")
            |]

        let ast = Parser.parse tokens grammar
        Assert.Equal("assign", ast.RuleName)
