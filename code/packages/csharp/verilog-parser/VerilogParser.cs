using System.Collections.Concurrent;
using System.Text;
using CodingAdventures.GrammarTools;
using CodingAdventures.Parser;

namespace CodingAdventures.VerilogParser;

/// <summary>
/// Verilog parser backed by shared grammar definitions.
/// </summary>
public static class VerilogParser
{
    public static string DefaultVersion => CodingAdventures.VerilogLexer.VerilogLexer.DefaultVersion;
    public static IReadOnlyList<string> SupportedVersions => CodingAdventures.VerilogLexer.VerilogLexer.SupportedVersions;

    private static readonly ConcurrentDictionary<string, ParserGrammar> ParserGrammars = new();

    public static GrammarParser CreateVerilogParser() => CreateVerilogParser(DefaultVersion);

    public static GrammarParser CreateVerilogParser(string? version) => new(LoadParserGrammar(version));

    public static ASTNode ParseVerilog(string source) => ParseVerilog(source, DefaultVersion);

    public static ASTNode ParseVerilog(string source, string? version)
    {
        try
        {
            return CreateVerilogParser(version).Parse(CodingAdventures.VerilogLexer.VerilogLexer.TokenizeVerilog(source, version));
        }
        catch (GrammarParseError error)
        {
            throw new ArgumentException("Verilog parse failed: " + error.Message, nameof(source), error);
        }
    }

    private static ParserGrammar LoadParserGrammar(string? version)
    {
        var validated = ValidateVersion(version);
        return ParserGrammars.GetOrAdd(validated, ParseParserGrammarResource);
    }

    private static string ValidateVersion(string? version)
    {
        if (string.IsNullOrWhiteSpace(version))
        {
            return DefaultVersion;
        }

        if (!SupportedVersions.Contains(version))
        {
            throw new ArgumentException("Unknown Verilog version '" + version + "'. Valid values: " + string.Join(", ", SupportedVersions), nameof(version));
        }

        return version;
    }

    private static ParserGrammar ParseParserGrammarResource(string version)
    {
        try
        {
            return ParserGrammarParser.Parse(ReadResource("verilog" + version + ".grammar"));
        }
        catch (ParserGrammarError error)
        {
            throw new InvalidOperationException("Failed to parse bundled Verilog parser grammar for version " + version, error);
        }
    }

    private static string ReadResource(string resourceName)
    {
        var assembly = typeof(VerilogParser).Assembly;
        using var stream = assembly.GetManifestResourceStream(resourceName)
            ?? throw new InvalidOperationException("Missing bundled resource: " + resourceName);
        using var reader = new StreamReader(stream, Encoding.UTF8);
        return reader.ReadToEnd();
    }
}
