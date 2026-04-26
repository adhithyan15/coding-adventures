using System.Collections.Concurrent;
using System.Text;
using CodingAdventures.GrammarTools;
using CodingAdventures.Parser;

namespace CodingAdventures.VhdlParser;

/// <summary>
/// VHDL parser backed by shared grammar definitions.
/// </summary>
public static class VhdlParser
{
    public static string DefaultVersion => CodingAdventures.VhdlLexer.VhdlLexer.DefaultVersion;
    public static IReadOnlyList<string> SupportedVersions => CodingAdventures.VhdlLexer.VhdlLexer.SupportedVersions;

    private static readonly ConcurrentDictionary<string, ParserGrammar> ParserGrammars = new();

    public static GrammarParser CreateVhdlParser() => CreateVhdlParser(DefaultVersion);

    public static GrammarParser CreateVhdlParser(string? version) => new(LoadParserGrammar(version));

    public static ASTNode ParseVhdl(string source) => ParseVhdl(source, DefaultVersion);

    public static ASTNode ParseVhdl(string source, string? version)
    {
        try
        {
            return CreateVhdlParser(version).Parse(CodingAdventures.VhdlLexer.VhdlLexer.TokenizeVhdl(source, version));
        }
        catch (GrammarParseError error)
        {
            throw new ArgumentException("VHDL parse failed: " + error.Message, nameof(source), error);
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
            throw new ArgumentException("Unknown VHDL version '" + version + "'. Valid values: " + string.Join(", ", SupportedVersions), nameof(version));
        }

        return version;
    }

    private static ParserGrammar ParseParserGrammarResource(string version)
    {
        try
        {
            return ParserGrammarParser.Parse(ReadResource("vhdl" + version + ".grammar"));
        }
        catch (ParserGrammarError error)
        {
            throw new InvalidOperationException("Failed to parse bundled VHDL parser grammar for version " + version, error);
        }
    }

    private static string ReadResource(string resourceName)
    {
        var assembly = typeof(VhdlParser).Assembly;
        using var stream = assembly.GetManifestResourceStream(resourceName)
            ?? throw new InvalidOperationException("Missing bundled resource: " + resourceName);
        using var reader = new StreamReader(stream, Encoding.UTF8);
        return reader.ReadToEnd();
    }
}
