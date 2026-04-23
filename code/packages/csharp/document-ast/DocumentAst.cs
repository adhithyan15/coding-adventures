namespace CodingAdventures.DocumentAst;

public interface INode
{
    string Type { get; }
}

public interface IBlockNode : INode
{
}

public interface IListChildNode : IBlockNode
{
}

public interface IInlineNode : INode
{
}

public enum TableAlignment
{
    Left,
    Right,
    Center,
}

public sealed record DocumentNode(IReadOnlyList<IBlockNode> Children) : INode
{
    public string Type => "document";
}

public sealed record HeadingNode(int Level, IReadOnlyList<IInlineNode> Children) : IBlockNode
{
    public string Type => "heading";
}

public sealed record ParagraphNode(IReadOnlyList<IInlineNode> Children) : IBlockNode
{
    public string Type => "paragraph";
}

public sealed record CodeBlockNode(string? Language, string Value) : IBlockNode
{
    public string Type => "code_block";
}

public sealed record BlockquoteNode(IReadOnlyList<IBlockNode> Children) : IBlockNode
{
    public string Type => "blockquote";
}

public sealed record ListNode(bool Ordered, int? Start, bool Tight, IReadOnlyList<IListChildNode> Children) : IBlockNode
{
    public string Type => "list";
}

public sealed record ListItemNode(IReadOnlyList<IBlockNode> Children) : IListChildNode
{
    public string Type => "list_item";
}

public sealed record TaskItemNode(bool Checked, IReadOnlyList<IBlockNode> Children) : IListChildNode
{
    public string Type => "task_item";
}

public sealed record ThematicBreakNode() : IBlockNode
{
    public string Type => "thematic_break";
}

public sealed record RawBlockNode(string Format, string Value) : IBlockNode
{
    public string Type => "raw_block";
}

public sealed record TableNode(IReadOnlyList<TableAlignment?> Align, IReadOnlyList<TableRowNode> Children) : IBlockNode
{
    public string Type => "table";
}

public sealed record TableRowNode(bool IsHeader, IReadOnlyList<TableCellNode> Children) : INode
{
    public string Type => "table_row";
}

public sealed record TableCellNode(IReadOnlyList<IInlineNode> Children) : INode
{
    public string Type => "table_cell";
}

public sealed record TextNode(string Value) : IInlineNode
{
    public string Type => "text";
}

public sealed record EmphasisNode(IReadOnlyList<IInlineNode> Children) : IInlineNode
{
    public string Type => "emphasis";
}

public sealed record StrongNode(IReadOnlyList<IInlineNode> Children) : IInlineNode
{
    public string Type => "strong";
}

public sealed record StrikethroughNode(IReadOnlyList<IInlineNode> Children) : IInlineNode
{
    public string Type => "strikethrough";
}

public sealed record CodeSpanNode(string Value) : IInlineNode
{
    public string Type => "code_span";
}

public sealed record LinkNode(string Destination, string? Title, IReadOnlyList<IInlineNode> Children) : IInlineNode
{
    public string Type => "link";
}

public sealed record ImageNode(string Destination, string? Title, string Alt) : IInlineNode
{
    public string Type => "image";
}

public sealed record AutolinkNode(string Destination, bool IsEmail) : IInlineNode
{
    public string Type => "autolink";
}

public sealed record RawInlineNode(string Format, string Value) : IInlineNode
{
    public string Type => "raw_inline";
}

public sealed record HardBreakNode() : IInlineNode
{
    public string Type => "hard_break";
}

public sealed record SoftBreakNode() : IInlineNode
{
    public string Type => "soft_break";
}
