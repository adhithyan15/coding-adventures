namespace CodingAdventures.Parser.FSharp

open CodingAdventures.GrammarTools
open CodingAdventures.Lexer
open CodingAdventures.Parser

module Parser =
    let parseGrammar source =
        ParserGrammarParser.Parse(source)

    let parse tokens grammar =
        GrammarParser(grammar).Parse(tokens)
