namespace CodingAdventures.AlgolLexer.FSharp

open System
open System.Collections.Concurrent
open System.Collections.Generic
open System.IO
open System.Text
open CodingAdventures.GrammarTools.FSharp
open CodingAdventures.Lexer.FSharp

/// ALGOL lexer backed by shared grammar token definitions.
type AlgolLexer private () =
    static let defaultVersion = "algol60"
    static let supportedVersions = [| "algol60" |]
    static let tokenGrammars = ConcurrentDictionary<string, TokenGrammar>()

    static member DefaultVersion = defaultVersion
    static member SupportedVersions = supportedVersions :> IReadOnlyList<string>

    static member CreateAlgolLexer() = AlgolLexer.CreateAlgolLexer(defaultVersion)
    static member CreateAlgolLexer(version: string) = GrammarLexer(AlgolLexer.LoadTokenGrammar(version))

    static member TokenizeAlgol(source: string) = AlgolLexer.TokenizeAlgol(source, defaultVersion)
    static member TokenizeAlgol(source: string, version: string) =
        try
            AlgolLexer.CreateAlgolLexer(version).Tokenize(source)
        with
        | :? LexerError as error ->
            raise (ArgumentException(sprintf "ALGOL tokenization failed: %s" error.Message, "source", error))

    static member private LoadTokenGrammar(version: string) =
        let validated = AlgolLexer.ValidateVersion(version)
        tokenGrammars.GetOrAdd(validated, Func<string, TokenGrammar>(AlgolLexer.ParseTokenGrammarResource))

    static member private ValidateVersion(version: string) =
        if String.IsNullOrWhiteSpace(version) then
            defaultVersion
        elif Array.exists ((=) version) supportedVersions then
            version
        else
            invalidArg "version" (sprintf "Unknown ALGOL version '%s'. Valid values: %s" version (String.Join(", ", supportedVersions)))

    static member private ParseTokenGrammarResource(version: string) =
        try
            TokenGrammarParser.Parse(AlgolLexer.ReadResource(version + ".tokens"))
        with
        | :? TokenGrammarError as error ->
            raise (InvalidOperationException(sprintf "Failed to parse bundled ALGOL token grammar for version %s" version, error))

    static member private ReadResource(resourceName: string) =
        let assembly = typeof<AlgolLexer>.Assembly
        use stream = assembly.GetManifestResourceStream(resourceName)
        if isNull stream then
            invalidOp (sprintf "Missing bundled resource: %s" resourceName)
        use reader = new StreamReader(stream, Encoding.UTF8)
        reader.ReadToEnd()
