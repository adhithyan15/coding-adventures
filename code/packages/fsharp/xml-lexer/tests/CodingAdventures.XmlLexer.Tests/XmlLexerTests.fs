namespace CodingAdventures.XmlLexer.FSharp.Tests

open System
open CodingAdventures.Lexer.FSharp
open CodingAdventures.XmlLexer.FSharp
open Xunit

module XmlLexerTests =
    let private pairs source =
        XmlTokenizer.tokenizeXml source
        |> Seq.filter (fun token -> token.Type <> TokenType.EOF)
        |> Seq.map (fun token -> token.EffectiveTypeName, token.Value)
        |> Seq.toArray

    [<Fact>]
    let ``tokenizes simple namespaced and self-closing elements`` () =
        Assert.Equal<(string * string) array>(
            [|
                "OPEN_TAG_START", "<"
                "TAG_NAME", "ns:tag"
                "TAG_CLOSE", ">"
                "TEXT", "text"
                "CLOSE_TAG_START", "</"
                "TAG_NAME", "ns:tag"
                "TAG_CLOSE", ">"
                "OPEN_TAG_START", "<"
                "TAG_NAME", "br"
                "SELF_CLOSE", "/>"
            |],
            pairs "<ns:tag>text</ns:tag><br />")

    [<Fact>]
    let ``tokenizes single and double quoted attributes`` () =
        Assert.Equal<(string * string) array>(
            [|
                "OPEN_TAG_START", "<"
                "TAG_NAME", "div"
                "TAG_NAME", "id"
                "ATTR_EQUALS", "="
                "ATTR_VALUE", "\"main\""
                "TAG_NAME", "data-v"
                "ATTR_EQUALS", "="
                "ATTR_VALUE", "'x'"
                "TAG_CLOSE", ">"
            |],
            pairs "<div id=\"main\" data-v='x'>")

    [<Fact>]
    let ``preserves comment and CDATA content`` () =
        Assert.Equal<(string * string) array>(
            [|
                "COMMENT_START", "<!--"
                "COMMENT_TEXT", "  a-b\t "
                "COMMENT_END", "-->"
                "CDATA_START", "<![CDATA["
                "CDATA_TEXT", " x < y\n "
                "CDATA_END", "]]>"
            |],
            pairs "<!--  a-b\t --><![CDATA[ x < y\n ]]>")

    [<Fact>]
    let ``tokenizes processing instructions`` () =
        Assert.Equal<(string * string) array>(
            [|
                "PI_START", "<?"
                "PI_TARGET", "xml-stylesheet"
                "PI_TEXT", " type=\"text/xsl\""
                "PI_END", "?>"
            |],
            pairs "<?xml-stylesheet type=\"text/xsl\"?>")

    [<Fact>]
    let ``tokenizes named decimal and hexadecimal references`` () =
        Assert.Equal<(string * string) array>(
            [|
                "TEXT", "a"
                "ENTITY_REF", "&amp;"
                "CHAR_REF", "&#65;"
                "CHAR_REF", "&#x41;"
                "TEXT", "b"
            |],
            pairs "a&amp;&#65;&#x41;b")

    [<Fact>]
    let ``skips whitespace that starts a content or tag match`` () =
        let tokens = XmlTokenizer.tokenizeXml "\n  <a> <b/></a>"
        Assert.DoesNotContain(tokens, fun token -> token.EffectiveTypeName = "TEXT")
        Assert.True(tokens[0].HasFlag(Token.FlagPrecededByNewline))
        Assert.Equal(2, tokens[0].Line)
        Assert.Equal(3, tokens[0].Column)

    [<Fact>]
    let ``text may contain whitespace after its first character`` () =
        Assert.Equal<(string * string) array>([| "TEXT", "Hello world " |], pairs "Hello world ")

    [<Fact>]
    let ``empty and unterminated delimited inputs follow grammar lexer behavior`` () =
        let emptyTokens = XmlTokenizer.tokenizeXml String.Empty
        Assert.Equal("EOF", emptyTokens[0].EffectiveTypeName)

        Assert.Equal<(string * string) array>(
            [| "COMMENT_START", "<!--"; "COMMENT_TEXT", "open" |],
            pairs "<!--open")

        Assert.Equal<(string * string) array>(
            [| "CDATA_START", "<![CDATA["; "CDATA_TEXT", "open" |],
            pairs "<![CDATA[open")

        Assert.Equal<(string * string) array>(
            [| "PI_START", "<?"; "PI_TARGET", "open" |],
            pairs "<?open")

    [<Fact>]
    let ``emits slash token inside tags`` () =
        Assert.Contains(("SLASH", "/"), pairs "</a/ >")

    [<Theory>]
    [<InlineData("&bad")>]
    [<InlineData("&#;")>]
    [<InlineData("&#x;")>]
    [<InlineData("<a value=\"unterminated>")>]
    let ``rejects input that matches no grammar pattern`` source =
        let error = Assert.Throws<LexerError>(fun () -> XmlTokenizer.tokenizeXml source |> ignore)
        Assert.True(error.Line >= 1)
        Assert.True(error.Column >= 1)

    [<Fact>]
    let ``factory returns configured lexer object`` () =
        let tokens = (XmlTokenizer.createXmlLexer "<root/>").Tokenize()
        Assert.Equal("EOF", tokens[tokens.Count - 1].EffectiveTypeName)
