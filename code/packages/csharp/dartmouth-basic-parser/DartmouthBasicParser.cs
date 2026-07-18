using System.Text;
using CodingAdventures.GrammarTools;
using CodingAdventures.Lexer;
using CodingAdventures.Parser;

namespace CodingAdventures.DartmouthBasicParser;

/// <summary>
/// Parses original 1964 Dartmouth BASIC source into a grammar-shaped AST.
/// </summary>
public sealed class DartmouthBasicParser
{
    private const string GrammarResource = "dartmouth_basic.grammar";
    private static readonly Lazy<ParserGrammar> ParserGrammar = new(ParseParserGrammar);
    private readonly IReadOnlyList<Token> _tokens;

    private DartmouthBasicParser(IReadOnlyList<Token> tokens) => _tokens = tokens;

    /// <summary>Tokenizes <paramref name="source"/> and creates a configured parser.</summary>
    public static DartmouthBasicParser CreateDartmouthBasicParser(string source) =>
        new(CodingAdventures.DartmouthBasicLexer.DartmouthBasicLexer.TokenizeDartmouthBasic(source));

    /// <summary>Parses an existing Dartmouth BASIC token stream.</summary>
    public static ASTNode ParseTokens(IReadOnlyList<Token> tokens)
    {
        ArgumentNullException.ThrowIfNull(tokens);
        return new DartmouthBasicParser(tokens).Parse();
    }

    /// <summary>Tokenizes and parses <paramref name="source"/> in one call.</summary>
    public static ASTNode ParseDartmouthBasic(string source) =>
        CreateDartmouthBasicParser(source).Parse();

    /// <summary>Parses the configured token stream and requires complete input consumption.</summary>
    public ASTNode Parse()
    {
        try
        {
            var ast = new GrammarParser(ParserGrammar.Value).Parse(_tokens);
            EnsureCompleteParse(ast);
            return ast;
        }
        catch (GrammarParseError error)
        {
            throw new ArgumentException("Dartmouth BASIC parse failed: " + error.Message, "source", error);
        }
    }

    private void EnsureCompleteParse(ASTNode ast)
    {
        if (_tokens.Count == 0 || _tokens[^1].EffectiveTypeName != "EOF")
        {
            var finalToken = _tokens.Count > 0 ? _tokens[^1] : null;
            throw new GrammarParseError("Token stream must end with EOF", finalToken);
        }

        var eofIndex = _tokens.Count - 1;
        var parsedTokenCount = CountTokens(ast);
        if (parsedTokenCount != eofIndex)
        {
            var token = parsedTokenCount < _tokens.Count ? _tokens[parsedTokenCount] : null;
            throw new GrammarParseError("Unexpected token while parsing program", token);
        }
    }

    private static int CountTokens(ASTNode node) =>
        node.Children.Sum(child => child switch
        {
            Token => 1,
            ASTNode nested => CountTokens(nested),
            _ => 0,
        });

    private static ParserGrammar ParseParserGrammar()
    {
        try
        {
            var assembly = typeof(DartmouthBasicParser).Assembly;
            using var stream = assembly.GetManifestResourceStream(GrammarResource)
                ?? throw new InvalidOperationException("Missing bundled resource: " + GrammarResource);
            using var reader = new StreamReader(stream, Encoding.UTF8);
            return ParserGrammarParser.Parse(reader.ReadToEnd());
        }
        catch (ParserGrammarError error)
        {
            throw new InvalidOperationException("Failed to parse bundled Dartmouth BASIC grammar", error);
        }
    }
}
