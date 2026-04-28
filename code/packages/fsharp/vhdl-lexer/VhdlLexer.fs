namespace CodingAdventures.VhdlLexer.FSharp

open System
open System.Collections.Concurrent
open System.Collections.Generic
open System.IO
open System.Text
open CodingAdventures.GrammarTools.FSharp
open CodingAdventures.Lexer.FSharp

/// VHDL lexer backed by shared grammar token definitions.
type VhdlLexer private () =
    static let defaultVersion = "2008"
    static let supportedVersions = [| "1987"; "1993"; "2002"; "2008"; "2019" |]
    static let tokenGrammars = ConcurrentDictionary<string, TokenGrammar>()

    static member DefaultVersion = defaultVersion
    static member SupportedVersions = supportedVersions :> IReadOnlyList<string>

    static member CreateVhdlLexer() = VhdlLexer.CreateVhdlLexer(defaultVersion)
    static member CreateVhdlLexer(version: string) = GrammarLexer(VhdlLexer.LoadTokenGrammar(version))

    static member TokenizeVhdl(source: string) = VhdlLexer.TokenizeVhdl(source, defaultVersion)
    static member TokenizeVhdl(source: string, version: string) =
        let grammar = VhdlLexer.LoadTokenGrammar(version)
        try
            let tokens = GrammarLexer(grammar).Tokenize(source)
            VhdlLexer.NormalizeCase(tokens, HashSet<string>(grammar.Keywords, StringComparer.Ordinal))
        with
        | :? LexerError as error ->
            raise (ArgumentException(sprintf "VHDL tokenization failed: %s" error.Message, "source", error))

    static member private NormalizeCase(tokens: IReadOnlyList<Token>, keywords: ISet<string>) =
        tokens
        |> Seq.map (fun token ->
            let normalizeKeyword = token.Type = TokenType.Keyword
            let normalizeName = token.Type = TokenType.Grammar && token.TypeName = "NAME"
            if not normalizeKeyword && not normalizeName then
                token
            else
                let lowered = token.Value.ToLowerInvariant()
                let mutable normalizedType = if normalizeKeyword then TokenType.Keyword else token.Type
                let mutable normalizedTypeName = if normalizeKeyword then "KEYWORD" else token.TypeName
                if normalizeName && keywords.Contains(lowered) then
                    normalizedType <- TokenType.Keyword
                    normalizedTypeName <- "KEYWORD"
                Token(normalizedType, lowered, token.Line, token.Column, normalizedTypeName, token.Flags))
        |> ResizeArray<Token>
        :> IReadOnlyList<Token>

    static member private LoadTokenGrammar(version: string) =
        let validated = VhdlLexer.ValidateVersion(version)
        tokenGrammars.GetOrAdd(validated, Func<string, TokenGrammar>(VhdlLexer.ParseTokenGrammarResource))

    static member private ValidateVersion(version: string) =
        if String.IsNullOrWhiteSpace(version) then
            defaultVersion
        elif Array.exists ((=) version) supportedVersions then
            version
        else
            invalidArg "version" (sprintf "Unknown VHDL version '%s'. Valid values: %s" version (String.Join(", ", supportedVersions)))

    static member private ParseTokenGrammarResource(version: string) =
        try
            TokenGrammarParser.Parse(VhdlLexer.ReadResource("vhdl" + version + ".tokens"))
        with
        | :? TokenGrammarError as error ->
            raise (InvalidOperationException(sprintf "Failed to parse bundled VHDL token grammar for version %s" version, error))

    static member private ReadResource(resourceName: string) =
        let assembly = typeof<VhdlLexer>.Assembly
        use stream = assembly.GetManifestResourceStream(resourceName)
        if isNull stream then
            invalidOp (sprintf "Missing bundled resource: %s" resourceName)
        use reader = new StreamReader(stream, Encoding.UTF8)
        reader.ReadToEnd()
