namespace CodingAdventures.DartmouthBasicParser.FSharp

open System
open System.Collections.Generic
open System.IO
open System.Reflection
open System.Text
open CodingAdventures.GrammarTools.FSharp
open CodingAdventures.Lexer.FSharp
open CodingAdventures.Parser.FSharp
open CodingAdventures.DartmouthBasicLexer.FSharp

module private Implementation =
    let grammarResource = "dartmouth_basic.grammar"

    let parserGrammar =
        lazy
            try
                let assembly = Assembly.GetExecutingAssembly()
                use stream = assembly.GetManifestResourceStream(grammarResource)
                if isNull stream then
                    invalidOp ("Missing bundled resource: " + grammarResource)
                use reader = new StreamReader(stream, Encoding.UTF8)
                ParserGrammarParser.Parse(reader.ReadToEnd())
            with
            | :? ParserGrammarError as error ->
                raise (InvalidOperationException("Failed to parse bundled Dartmouth BASIC grammar", error))

    let rec countTokens (node: ASTNode) =
        node.Children
        |> Seq.sumBy (fun child ->
            match child with
            | :? Token -> 1
            | :? ASTNode as nested -> countTokens nested
            | _ -> 0)

/// Parses original 1964 Dartmouth BASIC source into a grammar-shaped AST.
type DartmouthBasicParser private (tokens: IReadOnlyList<Token>) =
    /// Parses the configured token stream and requires complete input consumption.
    member _.Parse() =
        try
            let ast = GrammarParser(Implementation.parserGrammar.Value).Parse(tokens)
            if tokens.Count = 0 then
                raise (GrammarParseError("Token stream must end with EOF"))
            elif tokens.[tokens.Count - 1].EffectiveTypeName <> "EOF" then
                raise (GrammarParseError("Token stream must end with EOF", tokens.[tokens.Count - 1]))

            let eofIndex = tokens.Count - 1
            let parsedTokenCount = Implementation.countTokens ast

            if parsedTokenCount <> eofIndex then
                let token =
                    if parsedTokenCount < tokens.Count then tokens.[parsedTokenCount]
                    else Unchecked.defaultof<Token>
                raise (GrammarParseError("Unexpected token while parsing program", token))

            ast
        with
        | :? GrammarParseError as error ->
            raise (ArgumentException("Dartmouth BASIC parse failed: " + error.Message, "source", error))

    /// Tokenizes source and creates a configured parser.
    static member CreateDartmouthBasicParser(source: string) =
        DartmouthBasicLexer.TokenizeDartmouthBasic(source)
        |> DartmouthBasicParser

    /// Parses an existing Dartmouth BASIC token stream.
    static member ParseTokens(tokens: IReadOnlyList<Token>) =
        if isNull tokens then nullArg "tokens"
        DartmouthBasicParser(tokens).Parse()

    /// Tokenizes and parses source in one call.
    static member ParseDartmouthBasic(source: string) =
        DartmouthBasicParser.CreateDartmouthBasicParser(source).Parse()
