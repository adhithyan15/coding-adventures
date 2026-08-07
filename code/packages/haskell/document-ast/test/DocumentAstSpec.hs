module DocumentAstSpec (spec) where

import DocumentAst
import Test.Hspec

text :: String -> InlineNode
text = InlineText . TextNode

paragraph :: String -> BlockNode
paragraph = BlockParagraph . ParagraphNode . pure . text

spec :: Spec
spec = do
    describe "document structure" $ do
        it "builds an immutable nested document" $ do
            let document =
                    DocumentNode
                        [ BlockHeading (HeadingNode 1 [text "Title"])
                        , paragraph "Hello"
                        , BlockBlockquote (BlockquoteNode [paragraph "Quoted"])
                        ]
            length (documentChildren document) `shouldBe` 3
            blockNodeTypeName (documentChildren document !! 0) `shouldBe` "heading"
            blockNodeTypeName (documentChildren document !! 2) `shouldBe` "blockquote"

        it "exposes every stable block discriminator" $ do
            let row = TableRowNode True []
                cell = TableCellNode []
                nodes =
                    [ BlockDocument (DocumentNode [])
                    , BlockHeading (HeadingNode 2 [])
                    , BlockParagraph (ParagraphNode [])
                    , BlockCodeBlock (CodeBlockNode Nothing "code\n")
                    , BlockBlockquote (BlockquoteNode [])
                    , BlockList (ListNode False Nothing True [])
                    , BlockListItem (ListItemNode [])
                    , BlockTaskItem (TaskItemNode True [])
                    , BlockThematicBreak ThematicBreakNode
                    , BlockRawBlock (RawBlockNode "html" "<hr>\n")
                    , BlockTable (TableNode [] [])
                    , BlockTableRow row
                    , BlockTableCell cell
                    ]
            map blockNodeTypeName nodes
                `shouldBe` [ "document"
                           , "heading"
                           , "paragraph"
                           , "code_block"
                           , "blockquote"
                           , "list"
                           , "list_item"
                           , "task_item"
                           , "thematic_break"
                           , "raw_block"
                           , "table"
                           , "table_row"
                           , "table_cell"
                           ]

        it "exposes concrete block payloads without mutation" $ do
            let heading = HeadingNode 6 [text "Deep"]
                para = ParagraphNode [text "Body"]
                quote = BlockquoteNode [BlockParagraph para]
                item = ListItemNode [BlockParagraph para]
                task = TaskItemNode False [BlockParagraph para]
            (headingLevel heading, headingChildren heading) `shouldBe` (6, [text "Deep"])
            paragraphChildren para `shouldBe` [text "Body"]
            blockquoteChildren quote `shouldBe` [BlockParagraph para]
            listItemChildren item `shouldBe` [BlockParagraph para]
            (taskItemChecked task, taskItemChildren task)
                `shouldBe` (False, [BlockParagraph para])

    describe "lists and tables" $ do
        it "preserves ordered-list metadata and task state" $ do
            let regular = ListItem (ListItemNode [paragraph "alpha"])
                task = TaskItem (TaskItemNode True [paragraph "beta"])
                node = ListNode True (Just 3) False [regular, task]
            (listOrdered node, listStart node, listTight node) `shouldBe` (True, Just 3, False)
            map listChildNodeTypeName (listChildren node) `shouldBe` ["list_item", "task_item"]
            taskItemChecked (TaskItemNode True []) `shouldBe` True

        it "preserves optional GFM table alignment and cell content" $ do
            let cell = TableCellNode [InlineStrong (StrongNode [text "Name"])]
                header = TableRowNode True [cell]
                table = TableNode [Just AlignLeft, Nothing, Just AlignRight, Just AlignCenter] [header]
            tableAlignments table `shouldBe` [Just AlignLeft, Nothing, Just AlignRight, Just AlignCenter]
            tableRowIsHeader (head (tableRows table)) `shouldBe` True
            tableRowChildren header `shouldBe` [cell]
            tableCellChildren cell `shouldBe` [InlineStrong (StrongNode [text "Name"])]
            tableRowNodeTypeName header `shouldBe` "table_row"
            tableCellNodeTypeName cell `shouldBe` "table_cell"

    describe "inline content" $ do
        it "exposes every stable inline discriminator" $ do
            let nodes =
                    [ text "text"
                    , InlineEmphasis (EmphasisNode [])
                    , InlineStrong (StrongNode [])
                    , InlineStrikethrough (StrikethroughNode [])
                    , InlineCodeSpan (CodeSpanNode "code")
                    , InlineLink (LinkNode "/" Nothing [])
                    , InlineImage (ImageNode "cat.png" Nothing "cat")
                    , InlineAutolink (AutolinkNode "user@example.com" True)
                    , InlineRawInline (RawInlineNode "html" "<em>x</em>")
                    , InlineHardBreak HardBreakNode
                    , InlineSoftBreak SoftBreakNode
                    ]
            map inlineNodeTypeName nodes
                `shouldBe` [ "text"
                           , "emphasis"
                           , "strong"
                           , "strikethrough"
                           , "code_span"
                           , "link"
                           , "image"
                           , "autolink"
                           , "raw_inline"
                           , "hard_break"
                           , "soft_break"
                           ]

        it "retains resolved link, image, and autolink data" $ do
            let link = LinkNode "https://example.com" (Just "Example") [text "click"]
                image = ImageNode "cat.png" Nothing "a cat"
                email = AutolinkNode "user@example.com" True
            (linkDestination link, linkTitle link, linkChildren link)
                `shouldBe` ("https://example.com", Just "Example", [text "click"])
            (imageDestination image, imageTitle image, imageAlt image)
                `shouldBe` ("cat.png", Nothing, "a cat")
            (autolinkDestination email, autolinkIsEmail email)
                `shouldBe` ("user@example.com", True)

        it "exposes text and formatting children" $ do
            let plain = TextNode "value"
                emphasis = EmphasisNode [InlineText plain]
                strong = StrongNode [InlineEmphasis emphasis]
                strike = StrikethroughNode [InlineStrong strong]
            textValue plain `shouldBe` "value"
            emphasisChildren emphasis `shouldBe` [InlineText plain]
            strongChildren strong `shouldBe` [InlineEmphasis emphasis]
            strikethroughChildren strike `shouldBe` [InlineStrong strong]

        it "retains raw and code payloads without interpretation" $ do
            let codeBlock = CodeBlockNode (Just "haskell") "main = pure ()\n"
                rawBlock = RawBlockNode "latex" "\\textbf{x}\n"
                rawInline = RawInlineNode "html" "<em>x</em>"
            (codeBlockLanguage codeBlock, codeBlockValue codeBlock)
                `shouldBe` (Just "haskell", "main = pure ()\n")
            (rawBlockFormat rawBlock, rawBlockValue rawBlock)
                `shouldBe` ("latex", "\\textbf{x}\n")
            (rawInlineFormat rawInline, rawInlineValue rawInline)
                `shouldBe` ("html", "<em>x</em>")
            codeSpanValue (CodeSpanNode "&amp;") `shouldBe` "&amp;"

    describe "generic traversal" $ do
        it "dispatches both Node union branches" $ do
            nodeTypeName (NodeBlock (paragraph "block")) `shouldBe` "paragraph"
            nodeTypeName (NodeInline (text "inline")) `shouldBe` "text"

        it "supports structural equality for recursively nested values" $ do
            let value =
                    BlockList
                        ( ListNode
                            False
                            Nothing
                            True
                            [ListItem (ListItemNode [paragraph "item"])]
                        )
            value `shouldBe` value
            show value `shouldContain` "ListNode"
