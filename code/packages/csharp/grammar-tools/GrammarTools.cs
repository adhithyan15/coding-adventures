using System.Collections.ObjectModel;
using System.Text;

namespace CodingAdventures.GrammarTools;

public sealed class TokenGrammarError : Exception
{
    public TokenGrammarError(string message, int lineNumber) : base($"Line {lineNumber}: {message}")
    {
        LineNumber = lineNumber;
    }

    public int LineNumber { get; }
}

public sealed record TokenDefinition(string Name, string Pattern, bool IsRegex, int LineNumber, string? Alias = null);
public sealed record PatternGroup(string Name, IReadOnlyList<TokenDefinition> Definitions);

public sealed record TokenGrammar(
    IReadOnlyList<TokenDefinition> Definitions,
    IReadOnlyList<string> Keywords,
    string? Mode = null,
    string? EscapeMode = null,
    IReadOnlyList<TokenDefinition>? SkipDefinitions = null,
    IReadOnlyList<string>? ReservedKeywords = null,
    IReadOnlyDictionary<string, PatternGroup>? Groups = null,
    bool CaseSensitive = true,
    int Version = 0,
    bool CaseInsensitive = false,
    IReadOnlyList<string>? ContextKeywords = null,
    IReadOnlyList<string>? SoftKeywords = null,
    IReadOnlyList<TokenDefinition>? ErrorDefinitions = null);

public abstract record GrammarElement;
public sealed record RuleReference(string Name, bool IsToken) : GrammarElement;
public sealed record Literal(string Value) : GrammarElement;
public sealed record Group(GrammarElement Element) : GrammarElement;
public sealed record Optional(GrammarElement Element) : GrammarElement;
public sealed record Repetition(GrammarElement Element) : GrammarElement;
public sealed record Alternation(IReadOnlyList<GrammarElement> Choices) : GrammarElement;
public sealed record Sequence(IReadOnlyList<GrammarElement> Elements) : GrammarElement;
public sealed record PositiveLookahead(GrammarElement Element) : GrammarElement;
public sealed record NegativeLookahead(GrammarElement Element) : GrammarElement;
public sealed record OneOrMoreRepetition(GrammarElement Element) : GrammarElement;
public sealed record SeparatedRepetition(GrammarElement Element, GrammarElement Separator, bool AtLeastOne) : GrammarElement;
public sealed record GrammarRule(string Name, GrammarElement Body);

public sealed class ParserGrammar
{
    public ParserGrammar()
    {
        Rules = new List<GrammarRule>();
    }

    public ParserGrammar(IReadOnlyList<GrammarRule> rules)
    {
        Rules = rules;
    }

    public IReadOnlyList<GrammarRule> Rules { get; init; }
}

public sealed class ParserGrammarError : Exception
{
    public ParserGrammarError(string message, int lineNumber) : base($"Line {lineNumber}: {message}")
    {
        LineNumber = lineNumber;
    }

    public int LineNumber { get; }
}

public static class TokenGrammarParser
{
    public static TokenGrammar Parse(string source)
    {
        var definitions = new List<TokenDefinition>();
        var skipDefinitions = new List<TokenDefinition>();
        var errorDefinitions = new List<TokenDefinition>();
        var keywords = new List<string>();
        var reserved = new List<string>();
        var contextKeywords = new List<string>();
        var softKeywords = new List<string>();
        var groups = new Dictionary<string, PatternGroup>(StringComparer.Ordinal);
        var groupDefinitions = new Dictionary<string, List<TokenDefinition>>(StringComparer.Ordinal);
        string? mode = null;
        string? escapeMode = null;
        var version = 0;
        var caseInsensitive = false;

        var section = "definitions";
        string? currentGroup = null;

        var lines = source.Replace("\r\n", "\n").Split('\n');
        for (var index = 0; index < lines.Length; index++)
        {
            var lineNumber = index + 1;
            var rawLine = lines[index];
            var trimmed = rawLine.Trim();
            if (trimmed.Length == 0)
            {
                continue;
            }

            if (trimmed.StartsWith("#", StringComparison.Ordinal))
            {
                if (TryParseMagicComment(trimmed, out var key, out var value))
                {
                    if (key == "version" && int.TryParse(value, out var parsedVersion))
                    {
                        version = parsedVersion;
                    }
                    else if (key == "case_insensitive" && bool.TryParse(value, out var parsedBool))
                    {
                        caseInsensitive = parsedBool;
                    }
                }

                continue;
            }

            if (trimmed.StartsWith("mode:", StringComparison.Ordinal))
            {
                mode = trimmed["mode:".Length..].Trim();
                continue;
            }

            if (trimmed.StartsWith("escape_mode:", StringComparison.Ordinal))
            {
                escapeMode = trimmed["escape_mode:".Length..].Trim();
                continue;
            }

            if (trimmed.StartsWith("escapes:", StringComparison.Ordinal))
            {
                escapeMode = trimmed["escapes:".Length..].Trim();
                continue;
            }

            if (trimmed.StartsWith("case_sensitive:", StringComparison.Ordinal))
            {
                if (bool.TryParse(trimmed["case_sensitive:".Length..].Trim(), out var parsedBool))
                {
                    caseInsensitive = !parsedBool;
                }

                continue;
            }

            if (trimmed == "keywords:" || trimmed == "reserved:" || trimmed == "context_keywords:" || trimmed == "soft_keywords:" || trimmed == "skip:" || trimmed == "errors:")
            {
                section = trimmed[..^1];
                currentGroup = null;
                continue;
            }

            if (trimmed.StartsWith("group ", StringComparison.Ordinal) && trimmed.EndsWith(':'))
            {
                currentGroup = trimmed[6..^1].Trim();
                if (currentGroup.Length == 0)
                {
                    throw new TokenGrammarError("Group name cannot be empty", lineNumber);
                }

                section = "group";
                groupDefinitions[currentGroup] = new List<TokenDefinition>();
                continue;
            }

            if ((section == "skip" || section == "errors" || section == "group")
                && !char.IsWhiteSpace(rawLine[0])
                && trimmed.Contains('='))
            {
                section = "definitions";
                currentGroup = null;
            }

            switch (section)
            {
                case "keywords":
                    keywords.Add(trimmed);
                    break;
                case "reserved":
                    reserved.Add(trimmed);
                    break;
                case "context_keywords":
                    contextKeywords.Add(trimmed);
                    break;
                case "soft_keywords":
                    softKeywords.Add(trimmed);
                    break;
                case "definitions":
                case "skip":
                case "errors":
                case "group":
                    var definition = ParseDefinition(trimmed, lineNumber);
                    if (section == "skip")
                    {
                        skipDefinitions.Add(definition);
                    }
                    else if (section == "errors")
                    {
                        errorDefinitions.Add(definition);
                    }
                    else if (section == "group")
                    {
                        groupDefinitions[currentGroup!].Add(definition);
                    }
                    else
                    {
                        definitions.Add(definition);
                    }
                    break;
                default:
                    throw new TokenGrammarError($"Unsupported section '{section}'", lineNumber);
            }
        }

        foreach (var (name, defs) in groupDefinitions)
        {
            groups[name] = new PatternGroup(name, defs);
        }

        return new TokenGrammar(
            definitions,
            keywords,
            mode,
            escapeMode,
            skipDefinitions,
            reserved,
            new ReadOnlyDictionary<string, PatternGroup>(groups),
            !caseInsensitive,
            version,
            caseInsensitive,
            contextKeywords,
            softKeywords,
            errorDefinitions);
    }

    private static bool TryParseMagicComment(string line, out string key, out string value)
    {
        key = string.Empty;
        value = string.Empty;
        var trimmed = line.Trim();
        if (!trimmed.StartsWith("# @", StringComparison.Ordinal))
        {
            return false;
        }

        var rest = trimmed[3..].Trim();
        var split = rest.Split(' ', 2, StringSplitOptions.RemoveEmptyEntries);
        if (split.Length == 0)
        {
            return false;
        }

        key = split[0];
        value = split.Length == 2 ? split[1].Trim() : string.Empty;
        return true;
    }

    private static TokenDefinition ParseDefinition(string line, int lineNumber)
    {
        var equalsIndex = line.IndexOf('=');
        if (equalsIndex < 1)
        {
            throw new TokenGrammarError("Expected TOKEN_NAME = PATTERN", lineNumber);
        }

        var name = line[..equalsIndex].Trim();
        var rhs = line[(equalsIndex + 1)..].Trim();
        string? alias = null;

        var aliasIndex = FindAliasMarker(rhs);
        if (aliasIndex >= 0)
        {
            alias = rhs[(aliasIndex + 2)..].Trim();
            rhs = rhs[..aliasIndex].Trim();
        }

        if (rhs.StartsWith('/') && rhs.EndsWith('/'))
        {
            return new TokenDefinition(name, rhs[1..^1], true, lineNumber, alias);
        }

        if (rhs.StartsWith('"') && rhs.EndsWith('"'))
        {
            return new TokenDefinition(name, UnescapeQuoted(rhs[1..^1]), false, lineNumber, alias);
        }

        throw new TokenGrammarError("Pattern must be /regex/ or \"literal\"", lineNumber);
    }

    private static int FindAliasMarker(string rhs)
    {
        var inRegex = false;
        var inString = false;
        var escaped = false;

        for (var index = 0; index < rhs.Length - 1; index++)
        {
            var ch = rhs[index];
            if (escaped)
            {
                escaped = false;
                continue;
            }

            if (ch == '\\')
            {
                escaped = true;
                continue;
            }

            if (ch == '"' && !inRegex)
            {
                inString = !inString;
                continue;
            }

            if (ch == '/' && !inString)
            {
                inRegex = !inRegex;
                continue;
            }

            if (!inRegex && !inString && ch == '-' && rhs[index + 1] == '>')
            {
                return index;
            }
        }

        return -1;
    }

    private static string UnescapeQuoted(string text)
    {
        var result = new StringBuilder(text.Length);
        for (var i = 0; i < text.Length; i++)
        {
            if (text[i] == '\\' && i + 1 < text.Length)
            {
                i++;
                result.Append(text[i] switch
                {
                    'n' => '\n',
                    't' => '\t',
                    'r' => '\r',
                    '"' => '"',
                    '\\' => '\\',
                    _ => text[i],
                });
            }
            else
            {
                result.Append(text[i]);
            }
        }

        return result.ToString();
    }
}

public static class TokenGrammarValidator
{
    public static void Validate(TokenGrammar grammar)
    {
        var seen = new HashSet<string>(StringComparer.Ordinal);
        foreach (var definition in grammar.Definitions)
        {
            if (!seen.Add(definition.Name))
            {
                throw new InvalidOperationException($"Duplicate token definition: {definition.Name}");
            }
        }
    }
}

public static class ParserGrammarParser
{
    private enum Kind
    {
        Identifier,
        String,
        Equals,
        Pipe,
        LBrace,
        RBrace,
        LBracket,
        RBracket,
        LParen,
        RParen,
        Semicolon,
        Ampersand,
        Bang,
        DoubleSlash,
        Plus,
        End,
    }

    private sealed record MetaToken(Kind Kind, string Text, int Line);

    private sealed class Parser
    {
        private readonly List<MetaToken> _tokens;
        private int _pos;

        public Parser(List<MetaToken> tokens)
        {
            _tokens = tokens;
        }

        public ParserGrammar Parse()
        {
            var rules = new List<GrammarRule>();
            while (Peek().Kind != Kind.End)
            {
                var name = Expect(Kind.Identifier).Text;
                Expect(Kind.Equals);
                var body = ParseAlternation([Kind.Semicolon]);
                Expect(Kind.Semicolon);
                rules.Add(new GrammarRule(name, body));
            }

            return new ParserGrammar(rules);
        }

        private GrammarElement ParseAlternation(HashSet<Kind> terminators)
        {
            var choices = new List<GrammarElement> { ParseSequence(new HashSet<Kind>(terminators) { Kind.Pipe }) };
            while (Match(Kind.Pipe))
            {
                choices.Add(ParseSequence(new HashSet<Kind>(terminators) { Kind.Pipe }));
            }

            return choices.Count == 1 ? choices[0] : new Alternation(choices);
        }

        private GrammarElement ParseSequence(HashSet<Kind> terminators)
        {
            var elements = new List<GrammarElement>();
            while (!terminators.Contains(Peek().Kind) && Peek().Kind != Kind.End)
            {
                elements.Add(ParseElement());
            }

            return elements.Count switch
            {
                0 => new Sequence(Array.Empty<GrammarElement>()),
                1 => elements[0],
                _ => new Sequence(elements),
            };
        }

        private GrammarElement ParseElement()
        {
            if (Match(Kind.Ampersand))
            {
                return new PositiveLookahead(ParseElement());
            }

            if (Match(Kind.Bang))
            {
                return new NegativeLookahead(ParseElement());
            }

            if (Match(Kind.Identifier, out var identifier))
            {
                var isToken = identifier!.Text.All(static ch => char.IsUpper(ch) || ch == '_');
                return new RuleReference(identifier.Text, isToken);
            }

            if (Match(Kind.String, out var literal))
            {
                return new Literal(literal!.Text);
            }

            if (Match(Kind.LParen))
            {
                var inner = ParseAlternation([Kind.RParen]);
                Expect(Kind.RParen);
                return new Group(inner);
            }

            if (Match(Kind.LBracket))
            {
                var inner = ParseAlternation([Kind.RBracket]);
                Expect(Kind.RBracket);
                return new Optional(inner);
            }

            if (Match(Kind.LBrace))
            {
                var inner = ParseAlternation([Kind.RBrace, Kind.DoubleSlash]);
                GrammarElement repetition;
                if (Match(Kind.DoubleSlash))
                {
                    var separator = ParseAlternation([Kind.RBrace]);
                    Expect(Kind.RBrace);
                    repetition = new SeparatedRepetition(inner, separator, false);
                }
                else
                {
                    Expect(Kind.RBrace);
                    repetition = new Repetition(inner);
                }

                if (Match(Kind.Plus))
                {
                    return repetition switch
                    {
                        SeparatedRepetition separated => separated with { AtLeastOne = true },
                        Repetition repeated => new OneOrMoreRepetition(repeated.Element),
                        _ => repetition,
                    };
                }

                return repetition;
            }

            throw new ParserGrammarError($"Unexpected token '{Peek().Text}'", Peek().Line);
        }

        private MetaToken Peek() => _tokens[_pos];

        private bool Match(Kind kind) => Match(kind, out _);

        private bool Match(Kind kind, out MetaToken? token)
        {
            if (Peek().Kind == kind)
            {
                token = _tokens[_pos++];
                return true;
            }

            token = null;
            return false;
        }

        private MetaToken Expect(Kind kind)
        {
            if (!Match(kind, out var token))
            {
                throw new ParserGrammarError($"Expected {kind} but found '{Peek().Text}'", Peek().Line);
            }

            return token!;
        }
    }

    public static ParserGrammar Parse(string source)
    {
        return new Parser(Tokenize(source)).Parse();
    }

    private static List<MetaToken> Tokenize(string source)
    {
        var tokens = new List<MetaToken>();
        var line = 1;
        for (var i = 0; i < source.Length;)
        {
            var ch = source[i];
            if (ch == '\r')
            {
                i++;
                continue;
            }

            if (ch == '\n')
            {
                line++;
                i++;
                continue;
            }

            if (char.IsWhiteSpace(ch))
            {
                i++;
                continue;
            }

            if (ch == '#')
            {
                while (i < source.Length && source[i] != '\n')
                {
                    i++;
                }

                continue;
            }

            if (char.IsLetter(ch) || ch == '_')
            {
                var start = i;
                while (i < source.Length && (char.IsLetterOrDigit(source[i]) || source[i] == '_' || source[i] == '-'))
                {
                    i++;
                }

                tokens.Add(new MetaToken(Kind.Identifier, source[start..i], line));
                continue;
            }

            if (ch == '"')
            {
                var builder = new StringBuilder();
                i++;
                while (i < source.Length)
                {
                    if (source[i] == '"' && source[i - 1] != '\\')
                    {
                        i++;
                        break;
                    }

                    if (source[i] == '\\' && i + 1 < source.Length)
                    {
                        i++;
                        builder.Append(source[i] switch
                        {
                            'n' => '\n',
                            't' => '\t',
                            'r' => '\r',
                            '"' => '"',
                            '\\' => '\\',
                            _ => source[i],
                        });
                    }
                    else
                    {
                        builder.Append(source[i]);
                    }

                    i++;
                }

                tokens.Add(new MetaToken(Kind.String, builder.ToString(), line));
                continue;
            }

            if (ch == '/' && i + 1 < source.Length && source[i + 1] == '/')
            {
                tokens.Add(new MetaToken(Kind.DoubleSlash, "//", line));
                i += 2;
                continue;
            }

            tokens.Add(ch switch
            {
                '=' => new MetaToken(Kind.Equals, "=", line),
                '|' => new MetaToken(Kind.Pipe, "|", line),
                '{' => new MetaToken(Kind.LBrace, "{", line),
                '}' => new MetaToken(Kind.RBrace, "}", line),
                '[' => new MetaToken(Kind.LBracket, "[", line),
                ']' => new MetaToken(Kind.RBracket, "]", line),
                '(' => new MetaToken(Kind.LParen, "(", line),
                ')' => new MetaToken(Kind.RParen, ")", line),
                ';' => new MetaToken(Kind.Semicolon, ";", line),
                '&' => new MetaToken(Kind.Ampersand, "&", line),
                '!' => new MetaToken(Kind.Bang, "!", line),
                '+' => new MetaToken(Kind.Plus, "+", line),
                _ => throw new ParserGrammarError($"Unexpected character '{ch}'", line),
            });
            i++;
        }

        tokens.Add(new MetaToken(Kind.End, string.Empty, line));
        return tokens;
    }
}

public static class ParserGrammarValidator
{
    public static void Validate(ParserGrammar grammar)
    {
        var names = new HashSet<string>(StringComparer.Ordinal);
        foreach (var rule in grammar.Rules)
        {
            if (!names.Add(rule.Name))
            {
                throw new InvalidOperationException($"Duplicate grammar rule: {rule.Name}");
            }
        }
    }
}

public static class CrossValidator
{
    public static void Validate(TokenGrammar tokenGrammar, ParserGrammar parserGrammar)
    {
        var tokenNames = tokenGrammar.Definitions.Select(def => def.Alias ?? def.Name).ToHashSet(StringComparer.Ordinal);
        if (tokenGrammar.Keywords.Count > 0)
        {
            tokenNames.Add("KEYWORD");
        }

        foreach (var rule in parserGrammar.Rules)
        {
            Visit(rule.Body);
        }

        void Visit(GrammarElement element)
        {
            switch (element)
            {
                case RuleReference reference when reference.IsToken && !tokenNames.Contains(reference.Name):
                    throw new InvalidOperationException($"Unknown token reference: {reference.Name}");
                case Group group:
                    Visit(group.Element);
                    break;
                case Optional optional:
                    Visit(optional.Element);
                    break;
                case Repetition repetition:
                    Visit(repetition.Element);
                    break;
                case OneOrMoreRepetition repetition:
                    Visit(repetition.Element);
                    break;
                case PositiveLookahead lookahead:
                    Visit(lookahead.Element);
                    break;
                case NegativeLookahead lookahead:
                    Visit(lookahead.Element);
                    break;
                case SeparatedRepetition separated:
                    Visit(separated.Element);
                    Visit(separated.Separator);
                    break;
                case Alternation alternation:
                    foreach (var choice in alternation.Choices)
                    {
                        Visit(choice);
                    }
                    break;
                case Sequence sequence:
                    foreach (var child in sequence.Elements)
                    {
                        Visit(child);
                    }
                    break;
            }
        }
    }
}
