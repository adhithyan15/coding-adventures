namespace CodingAdventures.DocumentAst.FSharp.Tests

open CodingAdventures.DocumentAst.FSharp
open Xunit

module DocumentAstTests =
    [<Fact>]
    let ``core nodes expose stable type names`` () =
        let doc: DocumentNode =
            { Children =
                [ HeadingNode (2, [ TextNode "Hello" ])
                  ParagraphNode [ TextNode "World" ] ] }

        Assert.Equal("heading", BlockNode.typeName doc.Children[0])
        Assert.Equal("paragraph", BlockNode.typeName doc.Children[1])

    [<Fact>]
    let ``list nodes retain ordered start and task shape`` () =
        let listNode =
            ListNode(
                true,
                Some 3,
                false,
                [ ListItemNode [ ParagraphNode [ TextNode "alpha" ] ]
                  TaskItemNode(true, [ ParagraphNode [ TextNode "beta" ] ]) ])

        match listNode with
        | ListNode (ordered, start, tight, children) ->
            Assert.True(ordered)
            Assert.Equal(Some 3, start)
            Assert.False(tight)
            Assert.Equal("list_item", ListChildNode.typeName children[0])
            Assert.Equal("task_item", ListChildNode.typeName children[1])
        | _ -> Assert.Fail("Expected list node")

    [<Fact>]
    let ``inline nodes model links images and breaks`` () =
        let link = LinkNode("https://example.com", Some "Example", [ TextNode "Example" ])
        let image = ImageNode("cat.png", None, "Cat")

        Assert.Equal("link", InlineNode.typeName link)
        Assert.Equal("image", InlineNode.typeName image)
        Assert.Equal("hard_break", InlineNode.typeName HardBreakNode)
        Assert.Equal("soft_break", InlineNode.typeName SoftBreakNode)

    [<Fact>]
    let ``gfm extensions model tables and strikethrough`` () =
        let table =
            TableNode(
                [ Some Left; None; Some Right ],
                [ ({ IsHeader = true; Children = [ ({ Children = [ TextNode "A" ] } : TableCellNode) ] } : TableRowNode)
                  ({ IsHeader = false; Children = [ ({ Children = [ TextNode "B" ] } : TableCellNode) ] } : TableRowNode) ])

        let strike = StrikethroughNode [ TextNode "gone" ]

        Assert.Equal("table", BlockNode.typeName table)
        Assert.Equal("table_row", TableNode.rowTypeName ({ IsHeader = true; Children = [] } : TableRowNode))
        Assert.Equal("table_cell", TableNode.cellTypeName ({ Children = [] } : TableCellNode))
        Assert.Equal("strikethrough", InlineNode.typeName strike)
