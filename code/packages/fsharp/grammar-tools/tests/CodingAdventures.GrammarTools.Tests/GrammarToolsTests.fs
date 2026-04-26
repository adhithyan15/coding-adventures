namespace CodingAdventures.GrammarTools.FSharp.Tests

open System
open System.Collections.Generic
open CodingAdventures.GrammarTools.FSharp
open Xunit

module GrammarToolsTests =
    [<Fact>]
    let ``token grammar parser handles sections metadata and groups`` () =
        let grammar =
            TokenGrammarParser.Parse """
                # @version 3
                # @case_insensitive true
                mode: default
                escape_mode: string
                NUMBER = /[0-9]+/
                NAME = /[a-z]+/ -> IDENT
                group punctuation:
                  PLUS = "+"
                skip:
                  WS = /[ \t]+/
                keywords:
                  if
                reserved:
                  class
                context_keywords:
                  await
                soft_keywords:
                  from
                errors:
                  BAD = /.+/
                """

        Assert.Equal(2, grammar.Definitions.Count)
        Assert.Equal("default", grammar.Mode)
        Assert.Equal("string", grammar.EscapeMode)
        Assert.False(grammar.CaseSensitive)
        Assert.True(grammar.CaseInsensitive)
        Assert.Equal(3, grammar.Version)
        Assert.Equal("IDENT", grammar.Definitions[1].Alias)
        Assert.Single(grammar.SkipDefinitions) |> ignore
        Assert.Single(grammar.Keywords) |> ignore
        Assert.Single(grammar.ReservedKeywords) |> ignore
        Assert.Single(grammar.ContextKeywords) |> ignore
        Assert.Single(grammar.SoftKeywords) |> ignore
        Assert.Single(grammar.ErrorDefinitions) |> ignore
        Assert.True(grammar.Groups.ContainsKey("punctuation"))

    [<Fact>]
    let ``token grammar parser allows arrow literal`` () =
        let grammar =
            TokenGrammarParser.Parse "escapes: none\ncase_sensitive: false\nskip:\n  WS = /[ \t]+/\nTRIGGER = \"->\"\nNAME = /[a-z]+/ -> IDENT"

        Assert.Equal("none", grammar.EscapeMode)
        Assert.False(grammar.CaseSensitive)
        Assert.True(grammar.CaseInsensitive)
        Assert.Single(grammar.SkipDefinitions) |> ignore
        Assert.Equal("->", grammar.Definitions[0].Pattern)
        Assert.Null(grammar.Definitions[0].Alias)
        Assert.Equal("IDENT", grammar.Definitions[1].Alias)

    [<Fact>]
    let ``token grammar parser rejects malformed definitions`` () =
        let error =
            Assert.Throws<TokenGrammarError>(fun () ->
                TokenGrammarParser.Parse "NAME = nope" |> ignore)

        Assert.Contains("Pattern must be", error.Message)

    [<Fact>]
    let ``token grammar validator rejects duplicate definitions`` () =
        let grammar =
            TokenGrammar(
                [|
                    TokenDefinition("NAME", "[a-z]+", true, 1)
                    TokenDefinition("NAME", "[0-9]+", true, 2)
                |]
                :> IReadOnlyList<TokenDefinition>,
                Array.Empty<string>() :> IReadOnlyList<string>)

        let error =
            Assert.Throws<InvalidOperationException>(fun () ->
                TokenGrammarValidator.Validate(grammar))

        Assert.Contains("Duplicate token definition", error.Message)

    [<Fact>]
    let ``parser grammar parser handles core grammar forms`` () =
        let grammar =
            ParserGrammarParser.Parse """
                program = { statement } ;
                statement = NAME | NUMBER | [ STRING ] | &NAME NAME | !PLUS NUMBER | { NUMBER // COMMA }+ | ( NAME ) ;
                """

        Assert.Equal(2, grammar.Rules.Count)
        Assert.Equal("program", grammar.Rules[0].Name)

        match grammar.Rules[1].Body with
        | :? Alternation as alternation ->
            Assert.True(alternation.Choices.Count >= 6)

            Assert.Contains(
                alternation.Choices,
                fun choice ->
                    match choice with
                    | :? SeparatedRepetition as repetition -> repetition.AtLeastOne
                    | _ -> false)
        | _ -> Assert.Fail("Expected alternation body")

    [<Fact>]
    let ``parser grammar parser reports unexpected characters`` () =
        let error =
            Assert.Throws<ParserGrammarError>(fun () ->
                ParserGrammarParser.Parse "rule = NAME @ NUMBER ;" |> ignore)

        Assert.Contains("Unexpected character '@'", error.Message)

    [<Fact>]
    let ``parser grammar validator rejects duplicate rules`` () =
        let rules =
            [|
                GrammarRule("program", RuleReference("NAME", true) :> GrammarElement)
                GrammarRule("program", RuleReference("NUMBER", true) :> GrammarElement)
            |]

        let error =
            Assert.Throws<InvalidOperationException>(fun () ->
                ParserGrammarValidator.Validate(ParserGrammar(rules :> IReadOnlyList<GrammarRule>)))

        Assert.Contains("Duplicate grammar rule", error.Message)

    [<Fact>]
    let ``cross validator accepts keyword references`` () =
        let tokenGrammar =
            TokenGrammar(
                [| TokenDefinition("NAME", "[a-z]+", true, 1) |] :> IReadOnlyList<TokenDefinition>,
                [| "if" |] :> IReadOnlyList<string>)

        let parserGrammar =
            ParserGrammar(
                [|
                    GrammarRule(
                        "program",
                        Sequence(
                            [|
                                RuleReference("KEYWORD", true) :> GrammarElement
                                RuleReference("NAME", true) :> GrammarElement
                            |]
                            :> IReadOnlyList<GrammarElement>)
                        :> GrammarElement)
                |]
                :> IReadOnlyList<GrammarRule>)

        CrossValidator.Validate(tokenGrammar, parserGrammar)

    [<Fact>]
    let ``cross validator rejects unknown token references`` () =
        let tokenGrammar =
            TokenGrammar(
                [| TokenDefinition("NAME", "[a-z]+", true, 1) |] :> IReadOnlyList<TokenDefinition>,
                Array.Empty<string>() :> IReadOnlyList<string>)

        let parserGrammar =
            ParserGrammar(
                [| GrammarRule("program", RuleReference("NUMBER", true) :> GrammarElement) |]
                :> IReadOnlyList<GrammarRule>)

        let error =
            Assert.Throws<InvalidOperationException>(fun () ->
                CrossValidator.Validate(tokenGrammar, parserGrammar))

        Assert.Contains("Unknown token reference", error.Message)
