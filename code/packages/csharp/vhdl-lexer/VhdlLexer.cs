using System.Collections.Concurrent;
using System.Text;
using CodingAdventures.GrammarTools;
using CodingAdventures.Lexer;

namespace CodingAdventures.VhdlLexer;

/// <summary>
/// VHDL lexer backed by shared grammar token definitions.
/// </summary>
public static class VhdlLexer
{
    public const string DefaultVersion = "2008";
    public static IReadOnlyList<string> SupportedVersions { get; } = new[] { "1987", "1993", "2002", "2008", "2019" };

    private static readonly ConcurrentDictionary<string, TokenGrammar> TokenGrammars = new();

    public static GrammarLexer CreateVhdlLexer() => CreateVhdlLexer(DefaultVersion);

    public static GrammarLexer CreateVhdlLexer(string? version) => new(LoadTokenGrammar(version));

    public static IReadOnlyList<Token> TokenizeVhdl(string source) => TokenizeVhdl(source, DefaultVersion);

    public static IReadOnlyList<Token> TokenizeVhdl(string source, string? version)
    {
        var grammar = LoadTokenGrammar(version);
        try
        {
            var tokens = new GrammarLexer(grammar).Tokenize(source);
            return NormalizeCase(tokens, new HashSet<string>(grammar.Keywords, StringComparer.Ordinal));
        }
        catch (LexerError error)
        {
            throw new ArgumentException("VHDL tokenization failed: " + error.Message, nameof(source), error);
        }
    }

    private static IReadOnlyList<Token> NormalizeCase(IReadOnlyList<Token> tokens, ISet<string> keywords)
    {
        var normalized = new List<Token>(tokens.Count);
        foreach (var token in tokens)
        {
            var normalizeKeyword = token.Type == TokenType.Keyword;
            var normalizeName = token.Type == TokenType.Grammar && token.TypeName == "NAME";
            if (!normalizeKeyword && !normalizeName)
            {
                normalized.Add(token);
                continue;
            }

            var lowered = token.Value.ToLowerInvariant();
            var normalizedType = normalizeKeyword ? TokenType.Keyword : token.Type;
            var normalizedTypeName = normalizeKeyword ? "KEYWORD" : token.TypeName;
            if (normalizeName && keywords.Contains(lowered))
            {
                normalizedType = TokenType.Keyword;
                normalizedTypeName = "KEYWORD";
            }

            normalized.Add(token with { Type = normalizedType, Value = lowered, TypeName = normalizedTypeName });
        }

        return normalized;
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
            throw new ArgumentException("Unknown VHDL version '" + version + "'. Valid values: " + string.Join(", ", SupportedVersions), nameof(version));
        }

        return version;
    }

    private static TokenGrammar ParseTokenGrammarResource(string version)
    {
        try
        {
            return TokenGrammarParser.Parse(ReadResource("vhdl" + version + ".tokens"));
        }
        catch (TokenGrammarError error)
        {
            throw new InvalidOperationException("Failed to parse bundled VHDL token grammar for version " + version, error);
        }
    }

    private static string ReadResource(string resourceName)
    {
        var assembly = typeof(VhdlLexer).Assembly;
        using var stream = assembly.GetManifestResourceStream(resourceName)
            ?? throw new InvalidOperationException("Missing bundled resource: " + resourceName);
        using var reader = new StreamReader(stream, Encoding.UTF8);
        return reader.ReadToEnd();
    }
}
