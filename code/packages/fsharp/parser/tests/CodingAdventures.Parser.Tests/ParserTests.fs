namespace CodingAdventures.Parser.FSharp.Tests

open System.Collections.Generic
open CodingAdventures.Lexer.FSharp
open CodingAdventures.Parser.FSharp
open Xunit

module ParserTests =
    [<Fact>]
    let ``parser builds an AST from tokens`` () =
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
        Assert.Equal(1, ast.StartLine)
        Assert.True(ast.DescendantCount() >= 3)

    [<Fact>]
    let ``parser handles repetition and optionals`` () =
        let grammar = Parser.parseGrammar "program = { NAME } [ NUMBER ] ;"
        let tokens =
            [|
                Token(TokenType.Grammar, "alpha", 1, 1, "NAME")
                Token(TokenType.Grammar, "beta", 1, 7, "NAME")
                Token(TokenType.Grammar, "7", 1, 12, "NUMBER")
                Token(TokenType.EOF, "", 1, 13, "EOF")
            |]

        let ast = Parser.parse tokens grammar
        Assert.Equal("program", ast.RuleName)
        Assert.True(ast.DescendantCount() >= 3)
