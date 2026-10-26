using CodingAdventures.DocumentAst;

namespace CodingAdventures.DocumentAst.Tests;

public class DocumentAstTests
{
    [Fact]
    public void CoreNodesExposeStableTypeNames()
    {
        var doc = new DocumentNode(
            new IBlockNode[]
            {
                new HeadingNode(2, new IInlineNode[] { new TextNode("Hello") }),
                new ParagraphNode(new IInlineNode[] { new TextNode("World") }),
            });

        Assert.Equal("document", doc.Type);
        Assert.Equal("heading", ((HeadingNode)doc.Children[0]).Type);
        Assert.Equal("paragraph", ((ParagraphNode)doc.Children[1]).Type);
    }

    [Fact]
    public void ListNodesRetainOrderedStartAndTaskShape()
    {
        var list = new ListNode(
            true,
            3,
            false,
            new IListChildNode[]
            {
                new ListItemNode(new IBlockNode[] { new ParagraphNode(new IInlineNode[] { new TextNode("alpha") }) }),
                new TaskItemNode(true, new IBlockNode[] { new ParagraphNode(new IInlineNode[] { new TextNode("beta") }) }),
            });

        Assert.True(list.Ordered);
        Assert.Equal(3, list.Start);
        Assert.False(list.Tight);
        Assert.Equal("list_item", list.Children[0].Type);
        Assert.Equal("task_item", list.Children[1].Type);
    }

    [Fact]
    public void InlineNodesModelLinksImagesAndBreaks()
    {
        var link = new LinkNode("https://example.com", "Example", new IInlineNode[] { new TextNode("Example") });
        var image = new ImageNode("cat.png", null, "Cat");

        Assert.Equal("link", link.Type);
        Assert.Equal("https://example.com", link.Destination);
        Assert.Equal("image", image.Type);
        Assert.Equal("Cat", image.Alt);
        Assert.Equal("hard_break", new HardBreakNode().Type);
        Assert.Equal("soft_break", new SoftBreakNode().Type);
    }

    [Fact]
    public void GfmExtensionsModelTablesAndStrikethrough()
    {
        var table = new TableNode(
            new TableAlignment?[] { TableAlignment.Left, null, TableAlignment.Right },
            new[]
            {
                new TableRowNode(true, new[] { new TableCellNode(new IInlineNode[] { new TextNode("A") }) }),
                new TableRowNode(false, new[] { new TableCellNode(new IInlineNode[] { new TextNode("B") }) }),
            });

        var strike = new StrikethroughNode(new IInlineNode[] { new TextNode("gone") });

        Assert.Equal("table", table.Type);
        Assert.Equal(TableAlignment.Left, table.Align[0]);
        Assert.True(table.Children[0].IsHeader);
        Assert.Equal("table_row", table.Children[0].Type);
        Assert.Equal("table_cell", table.Children[0].Children[0].Type);
        Assert.Equal("strikethrough", strike.Type);
    }

    [Fact]
    public void RemainingBlockNodesExposeTheirDataAndStableTypeNames()
    {
        var paragraph = new ParagraphNode(new IInlineNode[] { new TextNode("quoted") });
        var code = new CodeBlockNode("csharp", "return 0;");
        var quote = new BlockquoteNode(new IBlockNode[] { paragraph });
        var list = new ListNode(false, null, true, Array.Empty<IListChildNode>());
        var thematicBreak = new ThematicBreakNode();
        var raw = new RawBlockNode("html", "<aside></aside>");

        Assert.Equal("code_block", code.Type);
        Assert.Equal("csharp", code.Language);
        Assert.Equal("return 0;", code.Value);
        Assert.Equal("blockquote", quote.Type);
        Assert.Same(paragraph, quote.Children[0]);
        Assert.Equal("list", list.Type);
        Assert.Empty(list.Children);
        Assert.Equal("thematic_break", thematicBreak.Type);
        Assert.Equal("raw_block", raw.Type);
        Assert.Equal("html", raw.Format);
        Assert.Equal("<aside></aside>", raw.Value);
    }

    [Fact]
    public void RemainingInlineNodesExposeTheirDataAndStableTypeNames()
    {
        var text = new TextNode("text");
        var emphasis = new EmphasisNode(new IInlineNode[] { text });
        var strong = new StrongNode(new IInlineNode[] { text });
        var code = new CodeSpanNode("value");
        var autolink = new AutolinkNode("mailto:reader@example.com", true);
        var raw = new RawInlineNode("html", "<mark>text</mark>");

        Assert.Equal("text", text.Type);
        Assert.Equal("emphasis", emphasis.Type);
        Assert.Same(text, emphasis.Children[0]);
        Assert.Equal("strong", strong.Type);
        Assert.Same(text, strong.Children[0]);
        Assert.Equal("code_span", code.Type);
        Assert.Equal("value", code.Value);
        Assert.Equal("autolink", autolink.Type);
        Assert.True(autolink.IsEmail);
        Assert.Equal("mailto:reader@example.com", autolink.Destination);
        Assert.Equal("raw_inline", raw.Type);
        Assert.Equal("html", raw.Format);
        Assert.Equal("<mark>text</mark>", raw.Value);
    }
}
