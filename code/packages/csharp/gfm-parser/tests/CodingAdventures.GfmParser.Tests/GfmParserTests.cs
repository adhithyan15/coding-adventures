using CodingAdventures.DocumentAst;
using CodingAdventures.GfmParser;

namespace CodingAdventures.GfmParser.Tests;

public class GfmParserTests
{
    [Fact]
    public void Parse_ParsesTaskListItems()
    {
        var doc = GfmParser.Parse("- [x] done\n- [ ] todo");
        var list = Assert.IsType<ListNode>(Assert.Single(doc.Children));

        Assert.All(list.Children, child => Assert.IsType<TaskItemNode>(child));
        Assert.True(((TaskItemNode)list.Children[0]).Checked);
        Assert.False(((TaskItemNode)list.Children[1]).Checked);
    }

    [Fact]
    public void Parse_ParsesTablesWithAlignment()
    {
        var doc = GfmParser.Parse("| Left | Right |\n| :--- | ---: |\n| A | B |");
        var table = Assert.IsType<TableNode>(Assert.Single(doc.Children));

        Assert.Equal(TableAlignment.Left, table.Align[0]);
        Assert.Equal(TableAlignment.Right, table.Align[1]);
        Assert.True(table.Children[0].IsHeader);
        Assert.False(table.Children[1].IsHeader);
    }

    [Fact]
    public void Parse_ParsesStrikethrough()
    {
        var doc = GfmParser.Parse("~~gone~~");
        var paragraph = Assert.IsType<ParagraphNode>(Assert.Single(doc.Children));
        Assert.IsType<StrikethroughNode>(Assert.Single(paragraph.Children));
    }
}
