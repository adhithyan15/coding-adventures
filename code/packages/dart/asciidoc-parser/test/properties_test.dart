/// Property tests for `coding_adventures_asciidoc_parser`, using
/// the in-lane `pbt` package: totality over hostile strings, a
/// whole-tree AST invariant sweep, determinism, and marker round-trips
/// — all with fixed seeds.
library;

import 'package:coding_adventures_asciidoc_parser/asciidoc_parser.dart';
import 'package:coding_adventures_asciidoc_parser/src/block_parser.dart';
import 'package:coding_adventures_asciidoc_parser/src/inline_parser.dart';
import 'package:coding_adventures_pbt/coding_adventures_pbt.dart';
import 'package:test/test.dart';

void check(String name, Verdict verdict) {
  if (verdict is Fail) {
    fail('$name falsified: [${verdict.failureId}] ${verdict.message}');
  }
}

Gen<int> smallInt(int lo, int hi) =>
    Gen.integer(BigInt.from(lo), BigInt.from(hi)).map((i) => i.toInt());

/// Structural alphabet: markup, breaks, brackets, whitespace, plus
/// plain words and a little Unicode.
Gen<String> hostileText(int minLen, int maxLen) {
  const pieces = [
    '=',
    '*',
    '_',
    '`',
    '[',
    ']',
    '(',
    ')',
    '<',
    '>',
    '-',
    '.',
    '+',
    '/',
    ':',
    ',',
    ' ',
    '  ',
    '\n',
    '\r',
    '\t',
    '\\',
    '#',
    'a',
    'Z',
    '0',
    'https://',
    'http://',
    'link:',
    'image:',
    '<<',
    '>>',
    '----',
    '____',
    '++++',
    '....',
    "'''",
    'é',
    '🎉',
  ];
  return Gen.list(minLen, maxLen, smallInt(0, pieces.length - 1))
      .map((idx) => idx.map((i) => pieces[i]).join());
}

Gen<String> wordGen(int minLen, int maxLen) {
  const alphabet = 'abcdefghijklmnopqrstuvwxyz0123456789';
  return Gen.list(minLen, maxLen, smallInt(0, alphabet.length - 1))
      .map((idx) => idx.map((i) => alphabet[i]).join());
}

/// Every invariant the parser promises about the trees it builds.
bool invariantsHold(Node node) {
  var ok = true;
  void checkInline(InlineNode inline) {
    switch (inline) {
      case TextNode():
        break;
      case EmphasisNode(children: final children):
        for (final c in children) {
          checkInline(c);
        }
      case StrongNode(children: final children):
        for (final c in children) {
          checkInline(c);
        }
      case StrikethroughNode():
        ok = false; // the parser never makes strikethrough
      case CodeSpanNode():
        break;
      case LinkNode(title: final title, children: final children):
        if (title != null) ok = false;
        for (final c in children) {
          checkInline(c);
        }
      case ImageNode(title: final title):
        if (title != null) ok = false;
      case AutolinkNode(isEmail: final isEmail):
        if (isEmail) ok = false;
      case RawInlineNode():
        ok = false; // the parser never makes raw inline
      case HardBreakNode():
        break;
      case SoftBreakNode():
        break;
    }
  }

  void checkBlock(BlockNode block) {
    switch (block) {
      case HeadingNode(level: final level, children: final children):
        if (level < 1 || level > 6) ok = false;
        for (final c in children) {
          checkInline(c);
        }
      case ParagraphNode(children: final children):
        for (final c in children) {
          checkInline(c);
        }
      case CodeBlockNode(language: _, value: final value):
        if (value.isNotEmpty && !value.endsWith('\n')) ok = false;
      case BlockquoteNode(children: final children):
        for (final c in children) {
          checkBlock(c);
        }
      case ListNode(
          ordered: _,
          start: final start,
          tight: final tight,
          children: final children
        ):
        if (start != 1 || !tight) ok = false;
        for (final c in children) {
          switch (c) {
            case ListItemNode(children: final itemChildren):
              for (final b in itemChildren) {
                checkBlock(b);
              }
            case TaskItemNode():
              ok = false; // the parser never makes task items
          }
        }
      case ThematicBreakNode():
        break;
      case RawBlockNode(format: final format, value: _):
        if (format != 'html') ok = false;
      case TableNode():
      case TableRowNode():
      case TableCellNode():
        ok = false; // the parser never makes tables
      case DocumentNode(children: final children):
        for (final c in children) {
          checkBlock(c);
        }
      case ListItemNode(children: final children):
        // Only reachable nested; still must hold recursively.
        for (final c in children) {
          checkBlock(c);
        }
      case TaskItemNode():
        ok = false;
    }
  }

  if (node is DocumentNode) {
    for (final c in node.children) {
      checkBlock(c);
    }
  } else if (node is BlockNode) {
    checkBlock(node);
  } else if (node is InlineNode) {
    checkInline(node);
  }
  return ok;
}

void main() {
  group('properties', () {
    test('parse is total and invariant-clean', () {
      check(
        'totality',
        checkProperty<String>(
          BigInt.from(2201),
          PbtOptions(numTests: 400, seed: BigInt.from(2201)),
          (rng) => hostileText(0, 60).sample(rng),
          (input) {
            final doc = parse(input);
            if (!invariantsHold(doc)) return false;
            final inlines = parseInlines(input);
            for (final n in inlines) {
              if (!invariantsHold(n)) return false;
            }
            return true;
          },
        ),
      );
    });

    test('parse is deterministic', () {
      check(
        'determinism',
        checkProperty<String>(
          BigInt.from(2202),
          PbtOptions(numTests: 200, seed: BigInt.from(2202)),
          (rng) => hostileText(0, 60).sample(rng),
          (input) =>
              parse(input) == parse(input) &&
              parse(input).hashCode == parse(input).hashCode,
        ),
      );
    });

    test('blank inputs yield empty documents', () {
      check(
        'blank-empty',
        checkProperty<String>(
          BigInt.from(2203),
          PbtOptions(numTests: 100, seed: BigInt.from(2203)),
          (rng) => Gen.list(0, 10, Gen.constant('\n'))
              .map((l) => l.join())
              .sample(rng),
          (input) => parse(input).children.isEmpty,
        ),
      );
    });

    test('single-line headings stay singular', () {
      check(
        'heading-singular',
        checkProperty<(int, String)>(
          BigInt.from(2204),
          PbtOptions(numTests: 200, seed: BigInt.from(2204)),
          (rng) {
            final level = smallInt(1, 6).sample(rng);
            final content = wordGen(0, 12).sample(rng);
            return (level, content);
          },
          (input) {
            final (level, content) = input;
            final doc = parse('${'=' * level} $content\n');
            if (doc.children.length != 1) return false;
            final h = doc.children[0];
            return h is HeadingNode && h.level == level;
          },
        ),
      );
    });

    test('fenced code round-trips', () {
      check(
        'code-roundtrip',
        checkProperty<String>(
          BigInt.from(2205),
          PbtOptions(numTests: 200, seed: BigInt.from(2205)),
          (rng) => wordGen(1, 20).sample(rng),
          (content) {
            final doc = parse('----\n$content\n----\n');
            if (doc.children.length != 1) return false;
            final cb = doc.children[0];
            return cb is CodeBlockNode &&
                cb.language == null &&
                cb.value == '$content\n';
          },
        ),
      );
    });

    test('inline markers wrap words', () {
      check(
        'marker-wrap',
        checkProperty<String>(
          BigInt.from(2206),
          PbtOptions(numTests: 200, seed: BigInt.from(2206)),
          (rng) => wordGen(1, 10).sample(rng),
          (word) {
            bool single<T extends InlineNode>(String src) {
              final nodes = parseInlines(src);
              return nodes.length == 1 && nodes[0] is T;
            }

            return single<StrongNode>('**$word**') &&
                single<EmphasisNode>('__${word}__') &&
                single<StrongNode>('*$word*') &&
                single<EmphasisNode>('_${word}_') &&
                single<CodeSpanNode>('`$word`');
          },
        ),
      );
    });

    test('breaks split lines', () {
      check(
        'break-split',
        checkProperty<(String, String)>(
          BigInt.from(2207),
          PbtOptions(numTests: 200, seed: BigInt.from(2207)),
          (rng) {
            final a = wordGen(1, 8).sample(rng);
            final b = wordGen(1, 8).sample(rng);
            return (a, b);
          },
          (input) {
            final (a, b) = input;
            final soft = parseInlines('$a\n$b');
            final hard = parseInlines('$a  \n$b');
            return soft.length == 3 &&
                soft[1] is SoftBreakNode &&
                hard.length == 3 &&
                hard[1] is HardBreakNode;
          },
        ),
      );
    });

    test('block dispatch agrees with line classifiers', () {
      // A lone heading/list/delim line parses to the block its
      // classifier predicts — the dispatch table and the helpers
      // cannot drift apart.
      check(
        'dispatch-agreement',
        checkProperty<String>(
          BigInt.from(2208),
          PbtOptions(numTests: 200, seed: BigInt.from(2208)),
          (rng) => wordGen(1, 10).sample(rng),
          (word) {
            final headingLine = '== $word';
            final listLine = '* $word';
            final h = parseHeadingLine(headingLine);
            final l = parseListLine(listLine);
            if (h == null || h.level != 2 || l == null || l.level != 1) {
              return false;
            }
            final dh = parse('$headingLine\n').children;
            final dl = parse('$listLine\n').children;
            return dh.length == 1 &&
                dh[0] is HeadingNode &&
                dl.length == 1 &&
                dl[0] is ListNode;
          },
        ),
      );
    });
  });
}
