namespace CodingAdventures.Lexer.FSharp

open CodingAdventures.GrammarTools
open CodingAdventures.Lexer

module Lexer =
    let parseGrammar source =
        TokenGrammarParser.Parse(source)

    let tokenize source grammar =
        GrammarLexer(grammar).Tokenize(source) |> Seq.toList
