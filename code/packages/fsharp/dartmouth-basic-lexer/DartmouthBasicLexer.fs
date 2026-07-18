namespace CodingAdventures.DartmouthBasicLexer.FSharp

open System
open System.Collections.Generic
open System.IO
open System.Reflection
open System.Text
open CodingAdventures.GrammarTools.FSharp
open CodingAdventures.Lexer.FSharp

module private Implementation =
    let grammarResource = "dartmouth_basic.tokens"

    let prepareGrammar (source: string) =
        let output = ResizeArray<string>()
        let keywords = ResizeArray<string>()
        let mutable readingKeywords = false

        for rawLine in source.Replace("\r\n", "\n").Split('\n') do
            let trimmed = rawLine.Trim()
            if trimmed = "keywords:" then
                readingKeywords <- true
            elif readingKeywords && trimmed.Length > 0 && not (trimmed.StartsWith('#')) && Char.IsWhiteSpace(rawLine.[0]) then
                keywords.Add(trimmed.ToLowerInvariant())
            else
                if readingKeywords && trimmed.Length > 0 && not (trimmed.StartsWith('#')) then
                    readingKeywords <- false
                output.Add(rawLine)

        output.Add("keywords:")
        keywords |> Seq.map (fun keyword -> "  " + keyword) |> output.AddRange
        String.Join("\n", output)

    let tokenGrammar =
        lazy
            try
                let assembly = Assembly.GetExecutingAssembly()
                use stream = assembly.GetManifestResourceStream(grammarResource)
                if isNull stream then
                    invalidOp ("Missing bundled resource: " + grammarResource)
                use reader = new StreamReader(stream, Encoding.UTF8)
                TokenGrammarParser.Parse(prepareGrammar (reader.ReadToEnd()))
            with
            | :? TokenGrammarError as error ->
                raise (InvalidOperationException("Failed to parse bundled Dartmouth BASIC token grammar", error))

    let normalizeValue (token: Token) =
        let value =
            match token.EffectiveTypeName with
            | "KEYWORD" -> token.Value.ToUpperInvariant()
            | "BUILTIN_FN" | "USER_FN" | "NAME" | "NUMBER" | "LINE_NUM" -> token.Value.ToLowerInvariant()
            | "STRING" when token.Value.Length >= 2 && token.Value.[0] = '"' && token.Value.[token.Value.Length - 1] = '"' ->
                token.Value.Substring(1, token.Value.Length - 2)
            | "NEWLINE" -> "\\n"
            | _ -> token.Value
        Token(token.Type, value, token.Line, token.Column, token.TypeName, token.Flags)

    let postProcess (tokens: IReadOnlyList<Token>) =
        let result = ResizeArray<Token>(tokens.Count)
        let mutable atLineStart = true
        let mutable suppressingRemark = false

        for original in tokens do
            let normalized = normalizeValue original
            let token =
                if atLineStart && normalized.EffectiveTypeName = "NUMBER" then
                    Token(normalized.Type, normalized.Value, normalized.Line, normalized.Column, "LINE_NUM", normalized.Flags)
                else
                    normalized

            if atLineStart then
                atLineStart <- false

            if not suppressingRemark || token.EffectiveTypeName = "NEWLINE" then
                result.Add(token)

            if token.EffectiveTypeName = "KEYWORD" && token.Value = "REM" then
                suppressingRemark <- true
            elif token.EffectiveTypeName = "NEWLINE" then
                suppressingRemark <- false
                atLineStart <- true

        result |> Seq.toList :> IReadOnlyList<Token>

/// Tokenizes the original 1964 Dartmouth BASIC language.
type DartmouthBasicLexer private (source: string) =
    /// Produces the normalized, parser-ready token stream.
    member _.Tokenize() =
        try
            GrammarLexer(Implementation.tokenGrammar.Value).Tokenize(source)
            |> fun tokens -> Implementation.postProcess (tokens :> IReadOnlyList<Token>)
        with
        | :? LexerError as error ->
            raise (ArgumentException("Dartmouth BASIC tokenization failed: " + error.Message, "source", error))

    /// Creates a configured lexer for source.
    static member CreateDartmouthBasicLexer(source: string) =
        if isNull source then nullArg "source"
        DartmouthBasicLexer(source)

    /// Tokenizes source in one call.
    static member TokenizeDartmouthBasic(source: string) =
        DartmouthBasicLexer.CreateDartmouthBasicLexer(source).Tokenize()
