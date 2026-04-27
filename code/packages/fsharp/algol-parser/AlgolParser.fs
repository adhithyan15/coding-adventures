namespace CodingAdventures.AlgolParser.FSharp

open System
open System.Collections.Concurrent
open System.IO
open System.Text
open CodingAdventures.GrammarTools.FSharp
open CodingAdventures.Parser.FSharp
open CodingAdventures.AlgolLexer.FSharp

/// ALGOL parser backed by shared grammar definitions.
type AlgolParser private () =
    static let parserGrammars = ConcurrentDictionary<string, ParserGrammar>()

    static member DefaultVersion = AlgolLexer.DefaultVersion
    static member SupportedVersions = AlgolLexer.SupportedVersions

    static member CreateAlgolParser() = AlgolParser.CreateAlgolParser(AlgolParser.DefaultVersion)
    static member CreateAlgolParser(version: string) = GrammarParser(AlgolParser.LoadParserGrammar(version))

    static member ParseAlgol(source: string) = AlgolParser.ParseAlgol(source, AlgolParser.DefaultVersion)
    static member ParseAlgol(source: string, version: string) =
        try
            AlgolParser.CreateAlgolParser(version).Parse(AlgolLexer.TokenizeAlgol(source, version))
        with
        | :? GrammarParseError as error ->
            raise (ArgumentException(sprintf "ALGOL parse failed: %s" error.Message, "source", error))

    static member private LoadParserGrammar(version: string) =
        let validated = AlgolParser.ValidateVersion(version)
        parserGrammars.GetOrAdd(validated, Func<string, ParserGrammar>(AlgolParser.ParseParserGrammarResource))

    static member private ValidateVersion(version: string) =
        if String.IsNullOrWhiteSpace(version) then
            AlgolParser.DefaultVersion
        elif AlgolParser.SupportedVersions |> Seq.exists ((=) version) then
            version
        else
            invalidArg "version" (sprintf "Unknown ALGOL version '%s'. Valid values: %s" version (String.Join(", ", AlgolParser.SupportedVersions)))

    static member private ParseParserGrammarResource(version: string) =
        try
            ParserGrammarParser.Parse(AlgolParser.ReadResource(version + ".grammar"))
        with
        | :? ParserGrammarError as error ->
            raise (InvalidOperationException(sprintf "Failed to parse bundled ALGOL parser grammar for version %s" version, error))

    static member private ReadResource(resourceName: string) =
        let assembly = typeof<AlgolParser>.Assembly
        use stream = assembly.GetManifestResourceStream(resourceName)
        if isNull stream then
            invalidOp (sprintf "Missing bundled resource: %s" resourceName)
        use reader = new StreamReader(stream, Encoding.UTF8)
        reader.ReadToEnd()
