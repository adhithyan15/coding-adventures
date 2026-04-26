namespace CodingAdventures.VerilogLexer.FSharp

open System
open System.Collections.Concurrent
open System.Collections.Generic
open System.IO
open System.Text
open CodingAdventures.GrammarTools.FSharp
open CodingAdventures.Lexer.FSharp

/// Verilog lexer backed by shared grammar token definitions.
type VerilogLexer private () =
    static let defaultVersion = "2005"
    static let supportedVersions = [| "1995"; "2001"; "2005" |]
    static let tokenGrammars = ConcurrentDictionary<string, TokenGrammar>()

    static member DefaultVersion = defaultVersion
    static member SupportedVersions = supportedVersions :> IReadOnlyList<string>

    static member CreateVerilogLexer() = VerilogLexer.CreateVerilogLexer(defaultVersion)
    static member CreateVerilogLexer(version: string) = GrammarLexer(VerilogLexer.LoadTokenGrammar(version))

    static member TokenizeVerilog(source: string) = VerilogLexer.TokenizeVerilog(source, defaultVersion)
    static member TokenizeVerilog(source: string, version: string) =
        try
            VerilogLexer.CreateVerilogLexer(version).Tokenize(source)
        with
        | :? LexerError as error ->
            raise (ArgumentException(sprintf "Verilog tokenization failed: %s" error.Message, "source", error))

    static member private LoadTokenGrammar(version: string) =
        let validated = VerilogLexer.ValidateVersion(version)
        tokenGrammars.GetOrAdd(validated, Func<string, TokenGrammar>(VerilogLexer.ParseTokenGrammarResource))

    static member private ValidateVersion(version: string) =
        if String.IsNullOrWhiteSpace(version) then
            defaultVersion
        elif Array.exists ((=) version) supportedVersions then
            version
        else
            invalidArg "version" (sprintf "Unknown Verilog version '%s'. Valid values: %s" version (String.Join(", ", supportedVersions)))

    static member private ParseTokenGrammarResource(version: string) =
        try
            TokenGrammarParser.Parse(VerilogLexer.ReadResource("verilog" + version + ".tokens"))
        with
        | :? TokenGrammarError as error ->
            raise (InvalidOperationException(sprintf "Failed to parse bundled Verilog token grammar for version %s" version, error))

    static member private ReadResource(resourceName: string) =
        let assembly = typeof<VerilogLexer>.Assembly
        use stream = assembly.GetManifestResourceStream(resourceName)
        if isNull stream then
            invalidOp (sprintf "Missing bundled resource: %s" resourceName)
        use reader = new StreamReader(stream, Encoding.UTF8)
        reader.ReadToEnd()
