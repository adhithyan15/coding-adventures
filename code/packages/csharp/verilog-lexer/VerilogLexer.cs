using System.Collections.Concurrent;
using System.Text;
using CodingAdventures.GrammarTools;
using CodingAdventures.Lexer;

namespace CodingAdventures.VerilogLexer;

/// <summary>
/// Verilog lexer backed by shared grammar token definitions.
/// </summary>
public static class VerilogLexer
{
    public const string DefaultVersion = "2005";
    public static IReadOnlyList<string> SupportedVersions { get; } = new[] { "1995", "2001", "2005" };

    private static readonly ConcurrentDictionary<string, TokenGrammar> TokenGrammars = new();

    public static GrammarLexer CreateVerilogLexer() => CreateVerilogLexer(DefaultVersion);

    public static GrammarLexer CreateVerilogLexer(string? version) => new(LoadTokenGrammar(version));

    public static IReadOnlyList<Token> TokenizeVerilog(string source) => TokenizeVerilog(source, DefaultVersion);

    public static IReadOnlyList<Token> TokenizeVerilog(string source, string? version)
    {
        try
        {
            return CreateVerilogLexer(version).Tokenize(source);
        }
        catch (LexerError error)
        {
            throw new ArgumentException("Verilog tokenization failed: " + error.Message, nameof(source), error);
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
            throw new ArgumentException("Unknown Verilog version '" + version + "'. Valid values: " + string.Join(", ", SupportedVersions), nameof(version));
        }

        return version;
    }

    private static TokenGrammar ParseTokenGrammarResource(string version)
    {
        try
        {
            return TokenGrammarParser.Parse(ReadResource("verilog" + version + ".tokens"));
        }
        catch (TokenGrammarError error)
        {
            throw new InvalidOperationException("Failed to parse bundled Verilog token grammar for version " + version, error);
        }
    }

    private static string ReadResource(string resourceName)
    {
        var assembly = typeof(VerilogLexer).Assembly;
        using var stream = assembly.GetManifestResourceStream(resourceName)
            ?? throw new InvalidOperationException("Missing bundled resource: " + resourceName);
        using var reader = new StreamReader(stream, Encoding.UTF8);
        return reader.ReadToEnd();
    }
}
