using System.Collections.Concurrent;
using System.Text;
using CodingAdventures.GrammarTools;
using CodingAdventures.Parser;

namespace CodingAdventures.AlgolParser;

/// <summary>
/// ALGOL parser backed by shared grammar definitions.
/// </summary>
public static class AlgolParser
{
    public static string DefaultVersion => CodingAdventures.AlgolLexer.AlgolLexer.DefaultVersion;
    public static IReadOnlyList<string> SupportedVersions => CodingAdventures.AlgolLexer.AlgolLexer.SupportedVersions;

    private static readonly ConcurrentDictionary<string, ParserGrammar> ParserGrammars = new();

    public static GrammarParser CreateAlgolParser() => CreateAlgolParser(DefaultVersion);

    public static GrammarParser CreateAlgolParser(string? version) => new(LoadParserGrammar(version));

    public static ASTNode ParseAlgol(string source) => ParseAlgol(source, DefaultVersion);

    public static ASTNode ParseAlgol(string source, string? version)
    {
        try
        {
            return CreateAlgolParser(version).Parse(CodingAdventures.AlgolLexer.AlgolLexer.TokenizeAlgol(source, version));
        }
        catch (GrammarParseError error)
        {
            throw new ArgumentException("ALGOL parse failed: " + error.Message, nameof(source), error);
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
            throw new ArgumentException("Unknown ALGOL version '" + version + "'. Valid values: " + string.Join(", ", SupportedVersions), nameof(version));
        }

        return version;
    }

    private static ParserGrammar ParseParserGrammarResource(string version)
    {
        try
        {
            return ParserGrammarParser.Parse(ReadResource(version + ".grammar"));
        }
        catch (ParserGrammarError error)
        {
            throw new InvalidOperationException("Failed to parse bundled ALGOL parser grammar for version " + version, error);
        }
    }

    private static string ReadResource(string resourceName)
    {
        var assembly = typeof(AlgolParser).Assembly;
        using var stream = assembly.GetManifestResourceStream(resourceName)
            ?? throw new InvalidOperationException("Missing bundled resource: " + resourceName);
        using var reader = new StreamReader(stream, Encoding.UTF8);
        return reader.ReadToEnd();
    }
}
