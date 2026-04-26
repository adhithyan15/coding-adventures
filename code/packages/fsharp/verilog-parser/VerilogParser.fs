namespace CodingAdventures.VerilogParser.FSharp

open System
open System.Collections.Concurrent
open System.IO
open System.Text
open CodingAdventures.GrammarTools.FSharp
open CodingAdventures.Parser.FSharp
open CodingAdventures.VerilogLexer.FSharp

/// Verilog parser backed by shared grammar definitions.
type VerilogParser private () =
    static let parserGrammars = ConcurrentDictionary<string, ParserGrammar>()

    static member DefaultVersion = VerilogLexer.DefaultVersion
    static member SupportedVersions = VerilogLexer.SupportedVersions

    static member CreateVerilogParser() = VerilogParser.CreateVerilogParser(VerilogParser.DefaultVersion)
    static member CreateVerilogParser(version: string) = GrammarParser(VerilogParser.LoadParserGrammar(version))

    static member ParseVerilog(source: string) = VerilogParser.ParseVerilog(source, VerilogParser.DefaultVersion)
    static member ParseVerilog(source: string, version: string) =
        try
            VerilogParser.CreateVerilogParser(version).Parse(VerilogLexer.TokenizeVerilog(source, version))
        with
        | :? GrammarParseError as error ->
            raise (ArgumentException(sprintf "Verilog parse failed: %s" error.Message, "source", error))

    static member private LoadParserGrammar(version: string) =
        let validated = VerilogParser.ValidateVersion(version)
        parserGrammars.GetOrAdd(validated, Func<string, ParserGrammar>(VerilogParser.ParseParserGrammarResource))

    static member private ValidateVersion(version: string) =
        if String.IsNullOrWhiteSpace(version) then
            VerilogParser.DefaultVersion
        elif VerilogParser.SupportedVersions |> Seq.exists ((=) version) then
            version
        else
            invalidArg "version" (sprintf "Unknown Verilog version '%s'. Valid values: %s" version (String.Join(", ", VerilogParser.SupportedVersions)))

    static member private ParseParserGrammarResource(version: string) =
        try
            ParserGrammarParser.Parse(VerilogParser.ReadResource("verilog" + version + ".grammar"))
        with
        | :? ParserGrammarError as error ->
            raise (InvalidOperationException(sprintf "Failed to parse bundled Verilog parser grammar for version %s" version, error))

    static member private ReadResource(resourceName: string) =
        let assembly = typeof<VerilogParser>.Assembly
        use stream = assembly.GetManifestResourceStream(resourceName)
        if isNull stream then
            invalidOp (sprintf "Missing bundled resource: %s" resourceName)
        use reader = new StreamReader(stream, Encoding.UTF8)
        reader.ReadToEnd()
