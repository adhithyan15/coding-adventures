using System.Collections.Concurrent;
using System.Text;
using CodingAdventures.GrammarTools;
using CodingAdventures.Lexer;

namespace CodingAdventures.AlgolLexer;

/// <summary>
/// ALGOL lexer backed by shared grammar token definitions.
/// </summary>
public static class AlgolLexer
{
    public const string DefaultVersion = "algol60";
    public static IReadOnlyList<string> SupportedVersions { get; } = new[] { "algol60" };

    private static readonly ConcurrentDictionary<string, TokenGrammar> TokenGrammars = new();

    public static GrammarLexer CreateAlgolLexer() => CreateAlgolLexer(DefaultVersion);

    public static GrammarLexer CreateAlgolLexer(string? version) => new(LoadTokenGrammar(version));

    public static IReadOnlyList<Token> TokenizeAlgol(string source) => TokenizeAlgol(source, DefaultVersion);

    public static IReadOnlyList<Token> TokenizeAlgol(string source, string? version)
    {
        try
        {
            return CreateAlgolLexer(version).Tokenize(source);
        }
        catch (LexerError error)
        {
            throw new ArgumentException("ALGOL tokenization failed: " + error.Message, nameof(source), error);
        }
    }

    private static TokenGrammar LoadTokenGrammar(string? version)
    {
        var validated = ValidateVersion(version);
        return TokenGrammars.GetOrAdd(validated, ParseTokenGrammarResource);
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

    private static TokenGrammar ParseTokenGrammarResource(string version)
    {
        try
        {
            return TokenGrammarParser.Parse(ReadResource(version + ".tokens"));
        }
        catch (TokenGrammarError error)
        {
            throw new InvalidOperationException("Failed to parse bundled ALGOL token grammar for version " + version, error);
        }
    }

    private static string ReadResource(string resourceName)
    {
        var assembly = typeof(AlgolLexer).Assembly;
        using var stream = assembly.GetManifestResourceStream(resourceName)
            ?? throw new InvalidOperationException("Missing bundled resource: " + resourceName);
        using var reader = new StreamReader(stream, Encoding.UTF8);
        return reader.ReadToEnd();
    }
}
