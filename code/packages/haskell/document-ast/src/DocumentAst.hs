-- | A format-agnostic intermediate representation for structured documents.
--
-- Front-end parsers produce these immutable values and back-end renderers
-- consume them. The types implement TE00 Document AST together with the
-- shared GFM task-list, strikethrough, and table extensions.
module DocumentAst
    ( TableAlignment (..)
    , DocumentNode (..)
    , HeadingNode (..)
    , ParagraphNode (..)
    , CodeBlockNode (..)
    , BlockquoteNode (..)
    , ListNode (..)
    , ListItemNode (..)
    , TaskItemNode (..)
    , ThematicBreakNode (..)
    , RawBlockNode (..)
    , TableNode (..)
    , TableRowNode (..)
    , TableCellNode (..)
    , TextNode (..)
    , EmphasisNode (..)
    , StrongNode (..)
    , StrikethroughNode (..)
    , CodeSpanNode (..)
    , LinkNode (..)
    , ImageNode (..)
    , AutolinkNode (..)
    , RawInlineNode (..)
    , HardBreakNode (..)
    , SoftBreakNode (..)
    , ListChildNode (..)
    , BlockNode (..)
    , InlineNode (..)
    , Node (..)
    , blockNodeTypeName
    , listChildNodeTypeName
    , tableRowNodeTypeName
    , tableCellNodeTypeName
    , inlineNodeTypeName
    , nodeTypeName
    ) where

-- | Column alignment hint for a table. 'Nothing' represents no hint.
data TableAlignment
    = AlignLeft
    | AlignRight
    | AlignCenter
    deriving (Eq, Show)

-- | Root of a document. A well-formed tree does not nest it below another node.
newtype DocumentNode = DocumentNode
    { documentChildren :: [BlockNode]
    }
    deriving (Eq, Show)

-- | Section heading. Front-ends clamp 'headingLevel' to the range 1 through 6.
data HeadingNode = HeadingNode
    { headingLevel :: Int
    , headingChildren :: [InlineNode]
    }
    deriving (Eq, Show)

-- | Block of prose containing inline content.
newtype ParagraphNode = ParagraphNode
    { paragraphChildren :: [InlineNode]
    }
    deriving (Eq, Show)

-- | Literal code or preformatted text, with an optional language hint.
data CodeBlockNode = CodeBlockNode
    { codeBlockLanguage :: Maybe String
    , codeBlockValue :: String
    }
    deriving (Eq, Show)

-- | Quoted block content.
newtype BlockquoteNode = BlockquoteNode
    { blockquoteChildren :: [BlockNode]
    }
    deriving (Eq, Show)

-- | Ordered or unordered list with its source tightness preserved.
data ListNode = ListNode
    { listOrdered :: Bool
    , listStart :: Maybe Int
    , listTight :: Bool
    , listChildren :: [ListChildNode]
    }
    deriving (Eq, Show)

-- | Ordinary list item containing block content.
newtype ListItemNode = ListItemNode
    { listItemChildren :: [BlockNode]
    }
    deriving (Eq, Show)

-- | GFM task-list item containing block content.
data TaskItemNode = TaskItemNode
    { taskItemChecked :: Bool
    , taskItemChildren :: [BlockNode]
    }
    deriving (Eq, Show)

-- | Visual separator between sections.
data ThematicBreakNode = ThematicBreakNode
    deriving (Eq, Show)

-- | Verbatim block content for one target format.
data RawBlockNode = RawBlockNode
    { rawBlockFormat :: String
    , rawBlockValue :: String
    }
    deriving (Eq, Show)

-- | GFM table with one alignment entry per column.
data TableNode = TableNode
    { tableAlignments :: [Maybe TableAlignment]
    , tableRows :: [TableRowNode]
    }
    deriving (Eq, Show)

-- | One header or body row in a table.
data TableRowNode = TableRowNode
    { tableRowIsHeader :: Bool
    , tableRowChildren :: [TableCellNode]
    }
    deriving (Eq, Show)

-- | One table cell containing inline content.
newtype TableCellNode = TableCellNode
    { tableCellChildren :: [InlineNode]
    }
    deriving (Eq, Show)

-- | Plain decoded Unicode text.
newtype TextNode = TextNode
    { textValue :: String
    }
    deriving (Eq, Show)

-- | Stressed emphasis.
newtype EmphasisNode = EmphasisNode
    { emphasisChildren :: [InlineNode]
    }
    deriving (Eq, Show)

-- | Strong importance.
newtype StrongNode = StrongNode
    { strongChildren :: [InlineNode]
    }
    deriving (Eq, Show)

-- | GFM deleted or struck-through text.
newtype StrikethroughNode = StrikethroughNode
    { strikethroughChildren :: [InlineNode]
    }
    deriving (Eq, Show)

-- | Raw inline code.
newtype CodeSpanNode = CodeSpanNode
    { codeSpanValue :: String
    }
    deriving (Eq, Show)

-- | Hyperlink with a resolved destination and optional title.
data LinkNode = LinkNode
    { linkDestination :: String
    , linkTitle :: Maybe String
    , linkChildren :: [InlineNode]
    }
    deriving (Eq, Show)

-- | Embedded image with plain-text alternative content.
data ImageNode = ImageNode
    { imageDestination :: String
    , imageTitle :: Maybe String
    , imageAlt :: String
    }
    deriving (Eq, Show)

-- | Direct URL or email link.
data AutolinkNode = AutolinkNode
    { autolinkDestination :: String
    , autolinkIsEmail :: Bool
    }
    deriving (Eq, Show)

-- | Verbatim inline content for one target format.
data RawInlineNode = RawInlineNode
    { rawInlineFormat :: String
    , rawInlineValue :: String
    }
    deriving (Eq, Show)

-- | Forced line break.
data HardBreakNode = HardBreakNode
    deriving (Eq, Show)

-- | Source-preserving soft line break.
data SoftBreakNode = SoftBreakNode
    deriving (Eq, Show)

-- | The only node shapes permitted directly inside a list.
data ListChildNode
    = ListItem ListItemNode
    | TaskItem TaskItemNode
    deriving (Eq, Show)

-- | Structural document nodes. Document, list-item, and table component
-- variants are included to support exhaustive generic traversal.
data BlockNode
    = BlockDocument DocumentNode
    | BlockHeading HeadingNode
    | BlockParagraph ParagraphNode
    | BlockCodeBlock CodeBlockNode
    | BlockBlockquote BlockquoteNode
    | BlockList ListNode
    | BlockListItem ListItemNode
    | BlockTaskItem TaskItemNode
    | BlockThematicBreak ThematicBreakNode
    | BlockRawBlock RawBlockNode
    | BlockTable TableNode
    | BlockTableRow TableRowNode
    | BlockTableCell TableCellNode
    deriving (Eq, Show)

-- | Inline document nodes.
data InlineNode
    = InlineText TextNode
    | InlineEmphasis EmphasisNode
    | InlineStrong StrongNode
    | InlineStrikethrough StrikethroughNode
    | InlineCodeSpan CodeSpanNode
    | InlineLink LinkNode
    | InlineImage ImageNode
    | InlineAutolink AutolinkNode
    | InlineRawInline RawInlineNode
    | InlineHardBreak HardBreakNode
    | InlineSoftBreak SoftBreakNode
    deriving (Eq, Show)

-- | Union of block and inline nodes for generic traversal.
data Node
    = NodeBlock BlockNode
    | NodeInline InlineNode
    deriving (Eq, Show)

-- | Stable discriminator used by serializers and renderers.
blockNodeTypeName :: BlockNode -> String
blockNodeTypeName (BlockDocument _) = "document"
blockNodeTypeName (BlockHeading _) = "heading"
blockNodeTypeName (BlockParagraph _) = "paragraph"
blockNodeTypeName (BlockCodeBlock _) = "code_block"
blockNodeTypeName (BlockBlockquote _) = "blockquote"
blockNodeTypeName (BlockList _) = "list"
blockNodeTypeName (BlockListItem _) = "list_item"
blockNodeTypeName (BlockTaskItem _) = "task_item"
blockNodeTypeName (BlockThematicBreak _) = "thematic_break"
blockNodeTypeName (BlockRawBlock _) = "raw_block"
blockNodeTypeName (BlockTable _) = "table"
blockNodeTypeName (BlockTableRow _) = "table_row"
blockNodeTypeName (BlockTableCell _) = "table_cell"

-- | Stable discriminator for direct list children.
listChildNodeTypeName :: ListChildNode -> String
listChildNodeTypeName (ListItem _) = "list_item"
listChildNodeTypeName (TaskItem _) = "task_item"

-- | Stable discriminator for table rows.
tableRowNodeTypeName :: TableRowNode -> String
tableRowNodeTypeName _ = "table_row"

-- | Stable discriminator for table cells.
tableCellNodeTypeName :: TableCellNode -> String
tableCellNodeTypeName _ = "table_cell"

-- | Stable discriminator used by serializers and renderers.
inlineNodeTypeName :: InlineNode -> String
inlineNodeTypeName (InlineText _) = "text"
inlineNodeTypeName (InlineEmphasis _) = "emphasis"
inlineNodeTypeName (InlineStrong _) = "strong"
inlineNodeTypeName (InlineStrikethrough _) = "strikethrough"
inlineNodeTypeName (InlineCodeSpan _) = "code_span"
inlineNodeTypeName (InlineLink _) = "link"
inlineNodeTypeName (InlineImage _) = "image"
inlineNodeTypeName (InlineAutolink _) = "autolink"
inlineNodeTypeName (InlineRawInline _) = "raw_inline"
inlineNodeTypeName (InlineHardBreak _) = "hard_break"
inlineNodeTypeName (InlineSoftBreak _) = "soft_break"

-- | Stable discriminator for either union branch.
nodeTypeName :: Node -> String
nodeTypeName (NodeBlock node) = blockNodeTypeName node
nodeTypeName (NodeInline node) = inlineNodeTypeName node
