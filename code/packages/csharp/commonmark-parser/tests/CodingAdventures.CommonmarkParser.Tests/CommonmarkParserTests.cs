using CodingAdventures.CommonmarkParser;
using CodingAdventures.DocumentAst;

namespace CodingAdventures.CommonmarkParser.Tests;

public class CommonmarkParserTests
{
    [Fact]
    public void Parse_ReturnsEmptyDocumentForEmptyInput()
    {
        var doc = CommonmarkParser.Parse(string.Empty);
        Assert.Empty(doc.Children);
    }

    [Fact]
    public void Parse_ParsesAtxAndSetextHeadings()
    {
        var doc = CommonmarkParser.Parse("# Hello\n\nWorld\n---\n");

        var atx = Assert.IsType<HeadingNode>(doc.Children[0]);
        var setext = Assert.IsType<HeadingNode>(doc.Children[1]);

        Assert.Equal(1, atx.Level);
        Assert.Equal(2, setext.Level);
    }

    [Fact]
    public void Parse_ParsesInlineFormattingLinksImagesAndEntities()
    {
        var doc = CommonmarkParser.Parse("A *small* **test** with [link](https://example.com \"Title\") ![cat](cat.png) &amp; `code`.");
        var paragraph = Assert.IsType<ParagraphNode>(Assert.Single(doc.Children));

        Assert.Contains(paragraph.Children, node => node is EmphasisNode);
        Assert.Contains(paragraph.Children, node => node is StrongNode);
        Assert.Contains(paragraph.Children, node => node is LinkNode link && link.Destination == "https://example.com");
        Assert.Contains(paragraph.Children, node => node is ImageNode image && image.Alt == "cat");
        Assert.Contains(paragraph.Children, node => node is CodeSpanNode code && code.Value == "code");
        Assert.Contains(paragraph.Children, node => node is TextNode text && text.Value.Contains("&"));
    }

    [Fact]
    public void Parse_ParsesAutolinksAndBreaks()
    {
        var doc = CommonmarkParser.Parse("<https://example.com>\nline  \nnext");
        var paragraph = Assert.IsType<ParagraphNode>(Assert.Single(doc.Children));

        Assert.IsType<AutolinkNode>(paragraph.Children[0]);
        Assert.Contains(paragraph.Children, node => node is SoftBreakNode);
        Assert.Contains(paragraph.Children, node => node is HardBreakNode);
    }

    [Fact]
    public void Parse_ParsesListsAndBlockquotes()
    {
        var doc = CommonmarkParser.Parse("> quoted\n>\n> again\n\n- one\n- two");

        var quote = Assert.IsType<BlockquoteNode>(doc.Children[0]);
        var list = Assert.IsType<ListNode>(doc.Children[1]);

        Assert.False(list.Ordered);
        Assert.Equal(2, list.Children.Count);
        Assert.Equal(2, quote.Children.Count);
    }

    [Fact]
    public void Parse_ParsesOrderedListFenceAndThematicBreak()
    {
        var doc = CommonmarkParser.Parse("1. one\n2. two\n\n---\n\n```csharp\nConsole.WriteLine(1);\n```");

        var list = Assert.IsType<ListNode>(doc.Children[0]);
        Assert.True(list.Ordered);
        Assert.Equal(1, list.Start);
        Assert.IsType<ThematicBreakNode>(doc.Children[1]);

        var code = Assert.IsType<CodeBlockNode>(doc.Children[2]);
        Assert.Equal("csharp", code.Language);
        Assert.Contains("Console.WriteLine", code.Value);
    }

    [Fact]
    public void Parse_ParsesSimpleRawHtmlBlock()
    {
        var doc = CommonmarkParser.Parse("<aside>hello</aside>");
        var raw = Assert.IsType<RawBlockNode>(Assert.Single(doc.Children));
        Assert.Equal("html", raw.Format);
    }

    [Fact]
    public void Parse_RejectsExcessiveBlockNesting()
    {
        var deeplyNested = string.Concat(Enumerable.Repeat("> ", 65)) + "boom";
        Assert.Throws<InvalidOperationException>(() => CommonmarkParser.Parse(deeplyNested));
    }

    [Fact]
    public void Parse_TreatsOversizedOrderedMarkerAsParagraph()
    {
        var doc = CommonmarkParser.Parse("999999999999999999999. item");
        var paragraph = Assert.IsType<ParagraphNode>(Assert.Single(doc.Children));
        Assert.Contains(paragraph.Children, node => node is TextNode text && text.Value.Contains("999999999999999999999. item"));
    }

    [Fact]
    public void Parse_HandlesDelimiterHeavyMalformedInputWithoutFailure()
    {
        var malformed = new string('[', 5000);
        var doc = CommonmarkParser.Parse(malformed);
        Assert.IsType<ParagraphNode>(Assert.Single(doc.Children));
    }
}
