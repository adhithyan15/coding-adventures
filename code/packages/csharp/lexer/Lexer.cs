using System.Text.RegularExpressions;
using CodingAdventures.GrammarTools;

namespace CodingAdventures.Lexer;

public enum TokenType
{
    Name,
    Number,
    String,
    Keyword,
    Plus,
    Minus,
    Star,
    Slash,
    Equals,
    EqualsEquals,
    LParen,
    RParen,
    Comma,
    Colon,
    Semicolon,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Dot,
    Bang,
    Newline,
    EOF,
    Grammar,
}

public sealed record Token(TokenType Type, string Value, int Line, int Column, string? TypeName = null, int Flags = 0)
{
    public const int FlagPrecededByNewline = 1;
    public const int FlagContextKeyword = 2;

    public string EffectiveTypeName => TypeName ?? Type.ToString().ToUpperInvariant();
    public bool HasFlag(int flag) => (Flags & flag) != 0;
    public override string ToString() => $"Token({EffectiveTypeName}, \"{Value}\", {Line}:{Column})";
}

public sealed class LexerError : Exception
{
    public LexerError(string message, int line, int column) : base($"Lexer error at {line}:{column}: {message}")
    {
        Line = line;
        Column = column;
    }

    public int Line { get; }
    public int Column { get; }
}

public sealed class GrammarLexer
{
    private sealed record CompiledPattern(string Name, Regex Regex, string? Alias);
    private sealed record MatcherNode(string Stage, CompiledPattern Pattern);

    private readonly TokenGrammar _grammar;
    private readonly List<MatcherNode> _matcherPipeline;
    private readonly HashSet<string> _keywords;
    private readonly HashSet<string> _reserved;
    private readonly HashSet<string> _contextKeywords;

    public GrammarLexer(TokenGrammar grammar)
    {
        _grammar = grammar;
        _matcherPipeline = BuildMatcherPipeline(grammar);
        _keywords = new HashSet<string>(grammar.Keywords, StringComparer.Ordinal);
        _reserved = new HashSet<string>(grammar.ReservedKeywords ?? [], StringComparer.Ordinal);
        _contextKeywords = new HashSet<string>(grammar.ContextKeywords ?? [], StringComparer.Ordinal);
    }

    public IReadOnlyList<Token> Tokenize(string source)
    {
        var workingSource = _grammar.CaseSensitive ? source : source.ToLowerInvariant();
        var tokens = new List<Token>();
        var pos = 0;
        var line = 1;
        var column = 1;
        var precededByNewline = false;

        while (pos < workingSource.Length)
        {
            var matched = false;
            foreach (var node in _matcherPipeline)
            {
                var match = node.Pattern.Regex.Match(workingSource, pos);
                if (!match.Success || match.Index != pos)
                {
                    continue;
                }

                var value = source.Substring(pos, match.Length);
                switch (node.Stage)
                {
                    case "skip":
                        Advance(value, ref line, ref column, ref precededByNewline);
                        pos += value.Length;
                        break;
                    case "token":
                    case "error":
                        var typeName = node.Pattern.Alias ?? node.Pattern.Name;
                        if (typeName == "NAME" && _reserved.Contains(value))
                        {
                            throw new LexerError($"Reserved keyword '{value}'", line, column);
                        }

                        var flags = 0;
                        if (precededByNewline)
                        {
                            flags |= Token.FlagPrecededByNewline;
                        }

                        if (typeName == "NAME" && _contextKeywords.Contains(_grammar.CaseSensitive ? value : value.ToLowerInvariant()))
                        {
                            flags |= Token.FlagContextKeyword;
                        }

                        tokens.Add(new Token(TokenType.Grammar, value, line, column, typeName, flags));
                        var localPrecededByNewline = false;
                        Advance(value, ref line, ref column, ref localPrecededByNewline);
                        precededByNewline = false;
                        pos += value.Length;
                        break;
                }

                matched = true;
                break;
            }

            if (!matched)
            {
                throw new LexerError($"Unexpected character '{source[pos]}'", line, column);
            }
        }

        PromoteKeywords(tokens);
        tokens.Add(new Token(TokenType.EOF, string.Empty, line, column, "EOF"));
        return tokens;
    }

    private void PromoteKeywords(List<Token> tokens)
    {
        if (_keywords.Count == 0)
        {
            return;
        }

        for (var i = 0; i < tokens.Count; i++)
        {
            var token = tokens[i];
            if (token.TypeName == "NAME")
            {
                var check = _grammar.CaseSensitive ? token.Value : token.Value.ToLowerInvariant();
                if (_keywords.Contains(check))
                {
                    tokens[i] = token with { Type = TokenType.Keyword, TypeName = "KEYWORD" };
                }
            }
        }
    }

    private static void Advance(string value, ref int line, ref int column, ref bool precededByNewline)
    {
        foreach (var ch in value)
        {
            if (ch == '\n')
            {
                line++;
                column = 1;
                precededByNewline = true;
            }
            else
            {
                column++;
            }
        }
    }

    private static List<CompiledPattern> CompileDefinitions(IEnumerable<TokenDefinition> definitions, bool caseSensitive)
    {
        var options = RegexOptions.Compiled | RegexOptions.CultureInvariant;
        if (!caseSensitive)
        {
            options |= RegexOptions.IgnoreCase;
        }

        return definitions.Select(definition =>
        {
            var pattern = definition.IsRegex ? $@"\G(?:{definition.Pattern})" : $@"\G{Regex.Escape(definition.Pattern)}";
            return new CompiledPattern(definition.Name, new Regex(pattern, options), definition.Alias);
        }).ToList();
    }

    private static List<MatcherNode> BuildMatcherPipeline(TokenGrammar grammar)
    {
        var pipeline = new List<MatcherNode>();
        pipeline.AddRange(CompileDefinitions(grammar.SkipDefinitions ?? [], grammar.CaseSensitive).Select(pattern => new MatcherNode("skip", pattern)));
        pipeline.AddRange(CompileDefinitions(grammar.Definitions, grammar.CaseSensitive).Select(pattern => new MatcherNode("token", pattern)));
        pipeline.AddRange(CompileDefinitions(grammar.ErrorDefinitions ?? [], grammar.CaseSensitive).Select(pattern => new MatcherNode("error", pattern)));
        return pipeline;
    }
}
