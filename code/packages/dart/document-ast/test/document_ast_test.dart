import 'package:coding_adventures_document_ast/document_ast.dart';
import 'package:test/test.dart';

void main() {
  group('block nodes', () {
    test('builds the core TE00 document structure', () {
      final heading = HeadingNode(1, [TextNode('Document AST')]);
      final paragraph = ParagraphNode([
        TextNode('Hello '),
        EmphasisNode([TextNode('portable')]),
        const SoftBreakNode(),
        StrongNode([TextNode('world')]),
      ]);
      final document = DocumentNode([heading, paragraph]);

      expect(document.type, 'document');
      expect(document.children, hasLength(2));
      expect(document.children.first, same(heading));
      expect(heading.type, 'heading');
      expect(heading.level, 1);
      expect(paragraph.type, 'paragraph');
      expect(paragraph.children.map((node) => node.type), [
        'text',
        'emphasis',
        'soft_break',
        'strong',
      ]);
    });

    test('supports all heading levels and rejects invalid levels', () {
      for (var level = 1; level <= 6; level++) {
        expect(HeadingNode(level, []).level, level);
      }
      expect(() => HeadingNode(0, []), throwsArgumentError);
      expect(() => HeadingNode(7, []), throwsArgumentError);
    });

    test('represents literal, quotation, and raw blocks', () {
      final code = CodeBlockNode(language: 'dart', value: 'final x = 1;\n');
      final unlabelled = CodeBlockNode(language: null, value: 'plain\n');
      final quote = BlockquoteNode([
        ParagraphNode([TextNode('quoted')]),
        BlockquoteNode([]),
      ]);
      final raw = RawBlockNode(format: 'latex', value: r'\textbf{bold}');

      expect(code.type, 'code_block');
      expect(code.language, 'dart');
      expect(code.value, endsWith('\n'));
      expect(unlabelled.language, isNull);
      expect(quote.type, 'blockquote');
      expect(quote.children.last.type, 'blockquote');
      expect(raw.type, 'raw_block');
      expect(raw.format, 'latex');
      expect(raw.value, r'\textbf{bold}');
      expect(const ThematicBreakNode().type, 'thematic_break');
    });

    test('represents regular and task list items', () {
      final regular = ListItemNode([
        ParagraphNode([TextNode('first')]),
      ]);
      final task = TaskItemNode(
        checked: true,
        children: [
          ParagraphNode([TextNode('ship it')])
        ],
      );
      final unordered = ListNode(
        ordered: false,
        start: null,
        tight: true,
        children: [regular, task],
      );
      final ordered = ListNode(
        ordered: true,
        start: 42,
        tight: false,
        children: [regular],
      );

      expect(unordered.type, 'list');
      expect(unordered.children.map((node) => node.type), [
        'list_item',
        'task_item',
      ]);
      expect(unordered.start, isNull);
      expect(unordered.tight, isTrue);
      expect(task.checked, isTrue);
      expect(ordered.start, 42);
      expect(ordered.ordered, isTrue);
    });

    test('represents the stabilized table model', () {
      final header = TableRowNode(
        isHeader: true,
        children: [
          TableCellNode([TextNode('Name')])
        ],
      );
      final body = TableRowNode(
        isHeader: false,
        children: [
          TableCellNode([
            StrongNode([TextNode('Ada')])
          ]),
        ],
      );
      final table = TableNode(
        align: [TableAlignment.left],
        children: [header, body],
      );

      expect(table.type, 'table');
      expect(table.align, [TableAlignment.left]);
      expect(table.children, hasLength(2));
      expect(table.children.first.type, 'table_row');
      expect(table.children.first.isHeader, isTrue);
      expect(table.children.first.children.first.type, 'table_cell');
      expect(table.children.last.children.first.children.first.type, 'strong');
      expect(TableAlignment.values.map((value) => value.name), [
        'left',
        'right',
        'center',
      ]);
    });
  });

  group('inline nodes', () {
    test('represents text and nested formatting', () {
      final text = TextNode('Hello & κόσμε');
      final emphasis = EmphasisNode([text, CodeSpanNode('&amp;')]);
      final strong = StrongNode([emphasis]);
      final strike = StrikethroughNode([strong]);

      expect(text.type, 'text');
      expect(text.value, 'Hello & κόσμε');
      expect(emphasis.type, 'emphasis');
      expect((emphasis.children.last as CodeSpanNode).value, '&amp;');
      expect(strong.type, 'strong');
      expect(strike.type, 'strikethrough');
    });

    test('represents resolved links, images, and autolinks', () {
      final link = LinkNode(
        destination: 'https://example.com',
        title: 'Example',
        children: [TextNode('visit')],
      );
      final untitled = LinkNode(
        destination: '/relative',
        title: null,
        children: [],
      );
      final image = ImageNode(
        destination: 'cat.png',
        title: null,
        alt: 'a tabby cat',
      );
      final email = AutolinkNode(
        destination: 'user@example.com',
        isEmail: true,
      );
      final url = AutolinkNode(
        destination: 'https://example.com',
        isEmail: false,
      );

      expect(link.type, 'link');
      expect(link.title, 'Example');
      expect(link.children.single.type, 'text');
      expect(untitled.title, isNull);
      expect(image.type, 'image');
      expect(image.alt, 'a tabby cat');
      expect(email.type, 'autolink');
      expect(email.isEmail, isTrue);
      expect(url.isEmail, isFalse);
    });

    test('represents raw content and break leaves', () {
      final raw = RawInlineNode(format: 'html', value: '<em>raw</em>');
      const hard = HardBreakNode();
      const soft = SoftBreakNode();

      expect(raw.type, 'raw_inline');
      expect(raw.format, 'html');
      expect(raw.value, '<em>raw</em>');
      expect(hard.type, 'hard_break');
      expect(soft.type, 'soft_break');
    });
  });

  group('sealed discriminators', () {
    test('covers every block-node discriminator', () {
      final nodes = <BlockNode>[
        DocumentNode([]),
        HeadingNode(1, []),
        ParagraphNode([]),
        CodeBlockNode(language: null, value: ''),
        BlockquoteNode([]),
        ListNode(ordered: false, start: null, tight: true, children: []),
        ListItemNode([]),
        TaskItemNode(checked: false, children: []),
        const ThematicBreakNode(),
        RawBlockNode(format: 'html', value: ''),
        TableNode(align: [], children: []),
        TableRowNode(isHeader: false, children: []),
        TableCellNode([]),
      ];

      expect(nodes.map(_classify), [
        'document',
        'heading',
        'paragraph',
        'code_block',
        'blockquote',
        'list',
        'list_item',
        'task_item',
        'thematic_break',
        'raw_block',
        'table',
        'table_row',
        'table_cell',
      ]);
    });

    test('covers every inline-node discriminator', () {
      final nodes = <InlineNode>[
        TextNode(''),
        EmphasisNode([]),
        StrongNode([]),
        StrikethroughNode([]),
        CodeSpanNode(''),
        LinkNode(destination: '/', title: null, children: []),
        ImageNode(destination: '/', title: null, alt: ''),
        AutolinkNode(destination: 'x@y.example', isEmail: true),
        RawInlineNode(format: 'html', value: ''),
        const HardBreakNode(),
        const SoftBreakNode(),
      ];

      expect(nodes.map(_classify), [
        'text',
        'emphasis',
        'strong',
        'strikethrough',
        'code_span',
        'link',
        'image',
        'autolink',
        'raw_inline',
        'hard_break',
        'soft_break',
      ]);
    });
  });

  group('immutability and value semantics', () {
    test('defensively copies child and alignment lists', () {
      final inlineChildren = <InlineNode>[TextNode('before')];
      final paragraph = ParagraphNode(inlineChildren);
      inlineChildren.add(TextNode('after'));

      final blockChildren = <BlockNode>[paragraph];
      final document = DocumentNode(blockChildren);
      blockChildren.clear();

      final align = <TableAlignment?>[TableAlignment.center, null];
      final table = TableNode(align: align, children: []);
      align[0] = TableAlignment.left;

      expect(paragraph.children, hasLength(1));
      expect(document.children, [paragraph]);
      expect(table.align, [TableAlignment.center, null]);
      expect(
          () => paragraph.children.add(TextNode('no')), throwsUnsupportedError);
      expect(() => document.children.clear(), throwsUnsupportedError);
      expect(
          () => table.align.add(TableAlignment.right), throwsUnsupportedError);
    });

    test('uses structural equality and stable hash codes', () {
      final left = DocumentNode([
        HeadingNode(2, [TextNode('Title')]),
        ParagraphNode([TextNode('Body'), const SoftBreakNode()]),
      ]);
      final right = DocumentNode([
        HeadingNode(2, [TextNode('Title')]),
        ParagraphNode([TextNode('Body'), const SoftBreakNode()]),
      ]);
      final different = DocumentNode([
        HeadingNode(3, [TextNode('Title')]),
      ]);

      expect(left, equals(right));
      expect(left.hashCode, right.hashCode);
      expect(left, isNot(equals(different)));
      expect(TextNode('same'), equals(TextNode('same')));
      expect(TextNode('same'), isNot(equals(TextNode('different'))));
    });

    test('applies value semantics to every stabilized node variant', () {
      final pairs = <(Node, Node)>[
        (DocumentNode([]), DocumentNode([])),
        (HeadingNode(2, []), HeadingNode(2, [])),
        (ParagraphNode([]), ParagraphNode([])),
        (
          const CodeBlockNode(language: 'dart', value: 'x\n'),
          const CodeBlockNode(language: 'dart', value: 'x\n'),
        ),
        (BlockquoteNode([]), BlockquoteNode([])),
        (
          ListNode(
            ordered: true,
            start: 3,
            tight: false,
            children: [],
          ),
          ListNode(
            ordered: true,
            start: 3,
            tight: false,
            children: [],
          ),
        ),
        (ListItemNode([]), ListItemNode([])),
        (
          TaskItemNode(checked: true, children: []),
          TaskItemNode(checked: true, children: []),
        ),
        (const ThematicBreakNode(), const ThematicBreakNode()),
        (
          const RawBlockNode(format: 'html', value: '<hr>'),
          const RawBlockNode(format: 'html', value: '<hr>'),
        ),
        (
          TableNode(align: [TableAlignment.right, null], children: []),
          TableNode(align: [TableAlignment.right, null], children: []),
        ),
        (
          TableRowNode(isHeader: true, children: []),
          TableRowNode(isHeader: true, children: []),
        ),
        (TableCellNode([]), TableCellNode([])),
        (const TextNode('text'), const TextNode('text')),
        (EmphasisNode([]), EmphasisNode([])),
        (StrongNode([]), StrongNode([])),
        (StrikethroughNode([]), StrikethroughNode([])),
        (const CodeSpanNode('code'), const CodeSpanNode('code')),
        (
          LinkNode(destination: '/', title: 'home', children: []),
          LinkNode(destination: '/', title: 'home', children: []),
        ),
        (
          const ImageNode(destination: 'x.png', title: null, alt: 'x'),
          const ImageNode(destination: 'x.png', title: null, alt: 'x'),
        ),
        (
          const AutolinkNode(destination: 'x@y.test', isEmail: true),
          const AutolinkNode(destination: 'x@y.test', isEmail: true),
        ),
        (
          const RawInlineNode(format: 'html', value: '<b>x</b>'),
          const RawInlineNode(format: 'html', value: '<b>x</b>'),
        ),
        (const HardBreakNode(), const HardBreakNode()),
        (const SoftBreakNode(), const SoftBreakNode()),
      ];

      expect(pairs.map((pair) => pair.$1.type).toSet(), hasLength(24));
      for (final (left, right) in pairs) {
        expect(left, equals(right), reason: left.type);
        expect(left.hashCode, right.hashCode, reason: left.type);
      }
    });
  });

  test('walks a nested document through typed containment', () {
    final document = DocumentNode([
      HeadingNode(1, [TextNode('Hello')]),
      BlockquoteNode([
        ListNode(
          ordered: false,
          start: null,
          tight: true,
          children: [
            ListItemNode([
              ParagraphNode([
                TextNode('World'),
                const SoftBreakNode(),
                TextNode('!'),
              ]),
            ]),
          ],
        ),
      ]),
    ]);

    final texts = <String>[];
    _collectText(document, texts);
    expect(texts, ['Hello', 'World', '!']);
  });
}

String _classify(Node node) => switch (node) {
      DocumentNode() => 'document',
      HeadingNode() => 'heading',
      ParagraphNode() => 'paragraph',
      CodeBlockNode() => 'code_block',
      BlockquoteNode() => 'blockquote',
      ListNode() => 'list',
      ListItemNode() => 'list_item',
      TaskItemNode() => 'task_item',
      ThematicBreakNode() => 'thematic_break',
      RawBlockNode() => 'raw_block',
      TableNode() => 'table',
      TableRowNode() => 'table_row',
      TableCellNode() => 'table_cell',
      TextNode() => 'text',
      EmphasisNode() => 'emphasis',
      StrongNode() => 'strong',
      StrikethroughNode() => 'strikethrough',
      CodeSpanNode() => 'code_span',
      LinkNode() => 'link',
      ImageNode() => 'image',
      AutolinkNode() => 'autolink',
      RawInlineNode() => 'raw_inline',
      HardBreakNode() => 'hard_break',
      SoftBreakNode() => 'soft_break',
    };

void _collectText(Node node, List<String> values) {
  switch (node) {
    case TextNode(:final value):
      values.add(value);
    case DocumentNode(:final children) ||
          BlockquoteNode(:final children) ||
          ListItemNode(:final children) ||
          TaskItemNode(:final children):
      for (final child in children) {
        _collectText(child, values);
      }
    case HeadingNode(:final children) ||
          ParagraphNode(:final children) ||
          EmphasisNode(:final children) ||
          StrongNode(:final children) ||
          StrikethroughNode(:final children) ||
          LinkNode(:final children) ||
          TableCellNode(:final children):
      for (final child in children) {
        _collectText(child, values);
      }
    case ListNode(:final children):
      for (final child in children) {
        _collectText(child, values);
      }
    case TableNode(:final children):
      for (final child in children) {
        _collectText(child, values);
      }
    case TableRowNode(:final children):
      for (final child in children) {
        _collectText(child, values);
      }
    case CodeBlockNode() ||
          ThematicBreakNode() ||
          RawBlockNode() ||
          CodeSpanNode() ||
          ImageNode() ||
          AutolinkNode() ||
          RawInlineNode() ||
          HardBreakNode() ||
          SoftBreakNode():
      return;
  }
}
