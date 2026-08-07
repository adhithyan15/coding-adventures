using CodingAdventures.Lexer;

namespace CodingAdventures.XmlLexer;

/// <summary>Tokenizes XML using the token names defined by the shared XML grammar.</summary>
public sealed class XmlLexer
{
    private enum Mode
    {
        Content,
        Tag,
        Comment,
        CData,
        ProcessingInstruction,
    }

    private readonly string _source;
    private int _position;
    private int _line = 1;
    private int _column = 1;
    private bool _precededByNewline;
    private Mode _mode;
    private bool _processingInstructionNeedsTarget;

    /// <summary>Creates a configured XML lexer for <paramref name="source"/>.</summary>
    public XmlLexer(string source)
    {
        ArgumentNullException.ThrowIfNull(source);
        _source = source;
    }

    /// <summary>Tokenizes the configured XML source and appends an EOF token.</summary>
    public IReadOnlyList<Token> Tokenize()
    {
        var tokens = new List<Token>();

        while (_position < _source.Length)
        {
            switch (_mode)
            {
                case Mode.Content:
                    ScanContent(tokens);
                    break;
                case Mode.Tag:
                    ScanTag(tokens);
                    break;
                case Mode.Comment:
                    ScanDelimitedContent(tokens, "-->", "COMMENT_TEXT", "COMMENT_END");
                    break;
                case Mode.CData:
                    ScanDelimitedContent(tokens, "]]>", "CDATA_TEXT", "CDATA_END");
                    break;
                case Mode.ProcessingInstruction:
                    ScanProcessingInstruction(tokens);
                    break;
            }
        }

        tokens.Add(new Token(TokenType.EOF, string.Empty, _line, _column, "EOF"));
        return tokens;
    }

    private void ScanContent(List<Token> tokens)
    {
        if (IsXmlWhitespace(_source[_position]))
        {
            SkipWhitespace();
            return;
        }

        if (TryEmitLiteral(tokens, "<!--", "COMMENT_START"))
        {
            _mode = Mode.Comment;
            return;
        }

        if (TryEmitLiteral(tokens, "<![CDATA[", "CDATA_START"))
        {
            _mode = Mode.CData;
            return;
        }

        if (TryEmitLiteral(tokens, "<?", "PI_START"))
        {
            _mode = Mode.ProcessingInstruction;
            _processingInstructionNeedsTarget = true;
            return;
        }

        if (TryEmitLiteral(tokens, "</", "CLOSE_TAG_START") ||
            TryEmitLiteral(tokens, "<", "OPEN_TAG_START"))
        {
            _mode = Mode.Tag;
            return;
        }

        if (_source[_position] == '&')
        {
            ScanReference(tokens);
            return;
        }

        var end = FindContentEnd(_position);
        Emit(tokens, "TEXT", _source.Substring(_position, end - _position));
    }

    private int FindContentEnd(int start)
    {
        var end = start;
        while (end < _source.Length && _source[end] != '<' && _source[end] != '&')
        {
            end++;
        }

        return end;
    }

    private void ScanTag(List<Token> tokens)
    {
        if (IsXmlWhitespace(_source[_position]))
        {
            SkipWhitespace();
            return;
        }

        if (TryEmitLiteral(tokens, "/>", "SELF_CLOSE"))
        {
            _mode = Mode.Content;
            return;
        }

        if (TryEmitLiteral(tokens, ">", "TAG_CLOSE"))
        {
            _mode = Mode.Content;
            return;
        }

        if (TryEmitLiteral(tokens, "=", "ATTR_EQUALS") ||
            TryEmitLiteral(tokens, "/", "SLASH"))
        {
            return;
        }

        var current = _source[_position];
        if (current is '\'' or '"')
        {
            var end = _source.IndexOf(current, _position + 1);
            if (end < 0)
            {
                ThrowUnexpected();
            }

            Emit(tokens, "ATTR_VALUE", _source.Substring(_position, end - _position + 1));
            return;
        }

        if (IsNameStart(current))
        {
            var end = _position + 1;
            while (end < _source.Length && IsNamePart(_source[end]))
            {
                end++;
            }

            Emit(tokens, "TAG_NAME", _source.Substring(_position, end - _position));
            return;
        }

        ThrowUnexpected();
    }

    private void ScanReference(List<Token> tokens)
    {
        var end = _position + 1;
        var typeName = "ENTITY_REF";

        if (end < _source.Length && _source[end] == '#')
        {
            typeName = "CHAR_REF";
            end++;
            var hexadecimal = end < _source.Length && _source[end] == 'x';
            if (hexadecimal)
            {
                end++;
            }

            var digitStart = end;
            while (end < _source.Length && (hexadecimal ? IsHexDigit(_source[end]) : char.IsAsciiDigit(_source[end])))
            {
                end++;
            }

            if (end == digitStart)
            {
                ThrowUnexpected();
            }
        }
        else
        {
            if (end >= _source.Length || !IsAsciiLetter(_source[end]))
            {
                ThrowUnexpected();
            }

            end++;
            while (end < _source.Length && (IsAsciiLetter(_source[end]) || char.IsAsciiDigit(_source[end])))
            {
                end++;
            }
        }

        if (end >= _source.Length || _source[end] != ';')
        {
            ThrowUnexpected();
        }

        Emit(tokens, typeName, _source.Substring(_position, end - _position + 1));
    }

    private void ScanDelimitedContent(List<Token> tokens, string delimiter, string textType, string endType)
    {
        if (TryEmitLiteral(tokens, delimiter, endType))
        {
            _mode = Mode.Content;
            return;
        }

        var end = _source.IndexOf(delimiter, _position, StringComparison.Ordinal);
        if (end < 0)
        {
            end = _source.Length;
        }

        Emit(tokens, textType, _source.Substring(_position, end - _position));
    }

    private void ScanProcessingInstruction(List<Token> tokens)
    {
        if (TryEmitLiteral(tokens, "?>", "PI_END"))
        {
            _mode = Mode.Content;
            return;
        }

        if (_processingInstructionNeedsTarget && IsNameStart(_source[_position]))
        {
            var end = _position + 1;
            while (end < _source.Length && IsNamePart(_source[end]))
            {
                end++;
            }

            Emit(tokens, "PI_TARGET", _source.Substring(_position, end - _position));
            _processingInstructionNeedsTarget = false;
            return;
        }

        var textEnd = _source.IndexOf("?>", _position, StringComparison.Ordinal);
        if (textEnd < 0)
        {
            textEnd = _source.Length;
        }

        Emit(tokens, "PI_TEXT", _source.Substring(_position, textEnd - _position));
        _processingInstructionNeedsTarget = false;
    }

    private bool TryEmitLiteral(List<Token> tokens, string literal, string typeName)
    {
        if (!_source.AsSpan(_position).StartsWith(literal, StringComparison.Ordinal))
        {
            return false;
        }

        Emit(tokens, typeName, literal);
        return true;
    }

    private void Emit(List<Token> tokens, string typeName, string value)
    {
        var flags = _precededByNewline ? Token.FlagPrecededByNewline : 0;
        tokens.Add(new Token(TokenType.Grammar, value, _line, _column, typeName, flags));
        AdvanceToken(value);
        _precededByNewline = false;
    }

    private void SkipWhitespace()
    {
        while (_position < _source.Length && IsXmlWhitespace(_source[_position]))
        {
            var character = _source[_position++];
            if (character == '\n')
            {
                _line++;
                _column = 1;
                _precededByNewline = true;
            }
            else
            {
                _column++;
            }
        }
    }

    private void AdvanceToken(string value)
    {
        foreach (var character in value)
        {
            if (character == '\n')
            {
                _line++;
                _column = 1;
            }
            else
            {
                _column++;
            }
        }

        _position += value.Length;
    }

    private void ThrowUnexpected() =>
        throw new LexerError($"Unexpected character '{_source[_position]}'", _line, _column);

    private static bool IsXmlWhitespace(char character) => character is ' ' or '\t' or '\r' or '\n';

    private static bool IsAsciiLetter(char character) =>
        character is >= 'A' and <= 'Z' or >= 'a' and <= 'z';

    private static bool IsHexDigit(char character) =>
        char.IsAsciiDigit(character) || character is >= 'A' and <= 'F' or >= 'a' and <= 'f';

    private static bool IsNameStart(char character) => IsAsciiLetter(character) || character == '_';

    private static bool IsNamePart(char character) =>
        IsNameStart(character) || char.IsAsciiDigit(character) || character is ':' or '.' or '-';
}

/// <summary>Convenience factory and one-shot XML tokenization helpers.</summary>
public static class XmlTokenizer
{
    public static XmlLexer CreateXmlLexer(string source) => new(source);

    public static IReadOnlyList<Token> TokenizeXml(string source) => CreateXmlLexer(source).Tokenize();
}
