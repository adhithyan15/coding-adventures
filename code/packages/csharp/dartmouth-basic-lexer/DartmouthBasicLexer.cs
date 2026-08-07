using System.Text;
using CodingAdventures.GrammarTools;
using CodingAdventures.Lexer;

namespace CodingAdventures.DartmouthBasicLexer;

/// <summary>
/// Tokenizes the original 1964 Dartmouth BASIC language.
/// </summary>
public sealed class DartmouthBasicLexer
{
    private const string GrammarResource = "dartmouth_basic.tokens";
    private static readonly Lazy<TokenGrammar> TokenGrammar = new(ParseTokenGrammar);
    private readonly string _source;

    private DartmouthBasicLexer(string source) => _source = source;

    /// <summary>Creates a configured lexer for <paramref name="source"/>.</summary>
    public static DartmouthBasicLexer CreateDartmouthBasicLexer(string source)
    {
        ArgumentNullException.ThrowIfNull(source);
        return new DartmouthBasicLexer(source);
    }

    /// <summary>Tokenizes <paramref name="source"/> in one call.</summary>
    public static IReadOnlyList<Token> TokenizeDartmouthBasic(string source) =>
        CreateDartmouthBasicLexer(source).Tokenize();

    /// <summary>Produces the normalized, parser-ready token stream.</summary>
    public IReadOnlyList<Token> Tokenize()
    {
        try
        {
            return PostProcess(new GrammarLexer(TokenGrammar.Value).Tokenize(_source));
        }
        catch (LexerError error)
        {
            throw new ArgumentException("Dartmouth BASIC tokenization failed: " + error.Message, nameof(_source), error);
        }
    }

    private static IReadOnlyList<Token> PostProcess(IReadOnlyList<Token> tokens)
    {
        var result = new List<Token>(tokens.Count);
        var atLineStart = true;
        var suppressingRemark = false;

        foreach (var original in tokens)
        {
            var token = NormalizeValue(original);
            if (atLineStart && token.EffectiveTypeName == "NUMBER")
            {
                token = token with { TypeName = "LINE_NUM" };
            }

            if (atLineStart)
            {
                atLineStart = false;
            }

            if (!suppressingRemark || token.EffectiveTypeName == "NEWLINE")
            {
                result.Add(token);
            }

            if (token.EffectiveTypeName == "KEYWORD" && token.Value == "REM")
            {
                suppressingRemark = true;
            }
            else if (token.EffectiveTypeName == "NEWLINE")
            {
                suppressingRemark = false;
                atLineStart = true;
            }
        }

        return result;
    }

    private static Token NormalizeValue(Token token)
    {
        var value = token.EffectiveTypeName switch
        {
            "KEYWORD" => token.Value.ToUpperInvariant(),
            "BUILTIN_FN" or "USER_FN" or "NAME" or "NUMBER" or "LINE_NUM" => token.Value.ToLowerInvariant(),
            "STRING" when token.Value.Length >= 2 && token.Value[0] == '"' && token.Value[^1] == '"' => token.Value[1..^1],
            "NEWLINE" => "\\n",
            _ => token.Value,
        };
        return token with { Value = value };
    }

    private static TokenGrammar ParseTokenGrammar()
    {
        try
        {
            var assembly = typeof(DartmouthBasicLexer).Assembly;
            using var stream = assembly.GetManifestResourceStream(GrammarResource)
                ?? throw new InvalidOperationException("Missing bundled resource: " + GrammarResource);
            using var reader = new StreamReader(stream, Encoding.UTF8);
            return TokenGrammarParser.Parse(PrepareGrammar(reader.ReadToEnd()));
        }
        catch (TokenGrammarError error)
        {
            throw new InvalidOperationException("Failed to parse bundled Dartmouth BASIC token grammar", error);
        }
    }

    // The .NET grammar parser keeps consuming a keyword section until another
    // named section appears. Move Dartmouth's mid-file keyword list to the end,
    // and lowercase it for the parser's case-insensitive lookup convention.
    private static string PrepareGrammar(string source)
    {
        var output = new List<string>();
        var keywords = new List<string>();
        var readingKeywords = false;

        foreach (var rawLine in source.Replace("\r\n", "\n").Split('\n'))
        {
            var trimmed = rawLine.Trim();
            if (trimmed == "keywords:")
            {
                readingKeywords = true;
                continue;
            }

            if (readingKeywords && trimmed.Length > 0 && !trimmed.StartsWith('#') && char.IsWhiteSpace(rawLine[0]))
            {
                keywords.Add(trimmed.ToLowerInvariant());
                continue;
            }

            if (readingKeywords && trimmed.Length > 0 && !trimmed.StartsWith('#'))
            {
                readingKeywords = false;
            }

            output.Add(rawLine);
        }

        output.Add("keywords:");
        output.AddRange(keywords.Select(keyword => "  " + keyword));
        return string.Join('\n', output);
    }
}
