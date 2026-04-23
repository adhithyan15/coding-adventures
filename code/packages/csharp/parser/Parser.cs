using CodingAdventures.GrammarTools;
using CodingAdventures.Lexer;

namespace CodingAdventures.Parser;

public sealed class GrammarParseError : Exception
{
    public GrammarParseError(string message, Token? token = null) : base(token is null
        ? $"Parse error: {message}"
        : $"Parse error at {token.Line}:{token.Column}: {message}")
    {
        Token = token;
    }

    public Token? Token { get; }
}

public sealed class ASTNode
{
    public ASTNode(string ruleName)
        : this(ruleName, [], 0, 0, 0, 0)
    {
    }

    public ASTNode(string ruleName, IReadOnlyList<object> children, int startLine = 0, int startColumn = 0, int endLine = 0, int endColumn = 0)
    {
        RuleName = ruleName;
        Children = children;
        StartLine = startLine;
        StartColumn = startColumn;
        EndLine = endLine;
        EndColumn = endColumn;
    }

    public string RuleName { get; }
    public IReadOnlyList<object> Children { get; }
    public int StartLine { get; }
    public int StartColumn { get; }
    public int EndLine { get; }
    public int EndColumn { get; }
    public bool IsLeaf => Children.Count == 1 && Children[0] is Token;
    public Token? Token => IsLeaf ? (Token)Children[0] : null;
    public int DescendantCount() => Children.Sum(child => child is ASTNode node ? 1 + node.DescendantCount() : 1);
}

public sealed class GrammarParser
{
    private sealed record MemoEntry(IReadOnlyList<object>? Children, int EndPos, bool Ok);
    private sealed record MatchResult(IReadOnlyList<object> Children, int EndPos);

    private readonly ParserGrammar _grammar;
    private readonly Dictionary<string, GrammarRule> _ruleMap;

    public GrammarParser(ParserGrammar grammar)
    {
        _grammar = grammar;
        _ruleMap = grammar.Rules.ToDictionary(rule => rule.Name, StringComparer.Ordinal);
    }

    public ASTNode Parse(IReadOnlyList<Token> tokens)
    {
        if (_grammar.Rules.Count == 0)
        {
            throw new GrammarParseError("No rules in grammar", tokens.Count > 0 ? tokens[^1] : null);
        }

        var startRule = _grammar.Rules[0].Name;
        var memo = new Dictionary<(string Rule, int Pos), MemoEntry>();
        var recursion = new HashSet<(string Rule, int Pos)>();
        var result = MatchRule(startRule, tokens, 0, memo, recursion);
        if (result is null)
        {
            throw new GrammarParseError($"Failed to parse starting rule '{startRule}'", tokens.Count > 0 ? tokens[0] : null);
        }

        if (result.Children.Count == 1 && result.Children[0] is ASTNode node && node.RuleName == startRule)
        {
            return node;
        }

        return BuildNode(startRule, result.Children);
    }

    private MatchResult? MatchRule(
        string ruleName,
        IReadOnlyList<Token> tokens,
        int pos,
        Dictionary<(string Rule, int Pos), MemoEntry> memo,
        HashSet<(string Rule, int Pos)> recursion)
    {
        if (memo.TryGetValue((ruleName, pos), out var cached))
        {
            return cached.Ok ? new MatchResult(cached.Children!, cached.EndPos) : null;
        }

        if (!recursion.Add((ruleName, pos)))
        {
            memo[(ruleName, pos)] = new MemoEntry(null, pos, false);
            return null;
        }

        try
        {
            if (!_ruleMap.TryGetValue(ruleName, out var rule))
            {
                memo[(ruleName, pos)] = new MemoEntry(null, pos, false);
                return null;
            }

            var result = MatchElement(rule.Body, tokens, pos, memo, recursion);
            if (result is null)
            {
                memo[(ruleName, pos)] = new MemoEntry(null, pos, false);
                return null;
            }

            var wrapped = (IReadOnlyList<object>)[BuildNode(ruleName, result.Children)];
            memo[(ruleName, pos)] = new MemoEntry(wrapped, result.EndPos, true);
            return new MatchResult(wrapped, result.EndPos);
        }
        finally
        {
            recursion.Remove((ruleName, pos));
        }
    }

    private MatchResult? MatchElement(
        GrammarElement element,
        IReadOnlyList<Token> tokens,
        int pos,
        Dictionary<(string Rule, int Pos), MemoEntry> memo,
        HashSet<(string Rule, int Pos)> recursion)
    {
        switch (element)
        {
            case RuleReference reference:
                if (reference.IsToken)
                {
                    return pos < tokens.Count && tokens[pos].EffectiveTypeName == reference.Name
                        ? new MatchResult([tokens[pos]], pos + 1)
                        : null;
                }

                return MatchRule(reference.Name, tokens, pos, memo, recursion);
            case Literal literal:
                return pos < tokens.Count && tokens[pos].Value == literal.Value
                    ? new MatchResult([tokens[pos]], pos + 1)
                    : null;
            case Sequence sequence:
            {
                var children = new List<object>();
                var currentPos = pos;
                foreach (var child in sequence.Elements)
                {
                    var result = MatchElement(child, tokens, currentPos, memo, recursion);
                    if (result is null)
                    {
                        return null;
                    }

                    children.AddRange(result.Children);
                    currentPos = result.EndPos;
                }

                return new MatchResult(children, currentPos);
            }
            case Alternation alternation:
                foreach (var choice in alternation.Choices)
                {
                    var result = MatchElement(choice, tokens, pos, memo, recursion);
                    if (result is not null)
                    {
                        return result;
                    }
                }

                return null;
            case Repetition repetition:
                return MatchRepeated(repetition.Element, tokens, pos, memo, recursion, false);
            case OneOrMoreRepetition repetition:
                return MatchRepeated(repetition.Element, tokens, pos, memo, recursion, true);
            case Optional optional:
            {
                var result = MatchElement(optional.Element, tokens, pos, memo, recursion);
                return result ?? new MatchResult([], pos);
            }
            case Group group:
                return MatchElement(group.Element, tokens, pos, memo, recursion);
            case PositiveLookahead lookahead:
                return MatchElement(lookahead.Element, tokens, pos, memo, recursion) is not null ? new MatchResult([], pos) : null;
            case NegativeLookahead lookahead:
                return MatchElement(lookahead.Element, tokens, pos, memo, recursion) is null ? new MatchResult([], pos) : null;
            case SeparatedRepetition separated:
            {
                var first = MatchElement(separated.Element, tokens, pos, memo, recursion);
                if (first is null)
                {
                    return separated.AtLeastOne ? null : new MatchResult([], pos);
                }

                var children = new List<object>(first.Children);
                var currentPos = first.EndPos;
                while (true)
                {
                    var separator = MatchElement(separated.Separator, tokens, currentPos, memo, recursion);
                    if (separator is null)
                    {
                        break;
                    }

                    var elementResult = MatchElement(separated.Element, tokens, separator.EndPos, memo, recursion);
                    if (elementResult is null)
                    {
                        break;
                    }

                    children.AddRange(separator.Children);
                    children.AddRange(elementResult.Children);
                    currentPos = elementResult.EndPos;
                }

                return new MatchResult(children, currentPos);
            }
            default:
                throw new InvalidOperationException($"Unsupported grammar element: {element.GetType().Name}");
        }
    }

    private MatchResult? MatchRepeated(
        GrammarElement element,
        IReadOnlyList<Token> tokens,
        int pos,
        Dictionary<(string Rule, int Pos), MemoEntry> memo,
        HashSet<(string Rule, int Pos)> recursion,
        bool requireOne)
    {
        var children = new List<object>();
        var currentPos = pos;
        var matchedOne = false;

        while (true)
        {
            var result = MatchElement(element, tokens, currentPos, memo, recursion);
            if (result is null || result.EndPos == currentPos)
            {
                break;
            }

            matchedOne = true;
            children.AddRange(result.Children);
            currentPos = result.EndPos;
        }

        return requireOne && !matchedOne ? null : new MatchResult(children, currentPos);
    }

    private static ASTNode BuildNode(string ruleName, IReadOnlyList<object> children)
    {
        var first = FindFirstToken(children);
        var last = FindLastToken(children);
        return new ASTNode(
            ruleName,
            children,
            first?.Line ?? 0,
            first?.Column ?? 0,
            last?.Line ?? 0,
            last?.Column ?? 0);
    }

    private static Token? FindFirstToken(IEnumerable<object> children)
    {
        foreach (var child in children)
        {
            if (child is Token token)
            {
                return token;
            }

            if (child is ASTNode node)
            {
                var nested = FindFirstToken(node.Children);
                if (nested is not null)
                {
                    return nested;
                }
            }
        }

        return null;
    }

    private static Token? FindLastToken(IEnumerable<object> children)
    {
        foreach (var child in children.Reverse())
        {
            if (child is Token token)
            {
                return token;
            }

            if (child is ASTNode node)
            {
                var nested = FindLastToken(node.Children);
                if (nested is not null)
                {
                    return nested;
                }
            }
        }

        return null;
    }
}
