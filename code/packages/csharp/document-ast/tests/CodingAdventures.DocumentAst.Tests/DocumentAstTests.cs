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
        Assert.Equal("table_cell", table.Children[0].Children[0].Type);
        Assert.Equal("strikethrough", strike.Type);
    }
}
