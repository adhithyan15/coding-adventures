using CodingAdventures.DocumentAst;
using Renderer = CodingAdventures.DocumentAstToHtml.DocumentAstToHtml;

namespace CodingAdventures.DocumentAstToHtml.Tests;

/// <summary>
/// These vectors are written from TE00 rather than copied from another
/// renderer.  Each assertion enters through the public renderer so the tests
/// describe the contract a parser, editor, or importer can rely on.
/// </summary>
public sealed class DocumentAstToHtmlTests
{
    private static DocumentNode Document(params IBlockNode[] children) => new(children);

    private static ParagraphNode Paragraph(params IInlineNode[] children) => new(children);

    private static TextNode Text(string value) => new(value);

    private sealed class UnknownBlockNode : IBlockNode
    {
        public string Type => "unknown_block";
    }

    private sealed class UnknownListChildNode : IListChildNode
    {
        public string Type => "unknown_list_child";
    }

    private sealed class UnknownInlineNode : IInlineNode
    {
        public string Type => "unknown_inline";
    }

    [Fact]
    public void ToHtml_RendersCoreBlockNodesInOrder()
    {
        var document = Document(
            new HeadingNode(2, [Text("A & B")]),
            Paragraph(Text("Hello "), new EmphasisNode([Text("small")]), Text(" "), new StrongNode([Text("world")]), Text(".")),
            new ThematicBreakNode(),
            new CodeBlockNode("csharp\"<&", "if (a < b && c > d)\n"),
            new BlockquoteNode([Paragraph(Text("quoted"))]));

        Assert.Equal(
            "<h2>A &amp; B</h2>\n" +
            "<p>Hello <em>small</em> <strong>world</strong>.</p>\n" +
            "<hr />\n" +
            "<pre><code class=\"language-csharp&quot;&lt;&amp;\">if (a &lt; b &amp;&amp; c &gt; d)\n</code></pre>\n" +
            "<blockquote>\n<p>quoted</p>\n</blockquote>\n",
            Renderer.ToHtml(document));
    }

    [Fact]
    public void ToHtml_RendersTightLooseOrderedAndEmptyLists()
    {
        var document = Document(
            new ListNode(false, null, true,
            [
                new ListItemNode([Paragraph(Text("one"))]),
                new ListItemNode([]),
            ]),
            new ListNode(true, 42, false,
            [
                new ListItemNode([Paragraph(Text("forty-two"))]),
            ]));

        Assert.Equal(
            "<ul>\n<li>one</li>\n<li></li>\n</ul>\n" +
            "<ol start=\"42\">\n<li>\n<p>forty-two</p>\n</li>\n</ol>\n",
            Renderer.ToHtml(document));
    }

    [Fact]
    public void ToHtml_RendersMultiBlockTightAndDirectListItems()
    {
        var tight = new ListNode(false, null, true,
        [
            new ListItemNode([
                Paragraph(Text("first")),
                new BlockquoteNode([Paragraph(Text("second"))]),
            ]),
            new ListItemNode([
                new BlockquoteNode([Paragraph(Text("third"))]),
                Paragraph(Text("fourth")),
            ]),
        ]);

        Assert.Equal(
            "<ul>\n<li>first\n<blockquote>\n<p>second</p>\n</blockquote>\n</li>\n" +
            "<li>\n<blockquote>\n<p>third</p>\n</blockquote>\nfourth</li>\n</ul>\n" +
            "<li>\n<p>direct</p>\n</li>\n" +
            "<li><input type=\"checkbox\" disabled=\"\" checked=\"\" /> \n<p>task</p>\n</li>\n",
            Renderer.ToHtml(Document(
                tight,
                new ListItemNode([Paragraph(Text("direct"))]),
                new TaskItemNode(true, [Paragraph(Text("task"))]))));
    }

    [Fact]
    public void ToHtml_RendersEveryInlineLeafAndContainer()
    {
        var document = Document(Paragraph(
            Text("<start> "),
            new CodeSpanNode("a < b"),
            Text(" "),
            new LinkNode("https://example.test/a?x=1&y=2", "A \"title\"", [Text("link")]),
            Text(" "),
            new ImageNode("cat & dog.png", null, "cat \"and\" dog"),
            new SoftBreakNode(),
            new AutolinkNode("https://example.test/a b?x=λ", false),
            new HardBreakNode(),
            new AutolinkNode("person@example.test", true)));

        Assert.Equal(
            "<p>&lt;start&gt; <code>a &lt; b</code> " +
            "<a href=\"https://example.test/a?x=1&amp;y=2\" title=\"A &quot;title&quot;\">link</a> " +
            "<img src=\"cat &amp; dog.png\" alt=\"cat &quot;and&quot; dog\" />\n" +
            "<a href=\"https://example.test/a%20b?x=%CE%BB\">https://example.test/a b?x=λ</a><br />\n" +
            "<a href=\"mailto:person@example.test\">person@example.test</a></p>\n",
            Renderer.ToHtml(document));
    }

    [Fact]
    public void ToHtml_RendersExtensionAstNodesWithoutClaimingGfmParsing()
    {
        var table = new TableNode(
            [TableAlignment.Left, TableAlignment.Center, null],
            [
                new TableRowNode(true,
                [
                    new TableCellNode([Text("A")]),
                    new TableCellNode([Text("B")]),
                    new TableCellNode([Text("C")]),
                ]),
                new TableRowNode(false,
                [
                    new TableCellNode([Text("1")]),
                    new TableCellNode([new StrikethroughNode([Text("2")])]),
                    new TableCellNode([Text("3")]),
                ]),
            ]);
        var tasks = new ListNode(false, null, true,
        [
            new TaskItemNode(false, [Paragraph(Text("todo"))]),
            new TaskItemNode(true, [Paragraph(Text("done"))]),
        ]);

        Assert.Equal(
            "<table>\n<thead>\n<tr>\n<th align=\"left\">A</th>\n<th align=\"center\">B</th>\n<th>C</th>\n</tr>\n</thead>\n" +
            "<tbody>\n<tr>\n<td align=\"left\">1</td>\n<td align=\"center\"><del>2</del></td>\n<td>3</td>\n</tr>\n</tbody>\n</table>\n" +
            "<ul>\n<li><input type=\"checkbox\" disabled=\"\" /> todo</li>\n<li><input type=\"checkbox\" disabled=\"\" checked=\"\" /> done</li>\n</ul>\n",
            Renderer.ToHtml(Document(table, tasks)));
    }

    [Fact]
    public void ToHtml_RendersRightAlignedAndEmptyTables()
    {
        var table = new TableNode(
            [TableAlignment.Right],
            [new TableRowNode(false, [new TableCellNode([Text("value")])])]);

        Assert.Equal(
            "<table>\n<tbody>\n<tr>\n<td align=\"right\">value</td>\n</tr>\n</tbody>\n</table>\n<table>\n</table>\n",
            Renderer.ToHtml(Document(table, new TableNode([], []))));
    }

    [Fact]
    public void ToHtml_RoutesRawNodesByFormatAndOption()
    {
        var document = Document(
            new RawBlockNode("html", "<aside>trusted</aside>\n"),
            new RawBlockNode("latex", "\\textbf{ignored}\n"),
            Paragraph(
                new RawInlineNode("html", "<mark>trusted</mark>"),
                new RawInlineNode("latex", "\\emph{ignored}")));

        Assert.Equal(
            "<aside>trusted</aside>\n<p><mark>trusted</mark></p>\n",
            Renderer.ToHtml(document));
        Assert.Equal(
            "<p></p>\n",
            Renderer.ToHtml(document, new RenderOptions(SuppressRawHtml: true)));
    }

    [Theory]
    [InlineData("javascript:alert(1)")]
    [InlineData("JAVASCRIPT:alert(1)")]
    [InlineData("java\rscript:alert(1)")]
    [InlineData("java\u200Bscript:alert(1)")]
    [InlineData(" javascript:alert(1)")]
    [InlineData("vbscript:msgbox(1)")]
    [InlineData("data:text/html,<script>1</script>")]
    [InlineData("blob:https://example.test/id")]
    public void ToHtml_BlocksBrowserExecutableDestinations(string destination)
    {
        var document = Document(Paragraph(new LinkNode(destination, null, [Text("click")])));
        Assert.Equal("<p><a href=\"\">click</a></p>\n", Renderer.ToHtml(document));
    }

    [Theory]
    [InlineData("https://example.test", "https://example.test")]
    [InlineData("irc://example.test/channel", "irc://example.test/channel")]
    [InlineData("/relative path", "/relative path")]
    public void ToHtml_PreservesNonExecutableDestinations(string destination, string expected)
    {
        var document = Document(Paragraph(new LinkNode(destination, null, [Text("go")])));
        Assert.Equal($"<p><a href=\"{expected}\">go</a></p>\n", Renderer.ToHtml(document));
    }

    [Fact]
    public void EscapeHtml_EscapesEveryHtmlMetacharacterAndEnforcesItsOutputLimit()
    {
        Assert.Equal("&amp;&lt;&gt;&quot;'", Renderer.EscapeHtml("&<>\"'"));
        Assert.Throws<ArgumentNullException>(() => Renderer.EscapeHtml(null!));
        Assert.Throws<InvalidOperationException>(() => Renderer.EscapeHtml(new string('x', 1_000_001)));
        Assert.Throws<InvalidOperationException>(() => Renderer.EscapeHtml(new string('&', 200_001)));
    }

    [Fact]
    public void ToHtml_RejectsOversizedAndExpansionHeavyUrls()
    {
        Assert.Throws<InvalidOperationException>(() => Renderer.ToHtml(Document(
            Paragraph(new LinkNode(new string('x', 1_000_001), null, [Text("large")])))));
        Assert.Throws<InvalidOperationException>(() => Renderer.ToHtml(Document(
            Paragraph(new AutolinkNode(new string(' ', 400_000), false)))));
    }

    [Fact]
    public void ToHtml_RejectsUnknownAstVariants()
    {
        Assert.Throws<InvalidOperationException>(() => Renderer.ToHtml(Document(new UnknownBlockNode())));
        Assert.Throws<InvalidOperationException>(() => Renderer.ToHtml(Document(
            new ListNode(false, null, true, [new UnknownListChildNode()]))));
        Assert.Throws<InvalidOperationException>(() => Renderer.ToHtml(Document(
            Paragraph(new UnknownInlineNode()))));
        Assert.Throws<InvalidOperationException>(() => Renderer.ToHtml(Document(
            new TableNode([(TableAlignment)99], [
                new TableRowNode(false, [new TableCellNode([Text("value")])]),
            ]))));
    }

    [Fact]
    public void ToHtml_AcceptsTheDepthLimitAndRejectsTheNextLevel()
    {
        IBlockNode current = Paragraph(Text("bottom"));
        // 254 blockquotes + one paragraph + one text leaf = depth 256.
        for (var index = 0; index < 254; index++)
        {
            current = new BlockquoteNode([current]);
        }

        Assert.Contains("<p>bottom</p>", Renderer.ToHtml(Document(current)));

        current = new BlockquoteNode([current]);
        Assert.Throws<InvalidOperationException>(() => Renderer.ToHtml(Document(current)));
    }

    [Fact]
    public void ToHtml_RejectsOutputPastTheCharacterLimit()
    {
        var document = Document(Paragraph(Text(new string('x', 1_100_000))));
        Assert.Throws<InvalidOperationException>(() => Renderer.ToHtml(document));
    }

    [Fact]
    public void ToHtml_RejectsTreesPastTheNodeLimit()
    {
        var children = Enumerable.Range(0, 100_001)
            .Select(index => (IInlineNode)new TextNode(index.ToString()))
            .ToArray();

        Assert.Throws<InvalidOperationException>(() => Renderer.ToHtml(Document(new ParagraphNode(children))));
    }

    [Fact]
    public void ToHtml_RejectsOversizedTablesBeforeAllocatingRowPartitions()
    {
        var rows = Enumerable.Range(0, 100_001)
            .Select(_ => new TableRowNode(false, []))
            .ToArray();

        Assert.Throws<InvalidOperationException>(() => Renderer.ToHtml(Document(new TableNode([], rows))));
    }

    [Theory]
    [InlineData(0)]
    [InlineData(7)]
    public void ToHtml_RejectsHeadingLevelsOutsideTe00(int level)
    {
        Assert.Throws<InvalidOperationException>(() => Renderer.ToHtml(Document(new HeadingNode(level, [Text("bad")]))));
    }
}
