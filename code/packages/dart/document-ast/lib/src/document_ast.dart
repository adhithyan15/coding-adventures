/// The horizontal alignment of a table column.
enum TableAlignment { left, right, center }

/// Base class for every Document AST node.
///
/// Nodes are immutable value objects. Equality and hashing are structural so
/// independently constructed trees with the same content compare equal.
sealed class Node {
  const Node();

  /// Stable snake_case discriminator shared by every implementation lane.
  String get type;

  List<Object?> get _fields;

  @override
  bool operator ==(Object other) =>
      identical(this, other) ||
      other.runtimeType == runtimeType &&
          other is Node &&
          _deepListEquals(_fields, other._fields);

  @override
  int get hashCode => Object.hash(
        runtimeType,
        Object.hashAll(_fields.map(_deepHash)),
      );
}

/// Structural nodes that form a document's block-level skeleton.
sealed class BlockNode extends Node {
  const BlockNode();
}

/// Character-level nodes contained by headings, paragraphs, and table cells.
sealed class InlineNode extends Node {
  const InlineNode();
}

/// The only node types accepted directly by a [ListNode].
sealed class ListChildNode extends BlockNode {
  const ListChildNode();
}

/// The root of a document.
final class DocumentNode extends BlockNode {
  DocumentNode(List<BlockNode> children)
      : children = List<BlockNode>.unmodifiable(children);

  @override
  String get type => 'document';

  final List<BlockNode> children;

  @override
  List<Object?> get _fields => [children];
}

/// A section heading with a level from one through six.
final class HeadingNode extends BlockNode {
  HeadingNode(this.level, List<InlineNode> children)
      : children = List<InlineNode>.unmodifiable(children) {
    if (level < 1 || level > 6) {
      throw ArgumentError.value(level, 'level', 'must be between 1 and 6');
    }
  }

  @override
  String get type => 'heading';

  final int level;
  final List<InlineNode> children;

  @override
  List<Object?> get _fields => [level, children];
}

/// A paragraph of inline content.
final class ParagraphNode extends BlockNode {
  ParagraphNode(List<InlineNode> children)
      : children = List<InlineNode>.unmodifiable(children);

  @override
  String get type => 'paragraph';

  final List<InlineNode> children;

  @override
  List<Object?> get _fields => [children];
}

/// A block of literal code or preformatted text.
final class CodeBlockNode extends BlockNode {
  const CodeBlockNode({required this.language, required this.value});

  @override
  String get type => 'code_block';

  final String? language;
  final String value;

  @override
  List<Object?> get _fields => [language, value];
}

/// A quotation containing block-level content.
final class BlockquoteNode extends BlockNode {
  BlockquoteNode(List<BlockNode> children)
      : children = List<BlockNode>.unmodifiable(children);

  @override
  String get type => 'blockquote';

  final List<BlockNode> children;

  @override
  List<Object?> get _fields => [children];
}

/// An ordered or unordered list.
final class ListNode extends BlockNode {
  ListNode({
    required this.ordered,
    required this.start,
    required this.tight,
    required List<ListChildNode> children,
  }) : children = List<ListChildNode>.unmodifiable(children);

  @override
  String get type => 'list';

  final bool ordered;
  final int? start;
  final bool tight;
  final List<ListChildNode> children;

  @override
  List<Object?> get _fields => [ordered, start, tight, children];
}

/// One regular item in a list.
final class ListItemNode extends ListChildNode {
  ListItemNode(List<BlockNode> children)
      : children = List<BlockNode>.unmodifiable(children);

  @override
  String get type => 'list_item';

  final List<BlockNode> children;

  @override
  List<Object?> get _fields => [children];
}

/// One checkbox item in a task list.
final class TaskItemNode extends ListChildNode {
  TaskItemNode({required this.checked, required List<BlockNode> children})
      : children = List<BlockNode>.unmodifiable(children);

  @override
  String get type => 'task_item';

  final bool checked;
  final List<BlockNode> children;

  @override
  List<Object?> get _fields => [checked, children];
}

/// A horizontal separator.
final class ThematicBreakNode extends BlockNode {
  const ThematicBreakNode();

  @override
  String get type => 'thematic_break';

  @override
  List<Object?> get _fields => const [];
}

/// Format-specific block content passed through by matching renderers.
final class RawBlockNode extends BlockNode {
  const RawBlockNode({required this.format, required this.value});

  @override
  String get type => 'raw_block';

  final String format;
  final String value;

  @override
  List<Object?> get _fields => [format, value];
}

/// A table whose alignments correspond to its columns.
final class TableNode extends BlockNode {
  TableNode({
    required List<TableAlignment?> align,
    required List<TableRowNode> children,
  })  : align = List<TableAlignment?>.unmodifiable(align),
        children = List<TableRowNode>.unmodifiable(children);

  @override
  String get type => 'table';

  final List<TableAlignment?> align;
  final List<TableRowNode> children;

  @override
  List<Object?> get _fields => [align, children];
}

/// One header or body row inside a table.
final class TableRowNode extends BlockNode {
  TableRowNode({
    required this.isHeader,
    required List<TableCellNode> children,
  }) : children = List<TableCellNode>.unmodifiable(children);

  @override
  String get type => 'table_row';

  final bool isHeader;
  final List<TableCellNode> children;

  @override
  List<Object?> get _fields => [isHeader, children];
}

/// One table cell containing inline content.
final class TableCellNode extends BlockNode {
  TableCellNode(List<InlineNode> children)
      : children = List<InlineNode>.unmodifiable(children);

  @override
  String get type => 'table_cell';

  final List<InlineNode> children;

  @override
  List<Object?> get _fields => [children];
}

/// Plain decoded Unicode text.
final class TextNode extends InlineNode {
  const TextNode(this.value);

  @override
  String get type => 'text';

  final String value;

  @override
  List<Object?> get _fields => [value];
}

/// Stressed inline content.
final class EmphasisNode extends InlineNode {
  EmphasisNode(List<InlineNode> children)
      : children = List<InlineNode>.unmodifiable(children);

  @override
  String get type => 'emphasis';

  final List<InlineNode> children;

  @override
  List<Object?> get _fields => [children];
}

/// Strongly emphasized inline content.
final class StrongNode extends InlineNode {
  StrongNode(List<InlineNode> children)
      : children = List<InlineNode>.unmodifiable(children);

  @override
  String get type => 'strong';

  final List<InlineNode> children;

  @override
  List<Object?> get _fields => [children];
}

/// Inline content marked as deleted or struck through.
final class StrikethroughNode extends InlineNode {
  StrikethroughNode(List<InlineNode> children)
      : children = List<InlineNode>.unmodifiable(children);

  @override
  String get type => 'strikethrough';

  final List<InlineNode> children;

  @override
  List<Object?> get _fields => [children];
}

/// Literal inline code.
final class CodeSpanNode extends InlineNode {
  const CodeSpanNode(this.value);

  @override
  String get type => 'code_span';

  final String value;

  @override
  List<Object?> get _fields => [value];
}

/// A resolved hyperlink with inline label content.
final class LinkNode extends InlineNode {
  LinkNode({
    required this.destination,
    required this.title,
    required List<InlineNode> children,
  }) : children = List<InlineNode>.unmodifiable(children);

  @override
  String get type => 'link';

  final String destination;
  final String? title;
  final List<InlineNode> children;

  @override
  List<Object?> get _fields => [destination, title, children];
}

/// An image reference with resolved destination and plain-text alternative.
final class ImageNode extends InlineNode {
  const ImageNode({
    required this.destination,
    required this.title,
    required this.alt,
  });

  @override
  String get type => 'image';

  final String destination;
  final String? title;
  final String alt;

  @override
  List<Object?> get _fields => [destination, title, alt];
}

/// An automatically recognized URL or email address.
final class AutolinkNode extends InlineNode {
  const AutolinkNode({required this.destination, required this.isEmail});

  @override
  String get type => 'autolink';

  final String destination;
  final bool isEmail;

  @override
  List<Object?> get _fields => [destination, isEmail];
}

/// Format-specific inline content passed through by matching renderers.
final class RawInlineNode extends InlineNode {
  const RawInlineNode({required this.format, required this.value});

  @override
  String get type => 'raw_inline';

  final String format;
  final String value;

  @override
  List<Object?> get _fields => [format, value];
}

/// A forced line break.
final class HardBreakNode extends InlineNode {
  const HardBreakNode();

  @override
  String get type => 'hard_break';

  @override
  List<Object?> get _fields => const [];
}

/// A source line break that a renderer may collapse to whitespace.
final class SoftBreakNode extends InlineNode {
  const SoftBreakNode();

  @override
  String get type => 'soft_break';

  @override
  List<Object?> get _fields => const [];
}

bool _deepListEquals(List<Object?> left, List<Object?> right) {
  if (left.length != right.length) return false;
  for (var index = 0; index < left.length; index++) {
    if (!_deepEquals(left[index], right[index])) return false;
  }
  return true;
}

bool _deepEquals(Object? left, Object? right) {
  if (identical(left, right)) return true;
  if (left is List<Object?> && right is List<Object?>) {
    return _deepListEquals(left, right);
  }
  return left == right;
}

int _deepHash(Object? value) {
  if (value is List<Object?>) {
    return Object.hashAll(value.map(_deepHash));
  }
  return value.hashCode;
}
