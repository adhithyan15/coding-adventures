using CodingAdventures.Lexer;

namespace CodingAdventures.XmlLexer.Tests;

public sealed class XmlLexerTests
{
    [Fact]
    public void TokenizesSimpleNamespacedAndSelfClosingElements()
    {
        Assert.Equal(
            [
                ("OPEN_TAG_START", "<"), ("TAG_NAME", "ns:tag"), ("TAG_CLOSE", ">"),
                ("TEXT", "text"), ("CLOSE_TAG_START", "</"), ("TAG_NAME", "ns:tag"),
                ("TAG_CLOSE", ">"), ("OPEN_TAG_START", "<"), ("TAG_NAME", "br"),
                ("SELF_CLOSE", "/>"),
            ],
            Pairs("<ns:tag>text</ns:tag><br />"));
    }

    [Fact]
    public void TokenizesSingleAndDoubleQuotedAttributes()
    {
        Assert.Equal(
            [
                ("OPEN_TAG_START", "<"), ("TAG_NAME", "div"), ("TAG_NAME", "id"),
                ("ATTR_EQUALS", "="), ("ATTR_VALUE", "\"main\""), ("TAG_NAME", "data-v"),
                ("ATTR_EQUALS", "="), ("ATTR_VALUE", "'x'"), ("TAG_CLOSE", ">"),
            ],
            Pairs("<div id=\"main\" data-v='x'>"));
    }

    [Fact]
    public void PreservesCommentAndCDataContent()
    {
        Assert.Equal(
            [
                ("COMMENT_START", "<!--"), ("COMMENT_TEXT", "  a-b\t "), ("COMMENT_END", "-->"),
                ("CDATA_START", "<![CDATA["), ("CDATA_TEXT", " x < y\n "), ("CDATA_END", "]]>"),
            ],
            Pairs("<!--  a-b\t --><![CDATA[ x < y\n ]]>"));
    }

    [Fact]
    public void TokenizesProcessingInstructions()
    {
        Assert.Equal(
            [
                ("PI_START", "<?"), ("PI_TARGET", "xml-stylesheet"),
                ("PI_TEXT", " type=\"text/xsl\""), ("PI_END", "?>"),
            ],
            Pairs("<?xml-stylesheet type=\"text/xsl\"?>"));
    }

    [Fact]
    public void TokenizesNamedDecimalAndHexReferences()
    {
        Assert.Equal(
            [
                ("TEXT", "a"), ("ENTITY_REF", "&amp;"), ("CHAR_REF", "&#65;"),
                ("CHAR_REF", "&#x41;"), ("TEXT", "b"),
            ],
            Pairs("a&amp;&#65;&#x41;b"));
    }

    [Fact]
    public void SkipsWhitespaceThatStartsAContentOrTagMatch()
    {
        var tokens = XmlTokenizer.TokenizeXml("\n  <a> <b/></a>");
        Assert.DoesNotContain(tokens, token => token.EffectiveTypeName == "TEXT");
        Assert.True(tokens[0].HasFlag(Token.FlagPrecededByNewline));
        Assert.Equal((2, 3), (tokens[0].Line, tokens[0].Column));
    }

    [Fact]
    public void TextMayContainWhitespaceAfterItsFirstCharacter()
    {
        Assert.Equal([("TEXT", "Hello world ")], Pairs("Hello world "));
    }

    [Fact]
    public void EmptyAndUnterminatedDelimitedInputsFollowGrammarLexerBehavior()
    {
        Assert.Equal("EOF", Assert.Single(XmlTokenizer.TokenizeXml(string.Empty)).EffectiveTypeName);
        Assert.Equal(
            [("COMMENT_START", "<!--"), ("COMMENT_TEXT", "open" )],
            Pairs("<!--open"));
        Assert.Equal(
            [("CDATA_START", "<![CDATA["), ("CDATA_TEXT", "open")],
            Pairs("<![CDATA[open"));
        Assert.Equal(
            [("PI_START", "<?"), ("PI_TARGET", "open")],
            Pairs("<?open"));
    }

    [Fact]
    public void EmitsSlashTokenInsideTags()
    {
        Assert.Contains(("SLASH", "/"), Pairs("</a/ >"));
    }

    [Theory]
    [InlineData("&bad")]
    [InlineData("&#;")]
    [InlineData("&#x;")]
    [InlineData("<a value=\"unterminated>")]
    public void RejectsInputThatMatchesNoGrammarPattern(string source)
    {
        var error = Assert.Throws<LexerError>(() => XmlTokenizer.TokenizeXml(source));
        Assert.True(error.Line >= 1);
        Assert.True(error.Column >= 1);
    }

    [Fact]
    public void FactoryReturnsReusableLexerObject()
    {
        var lexer = XmlTokenizer.CreateXmlLexer("<root/>");
        Assert.Equal("EOF", lexer.Tokenize()[^1].EffectiveTypeName);
    }

    private static (string Type, string Value)[] Pairs(string source) =>
        XmlTokenizer.TokenizeXml(source)
            .Where(token => token.Type != TokenType.EOF)
            .Select(token => (token.EffectiveTypeName, token.Value))
            .ToArray();
}
