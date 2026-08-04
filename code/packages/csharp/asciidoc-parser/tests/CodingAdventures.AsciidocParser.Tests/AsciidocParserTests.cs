using CodingAdventures.DocumentAst;
using Parser = CodingAdventures.AsciidocParser.AsciidocParser;

namespace CodingAdventures.AsciidocParser.Tests;

public sealed class AsciidocParserTests
{
    [Fact]
    public void ParseRejectsNull()
    {
        Assert.Throws<ArgumentNullException>(() => Parser.Parse(null!));
        Assert.Throws<ArgumentNullException>(() => Parser.ParseInline(null!));
    }

    [Fact]
    public void EmptyBlankAndCommentOnlySourcesProduceEmptyDocuments()
    {
        Assert.Empty(Parser.Parse(string.Empty).Children);
        Assert.Empty(Parser.Parse("  \n\t\n").Children);
        Assert.Empty(Parser.Parse("// hidden\n// also hidden").Children);
    }

    [Fact]
    public void HeadingsClampLevelsAndParseInlineMarkup()
    {
        var document = Parser.Parse("= One\n== Two\n======= *Deep*");
        var first = Assert.IsType<HeadingNode>(document.Children[0]);
        var second = Assert.IsType<HeadingNode>(document.Children[1]);
        var deep = Assert.IsType<HeadingNode>(document.Children[2]);
        Assert.Equal(1, first.Level);
        Assert.Equal(2, second.Level);
        Assert.Equal(6, deep.Level);
        Assert.IsType<StrongNode>(Assert.Single(deep.Children));
    }

    [Fact]
    public void ParagraphsPreserveSoftBreaksAndBlankLineBoundaries()
    {
        var document = Parser.Parse("first line\nsecond line\n\nnext");
        Assert.Equal(2, document.Children.Count);
        var first = Assert.IsType<ParagraphNode>(document.Children[0]);
        Assert.Collection(
            first.Children,
            node => Assert.Equal("first line", Assert.IsType<TextNode>(node).Value),
            node => Assert.IsType<SoftBreakNode>(node),
            node => Assert.Equal("second line", Assert.IsType<TextNode>(node).Value));
        Assert.Equal("next", Assert.IsType<TextNode>(Assert.Single(Assert.IsType<ParagraphNode>(document.Children[1]).Children)).Value);
    }

    [Fact]
    public void ThematicBreakAndFollowingHeadingInterruptParagraphs()
    {
        var document = Parser.Parse("paragraph\n'''\n== Heading");
        Assert.IsType<ParagraphNode>(document.Children[0]);
        Assert.IsType<ThematicBreakNode>(document.Children[1]);
        Assert.IsType<HeadingNode>(document.Children[2]);
    }

    [Fact]
    public void SourceAndLiteralBlocksPreserveContent()
    {
        var document = Parser.Parse("[source, csharp]\n----\nvar x = 1;\n----\n....\nliteral\n....");
        var source = Assert.IsType<CodeBlockNode>(document.Children[0]);
        var literal = Assert.IsType<CodeBlockNode>(document.Children[1]);
        Assert.Equal("csharp", source.Language);
        Assert.Equal("var x = 1;\n", source.Value);
        Assert.Null(literal.Language);
        Assert.Equal("literal\n", literal.Value);
    }

    [Fact]
    public void UnterminatedCodeBlockIsEmittedLeniently()
    {
        var block = Assert.IsType<CodeBlockNode>(Assert.Single(Parser.Parse("----\nopen").Children));
        Assert.Equal("open\n", block.Value);
    }

    [Fact]
    public void PassthroughAndQuoteBlocksProduceRawAndRecursiveNodes()
    {
        var document = Parser.Parse("++++\n<b>raw</b>\n++++\n____\n= Nested\n\ntext\n____");
        var raw = Assert.IsType<RawBlockNode>(document.Children[0]);
        Assert.Equal("html", raw.Format);
        Assert.Equal("<b>raw</b>", raw.Value);
        var quote = Assert.IsType<BlockquoteNode>(document.Children[1]);
        Assert.IsType<HeadingNode>(quote.Children[0]);
        Assert.IsType<ParagraphNode>(quote.Children[1]);
    }

    [Fact]
    public void OrderedUnorderedAndNestedListsAreBuilt()
    {
        var document = Parser.Parse("* one\n** nested\n* two\n\n. first\n. second");
        var unordered = Assert.IsType<ListNode>(document.Children[0]);
        Assert.False(unordered.Ordered);
        Assert.Null(unordered.Start);
        Assert.Equal(2, unordered.Children.Count);
        var firstItem = Assert.IsType<ListItemNode>(unordered.Children[0]);
        Assert.IsType<ListNode>(firstItem.Children[1]);

        var ordered = Assert.IsType<ListNode>(document.Children[1]);
        Assert.True(ordered.Ordered);
        Assert.Equal(1, ordered.Start);
        Assert.Equal(2, ordered.Children.Count);
    }

    [Fact]
    public void ListsTerminateWhenAnotherBlockStarts()
    {
        var document = Parser.Parse("* item\nparagraph\n\n. ordered\n== Heading");
        Assert.IsType<ListNode>(document.Children[0]);
        Assert.IsType<ParagraphNode>(document.Children[1]);
        Assert.IsType<ListNode>(document.Children[2]);
        Assert.IsType<HeadingNode>(document.Children[3]);
    }

    [Fact]
    public void InlineParserHandlesStrongEmphasisCodeAndLiteralDelimiters()
    {
        var nodes = Parser.ParseInline("a *bold* **wide** _ital_ __free__ `*code*` and *open");
        Assert.Contains(nodes, node => node is StrongNode);
        Assert.Equal(2, nodes.Count(node => node is StrongNode));
        Assert.Equal(2, nodes.Count(node => node is EmphasisNode));
        Assert.Equal("*code*", Assert.IsType<CodeSpanNode>(nodes.Single(node => node is CodeSpanNode)).Value);
        Assert.EndsWith("*open", Assert.IsType<TextNode>(nodes[^1]).Value);
    }

    [Fact]
    public void InlineParserHandlesLinksImagesCrossReferencesAndUrls()
    {
        var nodes = Parser.ParseInline(
            "link:https://a.test[A] image:cat.png[Cat] <<intro,Intro>> <<plain>> https://b.test[B] http://c.test");
        var links = nodes.OfType<LinkNode>().ToArray();
        Assert.Equal(4, links.Length);
        Assert.Equal("https://a.test", links[0].Destination);
        Assert.Equal("#intro", links[1].Destination);
        Assert.Equal("#plain", links[2].Destination);
        Assert.Equal("https://b.test", links[3].Destination);
        Assert.Equal("Cat", Assert.Single(nodes.OfType<ImageNode>()).Alt);
        Assert.Equal("http://c.test", Assert.Single(nodes.OfType<AutolinkNode>()).Destination);
    }

    [Fact]
    public void EmptyLinkLabelsUseTheirDestination()
    {
        var macro = Assert.IsType<LinkNode>(Assert.Single(Parser.ParseInline("link:https://a.test[]")));
        var url = Assert.IsType<LinkNode>(Assert.Single(Parser.ParseInline("https://b.test[]")));
        Assert.Equal("https://a.test", Assert.IsType<TextNode>(Assert.Single(macro.Children)).Value);
        Assert.Equal("https://b.test", Assert.IsType<TextNode>(Assert.Single(url.Children)).Value);
    }

    [Fact]
    public void InlineParserClassifiesHardAndSoftBreaks()
    {
        var nodes = Parser.ParseInline("a  \nb\\\nc\nd");
        Assert.Equal(2, nodes.Count(node => node is HardBreakNode));
        Assert.Single(nodes.OfType<SoftBreakNode>());
    }

    [Fact]
    public void InlineMarkupCanNestRecursively()
    {
        var strong = Assert.IsType<StrongNode>(Assert.Single(Parser.ParseInline("*bold _and italic_*")));
        Assert.IsType<EmphasisNode>(strong.Children[1]);
    }

    [Fact]
    public void CrLfAndBareCarriageReturnsAreNormalized()
    {
        var document = Parser.Parse("first\r\nsecond\r\rthird");
        Assert.Equal(2, document.Children.Count);
        Assert.Equal(3, Assert.IsType<ParagraphNode>(document.Children[0]).Children.Count);
    }

    [Fact]
    public void EmptyDelimitedBlocksProduceEmptyValues()
    {
        var document = Parser.Parse("----\n----\n....\n....\n++++\n++++");
        Assert.Equal(string.Empty, Assert.IsType<CodeBlockNode>(document.Children[0]).Value);
        Assert.Equal(string.Empty, Assert.IsType<CodeBlockNode>(document.Children[1]).Value);
        Assert.Equal(string.Empty, Assert.IsType<RawBlockNode>(document.Children[2]).Value);
    }

    [Fact]
    public void PassthroughBlocksDoNotAddTrailingNewlines()
    {
        var raw = Assert.IsType<RawBlockNode>(Assert.Single(Parser.Parse("++++\nline one\nline two\n++++").Children));
        Assert.Equal("line one\nline two", raw.Value);
    }

    [Fact]
    public void UnterminatedPassthroughBlocksAreEmittedLeniently()
    {
        var raw = Assert.IsType<RawBlockNode>(Assert.Single(Parser.Parse("++++\nraw").Children));
        Assert.Equal("raw", raw.Value);
    }

    [Fact]
    public void UnterminatedQuoteBlocksAreParsedRecursively()
    {
        var quote = Assert.IsType<BlockquoteNode>(Assert.Single(Parser.Parse("____\n== Inner").Children));
        Assert.IsType<HeadingNode>(Assert.Single(quote.Children));
    }

    [Fact]
    public void ListsInterruptParagraphsWithoutBlankLines()
    {
        var document = Parser.Parse("paragraph\n* item\n. ordered");
        Assert.IsType<ParagraphNode>(document.Children[0]);
        Assert.IsType<ListNode>(document.Children[1]);
        Assert.IsType<ListNode>(document.Children[2]);
    }

    [Fact]
    public void CommentsAndSourceAttributesInterruptParagraphs()
    {
        var document = Parser.Parse("paragraph\n// hidden\n[source, go]\n----\ncode\n----");
        Assert.Equal(2, document.Children.Count);
        Assert.IsType<ParagraphNode>(document.Children[0]);
        Assert.Equal("go", Assert.IsType<CodeBlockNode>(document.Children[1]).Language);
    }

    [Fact]
    public void SwitchingListKindsCreatesSeparateLists()
    {
        var document = Parser.Parse("* unordered\n. ordered\n* unordered again");
        Assert.Equal(3, document.Children.Count);
        Assert.False(Assert.IsType<ListNode>(document.Children[0]).Ordered);
        Assert.True(Assert.IsType<ListNode>(document.Children[1]).Ordered);
        Assert.False(Assert.IsType<ListNode>(document.Children[2]).Ordered);
    }

    [Fact]
    public void ListsSupportMultipleNestedLevels()
    {
        var root = Assert.IsType<ListNode>(Assert.Single(Parser.Parse("* one\n** two\n*** three").Children));
        var levelTwo = Assert.IsType<ListNode>(Assert.IsType<ListItemNode>(Assert.Single(root.Children)).Children[1]);
        Assert.IsType<ListNode>(Assert.IsType<ListItemNode>(Assert.Single(levelTwo.Children)).Children[1]);
    }

    [Fact]
    public void SourceAttributesAreCaseInsensitiveAndTrimLanguages()
    {
        var block = Assert.IsType<CodeBlockNode>(Assert.Single(Parser.Parse("[SOURCE,  rust ]\n----\nx\n----").Children));
        Assert.Equal("rust", block.Language);
    }

    [Fact]
    public void HeadingAndListContentTrimMarkerWhitespace()
    {
        var document = Parser.Parse("==   Heading\n\n*   item  ");
        Assert.Equal("Heading", Assert.IsType<TextNode>(Assert.Single(Assert.IsType<HeadingNode>(document.Children[0]).Children)).Value);
        var item = Assert.IsType<ListItemNode>(Assert.Single(Assert.IsType<ListNode>(document.Children[1]).Children));
        Assert.Equal("item", Assert.IsType<TextNode>(Assert.Single(Assert.IsType<ParagraphNode>(item.Children[0]).Children)).Value);
    }

    [Fact]
    public void LongDelimitersAreAccepted()
    {
        var document = Parser.Parse("'''''\n-----\nx\n-----");
        Assert.IsType<ThematicBreakNode>(document.Children[0]);
        Assert.IsType<CodeBlockNode>(document.Children[1]);
    }

    [Fact]
    public void PlainInlineInputCoalescesIntoOneTextNode()
    {
        Assert.Equal("just plain text", Assert.IsType<TextNode>(Assert.Single(Parser.ParseInline("just plain text"))).Value);
    }

    [Theory]
    [InlineData("*open")]
    [InlineData("_open")]
    [InlineData("`open")]
    [InlineData("<<open")]
    public void UnterminatedInlineConstructsRemainLiteral(string input)
    {
        Assert.Equal(input, Assert.IsType<TextNode>(Assert.Single(Parser.ParseInline(input))).Value);
    }

    [Fact]
    public void MalformedMacrosRemainLiteralText()
    {
        Assert.Equal("link:target[open", Assert.IsType<TextNode>(Assert.Single(Parser.ParseInline("link:target[open"))).Value);
        Assert.Equal("image:cat.png", Assert.IsType<TextNode>(Assert.Single(Parser.ParseInline("image:cat.png"))).Value);
    }
}
