/// Hostile-input fuzz campaign for `coding_adventures_asciidoc_parser`,
/// using the in-lane `fuzz` package: raw bytes decode (malformed
/// sequences become U+FFFD, like any caller-supplied string) and drive
/// both `parse` and `parseInlines`. The parser is total — anything
/// thrown is a crash — and every tree must satisfy the AST invariant
/// sweep. Fixed seed, so the campaign reproduces exactly.
library;

import 'dart:convert';
import 'dart:typed_data';

import 'package:coding_adventures_asciidoc_parser/asciidoc_parser.dart';
import 'package:coding_adventures_asciidoc_parser/src/inline_parser.dart';
import 'package:coding_adventures_fuzz/coding_adventures_fuzz.dart';
import 'package:test/test.dart';

bool _blocksOk(BlockNode block) {
  switch (block) {
    case HeadingNode(level: final level, children: final children):
      if (level < 1 || level > 6) return false;
      return children.every(_inlinesOk);
    case ParagraphNode(children: final children):
      return children.every(_inlinesOk);
    case CodeBlockNode(value: final value):
      return value.isEmpty || value.endsWith('\n');
    case BlockquoteNode(children: final children):
      return children.every(_blocksOk);
    case ListNode(
        start: final start,
        tight: final tight,
        children: final children
      ):
      if (start != 1 || !tight) return false;
      return children
          .every((c) => c is ListItemNode && c.children.every(_blocksOk));
    case ThematicBreakNode():
      return true;
    case RawBlockNode(format: final format):
      return format == 'html';
    case DocumentNode(children: final children):
      return children.every(_blocksOk);
    case ListItemNode(children: final children):
      return children.every(_blocksOk);
    case TaskItemNode():
    case TableNode():
    case TableRowNode():
    case TableCellNode():
      return false;
  }
}

bool _inlinesOk(InlineNode inline) {
  switch (inline) {
    case TextNode():
    case CodeSpanNode():
    case HardBreakNode():
    case SoftBreakNode():
      return true;
    case EmphasisNode(children: final children):
    case StrongNode(children: final children):
      return children.every(_inlinesOk);
    case LinkNode(title: final title, children: final children):
      return title == null && children.every(_inlinesOk);
    case ImageNode(title: final title):
      return title == null;
    case AutolinkNode(isEmail: final isEmail):
      return !isEmail;
    case StrikethroughNode():
    case RawInlineNode():
      return false;
  }
}

FuzzOutcome target(Uint8List input, CoverageBitmap cov) {
  final text = utf8.decode(input, allowMalformed: true);
  cov.recordEdge(input.length % 64);
  late final DocumentNode doc;
  late final List<InlineNode> inlines;
  try {
    doc = parse(text);
    inlines = parseInlines(text);
  } catch (e) {
    return OutcomeCrash('unexpected-escape', ['${e.runtimeType}']);
  }
  cov.recordEdge(100 + (doc.children.length % 8));
  if (!doc.children.every(_blocksOk)) {
    return const OutcomeCrash('model-divergence', ['block-invariant']);
  }
  if (!inlines.every(_inlinesOk)) {
    return const OutcomeCrash('model-divergence', ['inline-invariant']);
  }
  // Determinism under the fuzzer's own mutations: re-parsing the same
  // string must produce an equal tree.
  if (parse(text) != doc) {
    return const OutcomeCrash('model-divergence', ['nondeterministic']);
  }
  return const OutcomePass();
}

void main() {
  test('fuzz hostile bytes never escape or diverge', () {
    final seeds = [
      Uint8List(0),
      Uint8List.fromList([0]),
      Uint8List.fromList(utf8.encode('= Title\n\nHello *world*!\n')),
      Uint8List.fromList(utf8.encode('----\ncode\n----\n')),
      Uint8List.fromList(utf8.encode('* a\n** b\n. c\n')),
      Uint8List.fromList(
          utf8.encode('link:https://e.com[x] image:i.png[y] <<a,b>>')),
      Uint8List.fromList(utf8.encode('**__*`[[<<{{((##')),
      Uint8List.fromList(List.filled(64, 0xFF)),
      Uint8List.fromList(List.generate(128, (i) => (i * 31 + 7) & 255)),
    ];
    final report = fuzzTarget(
      seeds,
      FuzzOptions(maxIterations: 2000, seed: BigInt.from(2209)),
      target,
    );
    expect(report.crashes, isEmpty,
        reason: report.crashes
            .map((c) => '${c.kind} ${c.frames.join(',')}')
            .join('\n'));
    expect(report.iterations, 2000);
  });
}
