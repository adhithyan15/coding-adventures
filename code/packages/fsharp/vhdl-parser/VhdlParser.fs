namespace CodingAdventures.VhdlParser.FSharp

open System
open System.Collections.Concurrent
open System.IO
open System.Text
open CodingAdventures.GrammarTools.FSharp
open CodingAdventures.Parser.FSharp
open CodingAdventures.VhdlLexer.FSharp

/// VHDL parser backed by shared grammar definitions.
type VhdlParser private () =
    static let parserGrammars = ConcurrentDictionary<string, ParserGrammar>()

    static member DefaultVersion = VhdlLexer.DefaultVersion
    static member SupportedVersions = VhdlLexer.SupportedVersions

    static member CreateVhdlParser() = VhdlParser.CreateVhdlParser(VhdlParser.DefaultVersion)
    static member CreateVhdlParser(version: string) = GrammarParser(VhdlParser.LoadParserGrammar(version))

    static member ParseVhdl(source: string) = VhdlParser.ParseVhdl(source, VhdlParser.DefaultVersion)
    static member ParseVhdl(source: string, version: string) =
        try
            VhdlParser.CreateVhdlParser(version).Parse(VhdlLexer.TokenizeVhdl(source, version))
        with
        | :? GrammarParseError as error ->
            raise (ArgumentException(sprintf "VHDL parse failed: %s" error.Message, "source", error))

    static member private LoadParserGrammar(version: string) =
        let validated = VhdlParser.ValidateVersion(version)
        parserGrammars.GetOrAdd(validated, Func<string, ParserGrammar>(VhdlParser.ParseParserGrammarResource))

    static member private ValidateVersion(version: string) =
        if String.IsNullOrWhiteSpace(version) then
            VhdlParser.DefaultVersion
        elif VhdlParser.SupportedVersions |> Seq.exists ((=) version) then
            version
        else
            invalidArg "version" (sprintf "Unknown VHDL version '%s'. Valid values: %s" version (String.Join(", ", VhdlParser.SupportedVersions)))

    static member private ParseParserGrammarResource(version: string) =
        try
            ParserGrammarParser.Parse(VhdlParser.ReadResource("vhdl" + version + ".grammar"))
        with
        | :? ParserGrammarError as error ->
            raise (InvalidOperationException(sprintf "Failed to parse bundled VHDL parser grammar for version %s" version, error))

    static member private ReadResource(resourceName: string) =
        let assembly = typeof<VhdlParser>.Assembly
        use stream = assembly.GetManifestResourceStream(resourceName)
        if isNull stream then
            invalidOp (sprintf "Missing bundled resource: %s" resourceName)
        use reader = new StreamReader(stream, Encoding.UTF8)
        reader.ReadToEnd()
