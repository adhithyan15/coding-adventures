/// Unit tests for `coding_adventures_asciidoc_parser`: all 34
/// behavioral reference tests, plus the corners the reference leaves
/// uncovered — fence leniency, EOF shapes, pending-language lifetime,
/// level-skipping, type-change flushes, constrained-boundary failures,
/// macro fallbacks, URL terminator exactness, break edges, CRLF
/// preservation, and Unicode. (`test_version` has no mirror: packaging
/// metadata lives in pubspec.yaml on every Dart lane, by convention.)
library;

import 'package:coding_adventures_asciidoc_parser/asciidoc_parser.dart';
import 'package:coding_adventures_asciidoc_parser/src/inline_parser.dart';
import 'package:test/test.dart';

TextNode text(String s) => TextNode(s);

void main() {
  group('blocks', () {
    test('empty input yields no blocks', () {
      expect(parse('').children, isEmpty);
    });

    test('blank lines yield no blocks', () {
      expect(parse('\n\n\n').children, isEmpty);
    });

    test('level-1 heading', () {
      final doc = parse('= Hello World\n');
      expect(doc.children, hasLength(1));
      final h = doc.children[0] as HeadingNode;
      expect(h.level, 1);
      expect(h.children, [text('Hello World')]);
    });

    test('level-2 heading', () {
      final h = parse('== Section\n').children[0] as HeadingNode;
      expect(h.level, 2);
    });

    test('level-6 heading', () {
      final h = parse('====== Deep\n').children[0] as HeadingNode;
      expect(h.level, 6);
    });

    test('seven equals signs is not a heading', () {
      // The sixth `=` is followed by `=`, not a space, so the line
      // stays paragraph text — reference behavior (Python clamps).
      final doc = parse('======= Deep\n');
      expect(doc.children[0], isA<ParagraphNode>());
    });

    test('heading needs a space after the markers', () {
      expect(parse('==Section\n').children[0], isA<ParagraphNode>());
    });

    test('heading content is trimmed and inline-parsed', () {
      final h = parse('==  *Hi*  \n').children[0] as HeadingNode;
      expect(h.children, hasLength(1));
      expect(h.children[0], isA<StrongNode>());
    });

    test('bare marker yields an empty heading', () {
      final h = parse('= \n').children[0] as HeadingNode;
      expect(h.level, 1);
      expect(h.children, isEmpty);
    });

    test('single-line paragraph', () {
      final doc = parse('Hello world\n');
      expect(doc.children, hasLength(1));
      final p = doc.children[0] as ParagraphNode;
      expect(p.children[0], text('Hello world'));
    });

    test('multi-line paragraph joins with soft breaks', () {
      final p = parse('Line one\nLine two\n').children[0] as ParagraphNode;
      expect(p.children,
          [text('Line one'), const SoftBreakNode(), text('Line two')]);
    });

    test('blank line separates paragraphs', () {
      expect(parse('First\n\nSecond\n').children, hasLength(2));
    });

    test('thematic break', () {
      final doc = parse("'''\n");
      expect(doc.children, hasLength(1));
      expect(doc.children[0], isA<ThematicBreakNode>());
    });

    test('longer quote runs still break', () {
      expect(parse("'''''\n").children[0], isA<ThematicBreakNode>());
    });

    test('code block keeps its trailing newline', () {
      final doc = parse('----\nfoo := bar\n----\n');
      expect(doc.children, hasLength(1));
      final cb = doc.children[0] as CodeBlockNode;
      expect(cb.language, isNull);
      expect(cb.value, 'foo := bar\n');
    });

    test('source attribute sets the code language', () {
      final doc = parse('[source,go]\n----\nfmt.Println()\n----\n');
      final cb = doc.children[0] as CodeBlockNode;
      expect(cb.language, 'go');
    });

    test('source attribute tolerates spaces and case', () {
      final doc = parse('[Source, go ]\n----\nx\n----\n');
      final cb = doc.children[0] as CodeBlockNode;
      expect(cb.language, 'go');
    });

    test('bare source attribute clears the pending language', () {
      final doc = parse('[source,go]\n[source]\n----\nx\n----\n');
      final cb = doc.children[0] as CodeBlockNode;
      expect(cb.language, isNull);
    });

    test('other attributes leave the pending language armed', () {
      final doc = parse('[source,go]\n[role]\n----\nx\n----\n');
      final cb = doc.children[0] as CodeBlockNode;
      expect(cb.language, 'go');
    });

    test('literal block never takes the pending language', () {
      // …and the literal close does not consume it either: the later
      // code block still inherits `go`.
      final doc = parse('[source,go]\n....\nlit\n....\n----\ncode\n----\n');
      expect(doc.children, hasLength(2));
      expect((doc.children[0] as CodeBlockNode).language, isNull);
      expect((doc.children[1] as CodeBlockNode).language, 'go');
    });

    test('literal block', () {
      final doc = parse('....\nsome literal\n....\n');
      final cb = doc.children[0] as CodeBlockNode;
      expect(cb.language, isNull);
      expect(cb.value, 'some literal\n');
    });

    test('closed-but-empty code block emits empty', () {
      final cb = parse('----\n----\n').children[0] as CodeBlockNode;
      expect(cb.value, '');
    });

    test('unclosed code block runs to EOF', () {
      final cb = parse('----\nfoo\n').children[0] as CodeBlockNode;
      expect(cb.value, 'foo\n');
    });

    test('unclosed literal block runs to EOF', () {
      final cb = parse('....\nfoo\n').children[0] as CodeBlockNode;
      expect(cb.language, isNull);
      expect(cb.value, 'foo\n');
    });

    test('unclosed quote block runs to EOF', () {
      final bq = parse('____\nSome quote\n').children[0] as BlockquoteNode;
      expect(bq.children, hasLength(1));
      expect(bq.children[0], isA<ParagraphNode>());
    });

    test('unclosed empty code block still emits', () {
      // The spec's literal reading; Python/Ruby drop it instead.
      final doc = parse('----\n');
      expect(doc.children, hasLength(1));
      expect((doc.children[0] as CodeBlockNode).value, '');
    });

    test('any long-enough fence closes (lenient)', () {
      // No lane requires the closing token to equal the opening one.
      final cb = parse('----\ncode\n------\n').children[0] as CodeBlockNode;
      expect(cb.value, 'code\n');
    });

    test('indented fence is paragraph text', () {
      expect(parse('  ----\n').children[0], isA<ParagraphNode>());
    });

    test('passthrough block has no trailing newline', () {
      final doc = parse('++++\n<div>raw</div>\n++++\n');
      expect(doc.children, hasLength(1));
      final rb = doc.children[0] as RawBlockNode;
      expect(rb.format, 'html');
      expect(rb.value, '<div>raw</div>');
    });

    test('unclosed passthrough joins plainly', () {
      final rb = parse('++++\n<div>a</div>\n<div>b</div>\n').children[0]
          as RawBlockNode;
      expect(rb.value, '<div>a</div>\n<div>b</div>');
    });

    test('quote block re-parses its content', () {
      final doc = parse('____\nSome quote\n____\n');
      expect(doc.children, hasLength(1));
      final bq = doc.children[0] as BlockquoteNode;
      expect(bq.children, isNotEmpty);
      expect(bq.children[0], isA<ParagraphNode>());
    });

    test('quote block nests headings', () {
      final bq =
          parse('____\n== Inner Heading\n____\n').children[0] as BlockquoteNode;
      expect(bq.children, isNotEmpty);
      expect(bq.children[0], isA<HeadingNode>());
    });

    test('empty quote block is an empty blockquote', () {
      final bq = parse('____\n____\n').children[0] as BlockquoteNode;
      expect(bq.children, isEmpty);
    });

    test('quote re-parse starts fresh (no language leak)', () {
      final bq = parse('[source,go]\n____\n----\nx\n----\n____\n').children[0]
          as BlockquoteNode;
      final cb = bq.children[0] as CodeBlockNode;
      expect(cb.language, isNull);
    });

    test('unordered list', () {
      final doc = parse('* item one\n* item two\n');
      expect(doc.children, hasLength(1));
      final list = doc.children[0] as ListNode;
      expect(list.ordered, isFalse);
      expect(list.start, 1);
      expect(list.tight, isTrue);
      expect(list.children, hasLength(2));
      final first = list.children[0] as ListItemNode;
      expect(first.children[0], isA<ParagraphNode>());
    });

    test('ordered list', () {
      final list =
          parse('. first\n. second\n. third\n').children[0] as ListNode;
      expect(list.ordered, isTrue);
      expect(list.children, hasLength(3));
    });

    test('nested list items recurse', () {
      final list = parse('* a\n** b\n* c\n').children[0] as ListNode;
      expect(list.children, hasLength(2));
      final first = list.children[0] as ListItemNode;
      expect(first.children, hasLength(2));
      final nested = first.children[1] as ListNode;
      expect(nested.ordered, isFalse);
      expect(nested.children, hasLength(1));
    });

    test('level-skipping items vanish', () {
      // Nothing sits at level 2, so the deep call finds no children —
      // reference behavior, pinned.
      final list = parse('* a\n*** c\n').children[0] as ListNode;
      expect(list.children, hasLength(1));
      final first = list.children[0] as ListItemNode;
      expect(first.children, hasLength(1));
    });

    test('type change flushes, stranding deep items', () {
      // The group key is the FIRST item's type: `.. b` flushes the
      // unordered run, then sits alone at level 2 with no level-1
      // parent — and vanishes, like the reference.
      final doc = parse('* a\n.. b\n');
      expect(doc.children, hasLength(1));
      final list = doc.children[0] as ListNode;
      expect(list.ordered, isFalse);
      expect(list.children, hasLength(1));
    });

    test('ordered nesting groups under ordered parents', () {
      final list = parse('. a\n.. b\n').children[0] as ListNode;
      expect(list.ordered, isTrue);
      final first = list.children[0] as ListItemNode;
      final nested = first.children[1] as ListNode;
      expect(nested.ordered, isTrue);
    });

    test('dotted line without a space is a paragraph', () {
      expect(parse('.5 x\n').children[0], isA<ParagraphNode>());
    });

    test('continuation marker is swallowed inside lists', () {
      final list = parse('* a\n+\n* b\n').children[0] as ListNode;
      expect(list.children, hasLength(2));
    });

    test('bare plus outside a list is a paragraph', () {
      expect(parse('+\n').children[0], isA<ParagraphNode>());
    });

    test('comment lines are skipped', () {
      final doc = parse('// this is a comment\nHello\n');
      expect(doc.children, hasLength(1));
      expect(doc.children[0], isA<ParagraphNode>());
    });

    test('comment needs no space', () {
      expect(parse('//x\nHello\n').children, hasLength(1));
    });

    test('triple slash is not a comment', () {
      // `///` falls through to paragraph text, like the reference
      // (Python skips it — known divergence).
      expect(parse('/// x\n').children[0], isA<ParagraphNode>());
    });

    test('indented comment marker is paragraph text', () {
      expect(parse('  // x\n').children[0], isA<ParagraphNode>());
    });

    test('heading then paragraph', () {
      final doc = parse('= Title\n\nSome text.\n');
      expect(doc.children, hasLength(2));
      expect(doc.children[0], isA<HeadingNode>());
      expect(doc.children[1], isA<ParagraphNode>());
    });

    test('heading interrupts a paragraph without a blank line', () {
      final doc = parse('Some text.\n= Title\n');
      expect(doc.children, hasLength(2));
      expect(doc.children[0], isA<ParagraphNode>());
      expect(doc.children[1], isA<HeadingNode>());
    });

    test('list interrupts a paragraph without a blank line', () {
      final doc = parse('Some text.\n* a\n');
      expect(doc.children, hasLength(2));
      expect(doc.children[0], isA<ParagraphNode>());
      expect(doc.children[1], isA<ListNode>());
    });

    test('fences do not interrupt a paragraph', () {
      final p = parse('a\n----\n').children[0] as ParagraphNode;
      expect(p.children, [text('a'), const SoftBreakNode(), text('----')]);
    });

    test('attribute lines do not interrupt a paragraph', () {
      final p = parse('a\n[source,go]\n').children[0] as ParagraphNode;
      expect(
          p.children, [text('a'), const SoftBreakNode(), text('[source,go]')]);
    });

    test('carriage returns survive in block content', () {
      // Lines are split on \n only; \r is ordinary text.
      final p = parse('a\r\nb\r\n').children[0] as ParagraphNode;
      expect(p.children, [text('a\r'), const SoftBreakNode(), text('b\r')]);
    });
  });

  group('inline', () {
    test('star means strong, not emphasis', () {
      final p = parse('Hello *world*!\n').children[0] as ParagraphNode;
      expect(p.children.whereType<StrongNode>(), hasLength(1));
      expect(p.children.whereType<EmphasisNode>(), isEmpty);
    });

    test('underscore means emphasis', () {
      final p = parse('Hello _world_!\n').children[0] as ParagraphNode;
      expect(p.children.whereType<EmphasisNode>(), hasLength(1));
    });

    test('unconstrained strong', () {
      final nodes = parseInlines('**bold**');
      expect(nodes, hasLength(1));
      final s = nodes[0] as StrongNode;
      expect(s.children, [text('bold')]);
    });

    test('unconstrained emphasis', () {
      final nodes = parseInlines('__em__');
      expect(nodes, hasLength(1));
      final e = nodes[0] as EmphasisNode;
      expect(e.children, [text('em')]);
    });

    test('bare double markers make an empty span', () {
      // Unconstrained parsing allows empty content, like the reference.
      final nodes = parseInlines('____');
      expect(nodes, hasLength(1));
      expect((nodes[0] as EmphasisNode).children, isEmpty);
    });

    test('triple star stays literal', () {
      expect(parseInlines('***'), [text('***')]);
    });

    test('word-bounded stars stay literal', () {
      expect(parseInlines('*a*b'), [text('*a*b')]);
      expect(parseInlines('a*b*c'), [text('a*b*c')]);
    });

    test('adjacent spans split', () {
      expect(parseInlines('*a* *b*'), [
        StrongNode([text('a')]),
        text(' '),
        StrongNode([text('b')])
      ]);
    });

    test('spans nest recursively', () {
      final nodes = parseInlines('*a _b_ c*');
      expect(nodes, hasLength(1));
      final s = nodes[0] as StrongNode;
      expect(s.children, [
        text('a '),
        EmphasisNode([text('b')]),
        text(' c')
      ]);
    });

    test('code span', () {
      final nodes = parseInlines('Use `foo()` now');
      expect(nodes, [text('Use '), const CodeSpanNode('foo()'), text(' now')]);
    });

    test('code span is verbatim', () {
      expect(parseInlines('`*x*`'), [const CodeSpanNode('*x*')]);
    });

    test('multi-tick code span', () {
      expect(parseInlines('``x``'), [const CodeSpanNode('x')]);
    });

    test('code span strips one space on both sides only', () {
      expect(parseInlines('` a `'), [const CodeSpanNode('a')]);
      expect(parseInlines('` a`'), [const CodeSpanNode(' a')]);
      expect(parseInlines('`a `'), [const CodeSpanNode('a ')]);
    });

    test('unclosed code span stays literal', () {
      expect(parseInlines('`abc'), [text('`abc')]);
    });

    test('link macro', () {
      final nodes = parseInlines('See link:https://example.com[Example].');
      final link = nodes.whereType<LinkNode>().single;
      expect(link.destination, 'https://example.com');
      expect(link.title, isNull);
      expect(link.children, [text('Example')]);
      expect(nodes.first, text('See '));
      expect(nodes.last, text('.'));
    });

    test('link URL runs to the first bracket', () {
      // Even across spaces — reference behavior, pinned.
      final link = parseInlines('link:foo bar [x]').single as LinkNode;
      expect(link.destination, 'foo bar ');
      expect(link.children, [text('x')]);
    });

    test('empty link label reuses the URL', () {
      final link = parseInlines('link:u[]').single as LinkNode;
      expect(link.children, [text('u')]);
    });

    test('broken link macros stay literal', () {
      expect(parseInlines('link:[x]'), [text('link:[x]')]);
      expect(parseInlines('link:u'), [text('link:u')]);
      expect(parseInlines('link:u[x'), [text('link:u[x')]);
    });

    test('image macro', () {
      final nodes = parseInlines('image:cat.png[A cat]');
      expect(nodes, hasLength(1));
      final img = nodes[0] as ImageNode;
      expect(img.destination, 'cat.png');
      expect(img.title, isNull);
      expect(img.alt, 'A cat');
    });

    test('broken image macro stays literal', () {
      expect(parseInlines('image:u'), [text('image:u')]);
    });

    test('cross reference with text', () {
      final nodes = parseInlines('See <<section-id,Section Title>>.');
      final link = nodes.whereType<LinkNode>().single;
      expect(link.destination, '#section-id');
      expect(link.children, [text('Section Title')]);
    });

    test('cross reference splits on the first comma', () {
      final link = parseInlines('<<a,b,c>>').single as LinkNode;
      expect(link.destination, '#a');
      expect(link.children, [text('b,c')]);
    });

    test('cross reference without text reuses the anchor', () {
      final nodes = parseInlines('<<my-anchor>>');
      expect(nodes, hasLength(1));
      final link = nodes[0] as LinkNode;
      expect(link.destination, '#my-anchor');
      expect(link.children, [text('my-anchor')]);
    });

    test('unclosed cross reference stays literal', () {
      expect(parseInlines('<<a'), [text('<<a')]);
    });

    test('bare URL becomes an autolink', () {
      final nodes = parseInlines('Visit https://example.com for details.');
      final al = nodes.whereType<AutolinkNode>().single;
      expect(al.destination, 'https://example.com');
      expect(al.isEmail, isFalse);
    });

    test('URL with bracket text becomes a link', () {
      final nodes = parseInlines('https://example.com[Click here]');
      expect(nodes, hasLength(1));
      final link = nodes[0] as LinkNode;
      expect(link.destination, 'https://example.com');
      expect(link.children, [text('Click here')]);
    });

    test('empty URL label reuses the URL', () {
      final link = parseInlines('https://a.com[]').single as LinkNode;
      expect(link.children, [text('https://a.com')]);
    });

    test('http scheme works too', () {
      final link = parseInlines('http://a[b]').single as LinkNode;
      expect(link.destination, 'http://a');
    });

    test('unclosed URL bracket falls back to autolink', () {
      expect(parseInlines('https://a.com[x'), [
        const AutolinkNode(destination: 'https://a.com', isEmail: false),
        text('[x')
      ]);
    });

    test('only space, newline, and bracket end a URL', () {
      // Tabs and carriage returns are part of the URL — reference
      // behavior, pinned.
      final tab =
          parseInlines('https://a.com\tb c').whereType<AutolinkNode>().single;
      expect(tab.destination, 'https://a.com\tb');
      final nodes = parseInlines('https://a.com\r\nx');
      expect(nodes[0],
          const AutolinkNode(destination: 'https://a.com\r', isEmail: false));
      expect(nodes[1], const SoftBreakNode());
    });

    test('soft break', () {
      final nodes = parseInlines('line one\nline two');
      expect(
          nodes, [text('line one'), const SoftBreakNode(), text('line two')]);
    });

    test('hard break on two trailing spaces', () {
      final nodes = parseInlines('line one  \nline two');
      expect(
          nodes, [text('line one'), const HardBreakNode(), text('line two')]);
    });

    test('hard break consumes only the last two spaces', () {
      expect(parseInlines('a   \nb'),
          [text('a '), const HardBreakNode(), text('b')]);
    });

    test('hard break then blank line stacks breaks', () {
      expect(parseInlines('a  \n\nb'),
          [text('a'), const HardBreakNode(), const SoftBreakNode(), text('b')]);
    });

    test('trailing spaces at EOF stay literal', () {
      expect(parseInlines('a  '), [text('a  ')]);
    });

    test('hard break on backslash newline', () {
      final nodes = parseInlines('line one\\\nline two');
      expect(
          nodes, [text('line one'), const HardBreakNode(), text('line two')]);
    });

    test('lone backslash at EOF stays literal', () {
      expect(parseInlines('\\'), [text('\\')]);
    });

    test('plain text', () {
      final nodes = parseInlines('just plain text');
      expect(nodes, hasLength(1));
      expect(nodes[0], text('just plain text'));
    });

    test('empty input yields no nodes', () {
      expect(parseInlines(''), isEmpty);
    });

    test('unicode text and markup', () {
      expect(parseInlines('héllo *wörld* 🎉'), [
        text('héllo '),
        StrongNode([text('wörld')]),
        text(' 🎉')
      ]);
      expect(parseInlines('**🎉**'), [
        StrongNode([text('🎉')])
      ]);
    });

    test('list items inline-parse their content', () {
      final list = parse('* *bold* x\n').children[0] as ListNode;
      final item = list.children[0] as ListItemNode;
      final p = item.children[0] as ParagraphNode;
      expect(p.children, [
        StrongNode([text('bold')]),
        text(' x')
      ]);
    });
  });
}
