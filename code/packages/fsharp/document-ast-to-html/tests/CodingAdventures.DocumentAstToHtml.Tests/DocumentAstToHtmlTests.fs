namespace CodingAdventures.DocumentAstToHtml.FSharp.Tests

open System
open System.Globalization
open CodingAdventures.DocumentAst.FSharp
open CodingAdventures.DocumentAstToHtml.FSharp
open Xunit

module DocumentAstToHtmlTests =
    let private document children: DocumentNode = { Children = children }
    let private paragraph children = ParagraphNode children
    let private text value = TextNode value

    [<Fact>]
    let ``render emits core blocks in order`` () =
        let tree =
            document
                [ HeadingNode(2, [ text "A & B" ])
                  paragraph
                      [ text "Hello "
                        EmphasisNode [ text "small" ]
                        text " "
                        StrongNode [ text "world" ]
                        text "." ]
                  ThematicBreakNode
                  CodeBlockNode(Some "fsharp\"<&", "if a < b && c > d then\n")
                  BlockquoteNode [ paragraph [ text "quoted" ] ] ]

        Assert.Equal(
            "<h2>A &amp; B</h2>\n"
            + "<p>Hello <em>small</em> <strong>world</strong>.</p>\n"
            + "<hr />\n"
            + "<pre><code class=\"language-fsharp&quot;&lt;&amp;\">if a &lt; b &amp;&amp; c &gt; d then\n</code></pre>\n"
            + "<blockquote>\n<p>quoted</p>\n</blockquote>\n",
            DocumentAstToHtml.render tree)

    [<Fact>]
    let ``render emits empty code language and invariant heading levels`` () =
        let tree =
            document
                [ HeadingNode(1, [ text "one" ])
                  HeadingNode(6, [ text "six" ])
                  CodeBlockNode(None, "plain")
                  CodeBlockNode(Some "", "also plain") ]

        Assert.Equal(
            "<h1>one</h1>\n<h6>six</h6>\n"
            + "<pre><code>plain</code></pre>\n<pre><code>also plain</code></pre>\n",
            DocumentAstToHtml.render tree)

    [<Fact>]
    let ``render emits tight loose ordered and empty lists`` () =
        let tree =
            document
                [ ListNode(
                      false,
                      None,
                      true,
                      [ ListItemNode [ paragraph [ text "one" ] ]
                        ListItemNode [] ])
                  ListNode(
                      true,
                      Some 42,
                      false,
                      [ ListItemNode [ paragraph [ text "forty-two" ] ] ])
                  ListNode(true, Some 1, true, [ ListItemNode [ paragraph [ text "one again" ] ] ]) ]

        Assert.Equal(
            "<ul>\n<li>one</li>\n<li></li>\n</ul>\n"
            + "<ol start=\"42\">\n<li>\n<p>forty-two</p>\n</li>\n</ol>\n"
            + "<ol>\n<li>one again</li>\n</ol>\n",
            DocumentAstToHtml.render tree)

    [<Fact>]
    let ``render emits multi-block tight and task items`` () =
        let tree =
            document
                [ ListNode(
                      false,
                      None,
                      true,
                      [ ListItemNode
                            [ paragraph [ text "first" ]
                              BlockquoteNode [ paragraph [ text "second" ] ] ]
                        ListItemNode
                            [ BlockquoteNode [ paragraph [ text "third" ] ]
                              paragraph [ text "fourth" ] ]
                        TaskItemNode(false, [ paragraph [ text "todo" ] ])
                        TaskItemNode(true, []) ]) ]

        Assert.Equal(
            "<ul>\n<li>first\n<blockquote>\n<p>second</p>\n</blockquote>\n</li>\n"
            + "<li>\n<blockquote>\n<p>third</p>\n</blockquote>\nfourth</li>\n"
            + "<li><input type=\"checkbox\" disabled=\"\" /> todo</li>\n"
            + "<li><input type=\"checkbox\" disabled=\"\" checked=\"\" /> </li>\n</ul>\n",
            DocumentAstToHtml.render tree)

    [<Fact>]
    let ``render emits every inline leaf and container`` () =
        let tree =
            document
                [ paragraph
                      [ text "<start> "
                        CodeSpanNode "a < b"
                        text " "
                        LinkNode("https://example.test/a?x=1&y=2", Some "A \"title\"", [ text "link" ])
                        text " "
                        ImageNode("cat & dog.png", None, "cat \"and\" dog")
                        SoftBreakNode
                        AutolinkNode("https://example.test/a b?x=λ", false)
                        HardBreakNode
                        AutolinkNode("person@example.test", true) ] ]

        Assert.Equal(
            "<p>&lt;start&gt; <code>a &lt; b</code> "
            + "<a href=\"https://example.test/a?x=1&amp;y=2\" title=\"A &quot;title&quot;\">link</a> "
            + "<img src=\"cat &amp; dog.png\" alt=\"cat &quot;and&quot; dog\" />\n"
            + "<a href=\"https://example.test/a%20b?x=%CE%BB\">https://example.test/a b?x=λ</a><br />\n"
            + "<a href=\"mailto:person@example.test\">person@example.test</a></p>\n",
            DocumentAstToHtml.render tree)

    [<Fact>]
    let ``render emits tables strikethrough and every alignment`` () =
        let cells values =
            values
            |> List.map (fun value -> ({ Children = value } : TableCellNode))

        let table =
            TableNode(
                [ Some Left; Some Center; Some Right; None ],
                [ ({ IsHeader = true
                     Children = cells [ [ text "A" ]; [ text "B" ]; [ text "C" ]; [ text "D" ] ] }
                   : TableRowNode)
                  ({ IsHeader = false
                     Children =
                        cells
                            [ [ text "1" ]
                              [ StrikethroughNode [ text "2" ] ]
                              [ text "3" ]
                              [ text "4" ] ] }
                   : TableRowNode) ])

        Assert.Equal(
            "<table>\n<thead>\n<tr>\n<th align=\"left\">A</th>\n<th align=\"center\">B</th>\n"
            + "<th align=\"right\">C</th>\n<th>D</th>\n</tr>\n</thead>\n<tbody>\n<tr>\n"
            + "<td align=\"left\">1</td>\n<td align=\"center\"><del>2</del></td>\n"
            + "<td align=\"right\">3</td>\n<td>4</td>\n</tr>\n</tbody>\n</table>\n",
            DocumentAstToHtml.render (document [ table ]))

    [<Fact>]
    let ``render emits empty header-only and body-only tables`` () =
        let cell value = ({ Children = [ text value ] } : TableCellNode)
        let row header value = ({ IsHeader = header; Children = [ cell value ] } : TableRowNode)

        let tree =
            document
                [ TableNode([], [])
                  TableNode([], [ row true "head" ])
                  TableNode([], [ row false "body" ]) ]

        Assert.Equal(
            "<table>\n</table>\n"
            + "<table>\n<thead>\n<tr>\n<th>head</th>\n</tr>\n</thead>\n</table>\n"
            + "<table>\n<tbody>\n<tr>\n<td>body</td>\n</tr>\n</tbody>\n</table>\n",
            DocumentAstToHtml.render tree)

    [<Fact>]
    let ``render routes raw nodes by format and suppression option`` () =
        let tree =
            document
                [ RawBlockNode("html", "<aside>trusted</aside>\n")
                  RawBlockNode("latex", "\\textbf{ignored}\n")
                  paragraph
                      [ RawInlineNode("html", "<mark>trusted</mark>")
                        RawInlineNode("latex", "\\emph{ignored}") ] ]

        Assert.Equal("<aside>trusted</aside>\n<p><mark>trusted</mark></p>\n", DocumentAstToHtml.render tree)

        Assert.Equal(
            "<p></p>\n",
            DocumentAstToHtml.renderWithOptions { SuppressRawHtml = true } tree)

    [<Fact>]
    let ``render blocks browser-executable and obfuscated destinations`` () =
        let destinations =
            [ "javascript:alert(1)"
              "JAVASCRIPT:alert(1)"
              "java\rscript:alert(1)"
              "java​script:alert(1)"
              " javascript:alert(1)"
              "vbscript:msgbox(1)"
              "data:text/html,<script>1</script>"
              "blob:https://example.test/id" ]

        for destination in destinations do
            let tree = document [ paragraph [ LinkNode(destination, None, [ text "click" ]) ] ]
            Assert.Equal("<p><a href=\"\">click</a></p>\n", DocumentAstToHtml.render tree)

        let image = document [ paragraph [ ImageNode("data:image/svg+xml,bad", None, "alt") ] ]
        let autolink = document [ paragraph [ AutolinkNode("java\tscript:alert(1)", false) ] ]
        Assert.Equal("<p><img src=\"\" alt=\"alt\" /></p>\n", DocumentAstToHtml.render image)

        Assert.Equal(
            "<p><a href=\"\">java\tscript:alert(1)</a></p>\n",
            DocumentAstToHtml.render autolink)

    [<Fact>]
    let ``render preserves safe absolute and relative destinations`` () =
        let vectors =
            [ "https://example.test", "https://example.test"
              "irc://example.test/channel", "irc://example.test/channel"
              "/relative path", "/relative path" ]

        for destination, expected in vectors do
            let tree = document [ paragraph [ LinkNode(destination, None, [ text "go" ]) ] ]
            Assert.Equal(sprintf "<p><a href=\"%s\">go</a></p>\n" expected, DocumentAstToHtml.render tree)

    [<Fact>]
    let ``escapeHtml escapes metacharacters and enforces its bound`` () =
        Assert.Equal("&amp;&lt;&gt;&quot;'", DocumentAstToHtml.escapeHtml "&<>\"'")
        Assert.Equal("line\n", DocumentAstToHtml.escapeHtml "line\n")
        Assert.Throws<ArgumentNullException>(fun () -> DocumentAstToHtml.escapeHtml null |> ignore)
        |> ignore

        Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.escapeHtml (String('x', 1_000_001)) |> ignore)
        |> ignore

        Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.escapeHtml (String('&', 200_001)) |> ignore)
        |> ignore

    [<Fact>]
    let ``render rejects oversized and expansion-heavy URLs`` () =
        let oversized = document [ paragraph [ LinkNode(String('x', 1_000_001), None, [ text "large" ]) ] ]
        let expanding = document [ paragraph [ AutolinkNode(String(' ', 400_000), false) ] ]

        Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.render oversized |> ignore)
        |> ignore

        Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.render expanding |> ignore)
        |> ignore

    [<Fact>]
    let ``render bounds URL normalization across repeated aliases`` () =
        for image in [ false; true ] do
            for blocked in [ false; true ] do
                let destination =
                    if blocked then
                        "javascript:" + String('x', 9_989)
                    else
                        String('\000', 10_000)

                let alias =
                    if image then
                        ImageNode(destination, None, "")
                    else
                        LinkNode(destination, None, [])

                let tree = document [ paragraph (List.replicate 101 alias) ]
                let error =
                    Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.render tree |> ignore)

                Assert.Contains("URL normalization", error.Message)

    [<Fact>]
    let ``render URL normalization budget is inclusive and resets per call`` () =
        let alias = LinkNode(String('\000', 10_000), None, [])
        let accepted = document [ paragraph (List.replicate 100 alias) ]
        let expected = "<p>" + String.concat "" (List.replicate 100 "<a href=\"\"></a>") + "</p>\n"

        Assert.Equal(expected, DocumentAstToHtml.render accepted)
        Assert.Equal(expected, DocumentAstToHtml.render accepted)

        let rejected = document [ paragraph (List.replicate 100 alias @ [ LinkNode("\000", None, []) ]) ]
        let error =
            Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.render rejected |> ignore)

        Assert.Contains("URL normalization", error.Message)
        Assert.Equal(expected, DocumentAstToHtml.render accepted)

    [<Fact>]
    let ``render shares one UTF-16 URL budget across every URL node`` () =
        let astral = "\U0001F600"
        let nodes =
            [ LinkNode(String('\000', 499_999), None, [])
              ImageNode(String('\000', 499_999), None, "")
              AutolinkNode(astral, false) ]

        Assert.Equal(
            "<p><a href=\"\"></a><img src=\"\" alt=\"\" /><a href=\"%F0%9F%98%80\">"
            + astral
            + "</a></p>\n",
            DocumentAstToHtml.render (document [ paragraph nodes ]))

        let rejected = document [ paragraph (nodes @ [ ImageNode("\000", None, "") ]) ]
        let error =
            Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.render rejected |> ignore)

        Assert.Contains("URL normalization", error.Message)

    [<Fact>]
    let ``render accepts depth 256 and rejects depth 257`` () =
        let rec wrap count node =
            if count = 0 then node else wrap (count - 1) (BlockquoteNode [ node ])

        let accepted = document [ wrap 254 (paragraph [ text "bottom" ]) ]
        let rejected = document [ wrap 255 (paragraph [ text "bottom" ]) ]

        Assert.Contains("<p>bottom</p>", DocumentAstToHtml.render accepted)
        Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.render rejected |> ignore)
        |> ignore

    [<Fact>]
    let ``render rejects output and trees beyond resource limits`` () =
        let oversizedOutput = document [ paragraph [ text (String('x', 1_100_000)) ] ]
        let oversizedTree = document [ paragraph [ for index in 0..100_000 -> text (string index) ] ]
        let oversizedTable =
            document
                [ TableNode(
                      [],
                      [ for _ in 0..100_000 ->
                            ({ IsHeader = false; Children = [] } : TableRowNode) ]) ]
        let oversizedAlignment = document [ TableNode(List.replicate 100_001 None, []) ]

        Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.render oversizedOutput |> ignore)
        |> ignore

        Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.render oversizedTree |> ignore)
        |> ignore

        Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.render oversizedTable |> ignore)
        |> ignore

        Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.render oversizedAlignment |> ignore)
        |> ignore

    [<Fact>]
    let ``render rejects invalid heading levels and null documents`` () =
        for level in [ 0; 7 ] do
            let tree = document [ HeadingNode(level, [ text "bad" ]) ]
            Assert.Throws<InvalidOperationException>(fun () -> DocumentAstToHtml.render tree |> ignore)
            |> ignore

        Assert.Throws<ArgumentNullException>(fun () -> DocumentAstToHtml.render Unchecked.defaultof<DocumentNode> |> ignore)
        |> ignore

        Assert.Throws<ArgumentNullException>(fun () ->
            DocumentAstToHtml.renderWithOptions Unchecked.defaultof<RenderOptions> (document [])
            |> ignore)
        |> ignore

    [<Fact>]
    let ``render is culture invariant`` () =
        let originalCulture = CultureInfo.CurrentCulture

        try
            CultureInfo.CurrentCulture <- CultureInfo("ar-SA")

            let tree =
                document
                    [ HeadingNode(2, [ text "heading" ])
                      ListNode(true, Some 42, true, [ ListItemNode [ paragraph [ text "item" ] ] ]) ]

            Assert.Equal(
                "<h2>heading</h2>\n<ol start=\"42\">\n<li>item</li>\n</ol>\n",
                DocumentAstToHtml.render tree)
        finally
            CultureInfo.CurrentCulture <- originalCulture

    [<Fact>]
    let ``render is deterministic and leaves its immutable input unchanged`` () =
        let tree =
            document
                [ paragraph
                      [ text "same"
                        LinkNode("/path", Some "title", [ StrongNode [ text "link" ] ]) ] ]

        let snapshot = tree
        let first = DocumentAstToHtml.render tree
        let second = DocumentAstToHtml.render tree

        Assert.Equal("<p>same<a href=\"/path\" title=\"title\"><strong>link</strong></a></p>\n", first)
        Assert.Equal(first, second)
        Assert.Equal(snapshot, tree)
