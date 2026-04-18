namespace CodingAdventures.DocumentAst.FSharp

type TableAlignment =
    | Left
    | Right
    | Center

type DocumentNode = { Children: BlockNode list }

and BlockNode =
    | HeadingNode of level: int * children: InlineNode list
    | ParagraphNode of children: InlineNode list
    | CodeBlockNode of language: string option * value: string
    | BlockquoteNode of children: BlockNode list
    | ListNode of ordered: bool * start: int option * tight: bool * children: ListChildNode list
    | ThematicBreakNode
    | RawBlockNode of format: string * value: string
    | TableNode of align: TableAlignment option list * children: TableRowNode list

and ListChildNode =
    | ListItemNode of children: BlockNode list
    | TaskItemNode of checked': bool * children: BlockNode list

and TableRowNode = { IsHeader: bool; Children: TableCellNode list }

and TableCellNode = { Children: InlineNode list }

and InlineNode =
    | TextNode of value: string
    | EmphasisNode of children: InlineNode list
    | StrongNode of children: InlineNode list
    | StrikethroughNode of children: InlineNode list
    | CodeSpanNode of value: string
    | LinkNode of destination: string * title: string option * children: InlineNode list
    | ImageNode of destination: string * title: string option * alt: string
    | AutolinkNode of destination: string * isEmail: bool
    | RawInlineNode of format: string * value: string
    | HardBreakNode
    | SoftBreakNode

[<RequireQualifiedAccess>]
module BlockNode =
    let typeName =
        function
        | HeadingNode _ -> "heading"
        | ParagraphNode _ -> "paragraph"
        | CodeBlockNode _ -> "code_block"
        | BlockquoteNode _ -> "blockquote"
        | ListNode _ -> "list"
        | ThematicBreakNode -> "thematic_break"
        | RawBlockNode _ -> "raw_block"
        | TableNode _ -> "table"

[<RequireQualifiedAccess>]
module ListChildNode =
    let typeName =
        function
        | ListItemNode _ -> "list_item"
        | TaskItemNode _ -> "task_item"

[<RequireQualifiedAccess>]
module InlineNode =
    let typeName =
        function
        | TextNode _ -> "text"
        | EmphasisNode _ -> "emphasis"
        | StrongNode _ -> "strong"
        | StrikethroughNode _ -> "strikethrough"
        | CodeSpanNode _ -> "code_span"
        | LinkNode _ -> "link"
        | ImageNode _ -> "image"
        | AutolinkNode _ -> "autolink"
        | RawInlineNode _ -> "raw_inline"
        | HardBreakNode -> "hard_break"
        | SoftBreakNode -> "soft_break"

[<RequireQualifiedAccess>]
module TableNode =
    let rowTypeName (_: TableRowNode) = "table_row"
    let cellTypeName (_: TableCellNode) = "table_cell"
