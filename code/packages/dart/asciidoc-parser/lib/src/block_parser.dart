/// Block-level parser: phase 1 of AsciiDoc parsing.
/// Splits input into block structure; inline content stays raw for
/// phase 2 ([parseInlines]).
/// States: normal (dispatch), paragraph (accumulate), codeBlock,
/// literalBlock, passthrough, quoteBlock (content re-parsed).
/// List items accumulate flat and nest on flush; the level is the
/// count of leading `*` / `.` characters.
library;

import 'package:coding_adventures_document_ast/document_ast.dart';

import 'inline_parser.dart';

enum _State {
  normal,
  paragraph,
  codeBlock,
  literalBlock,
  passthrough,
  quoteBlock,
}

/// One buffered list item awaiting nesting.
class _ListItemAccum {
  _ListItemAccum(this.level, this.ordered, this.raw);
  final int level;
  final bool ordered;
  final String raw;
}

/// Converts AsciiDoc [text] into block nodes.
List<BlockNode> parseBlocks(String text) {
  final lines = splitLines(text);
  final result = <BlockNode>[];
  var state = _State.normal;
  final accum = <String>[];
  String? pendingLanguage;
  final listItems = <_ListItemAccum>[];

  void flushParagraph() {
    if (accum.isEmpty) return;
    final raw = accum.join('\n');
    result.add(ParagraphNode(parseInlines(raw)));
    accum.clear();
  }

  void flushList() {
    if (listItems.isEmpty) return;
    result.addAll(buildListNodes(listItems));
    listItems.clear();
  }

  for (final line in lines) {
    switch (state) {
      case _State.codeBlock:
        if (isDelim(line, 0x2D, 4)) {
          result.add(CodeBlockNode(
              language: pendingLanguage, value: _codeValue(accum)));
          pendingLanguage = null;
          accum.clear();
          state = _State.normal;
        } else {
          accum.add(line);
        }
      case _State.literalBlock:
        if (isDelim(line, 0x2E, 4)) {
          // A literal close never consumes the pending language: it
          // stays armed for a later code block, like the reference.
          result.add(CodeBlockNode(language: null, value: _codeValue(accum)));
          accum.clear();
          state = _State.normal;
        } else {
          accum.add(line);
        }
      case _State.passthrough:
        if (isDelim(line, 0x2B, 4)) {
          // Plain join: no trailing newline (pinned by the reference
          // test; the Python/Ruby lanes append one — known divergence).
          result.add(RawBlockNode(format: 'html', value: accum.join('\n')));
          accum.clear();
          state = _State.normal;
        } else {
          accum.add(line);
        }
      case _State.quoteBlock:
        if (isDelim(line, 0x5F, 4)) {
          final inner = accum.join('\n');
          result.add(BlockquoteNode(parseBlocks(inner)));
          accum.clear();
          state = _State.normal;
        } else {
          accum.add(line);
        }
      case _State.paragraph:
        final heading = parseHeadingLine(line);
        final listLine = parseListLine(line);
        if (line.trim().isEmpty) {
          flushParagraph();
          state = _State.normal;
        } else if (heading != null) {
          flushParagraph();
          result.add(HeadingNode(heading.level, parseInlines(heading.content)));
          state = _State.normal;
        } else if (listLine != null) {
          flushParagraph();
          listItems.add(_ListItemAccum(
              listLine.level, listLine.ordered, listLine.content));
          state = _State.normal;
        } else {
          // Delimiters, comments, and attribute lines do NOT interrupt
          // a paragraph — they accumulate as text, like the reference.
          accum.add(line);
        }
      case _State.normal:
        final trimmed = line.trim();
        if (trimmed.isEmpty) {
          flushList();
          continue;
        }
        // Single-line comment: // but not /// (checked untrimmed: an
        // indented // is paragraph text).
        if (line.startsWith('//') && !line.startsWith('///')) {
          continue;
        }
        // Block attribute: [source,lang] etc.
        if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
          flushList();
          final attr = trimmed.substring(1, trimmed.length - 1);
          if (attr.toLowerCase().startsWith('source')) {
            final comma = attr.indexOf(',');
            if (comma == -1) {
              pendingLanguage = null;
            } else {
              pendingLanguage = attr.substring(comma + 1).trim();
            }
          }
          continue;
        }
        final heading = parseHeadingLine(line);
        if (heading != null) {
          flushList();
          result.add(HeadingNode(heading.level, parseInlines(heading.content)));
          continue;
        }
        if (isThematicBreak(line)) {
          flushList();
          result.add(const ThematicBreakNode());
          continue;
        }
        if (isDelim(line, 0x2D, 4)) {
          flushList();
          accum.clear();
          state = _State.codeBlock;
          continue;
        }
        if (isDelim(line, 0x2E, 4)) {
          flushList();
          accum.clear();
          state = _State.literalBlock;
          continue;
        }
        if (isDelim(line, 0x2B, 4)) {
          flushList();
          accum.clear();
          state = _State.passthrough;
          continue;
        }
        if (isDelim(line, 0x5F, 4)) {
          flushList();
          accum.clear();
          state = _State.quoteBlock;
          continue;
        }
        final listLine = parseListLine(line);
        if (listLine != null) {
          // A type change flushes; the group key is the FIRST item.
          if (listItems.isNotEmpty &&
              listItems[0].ordered != listLine.ordered) {
            flushList();
          }
          listItems.add(_ListItemAccum(
              listLine.level, listLine.ordered, listLine.content));
          continue;
        }
        // List continuation marker (exact, untrimmed).
        if (line == '+' && listItems.isNotEmpty) {
          continue;
        }
        flushList();
        accum.add(line);
        state = _State.paragraph;
    }
  }

  // Lenient EOF: unclosed blocks emit whatever accumulated — even when
  // empty (the spec's literal reading; Python/Ruby drop empties).
  switch (state) {
    case _State.paragraph:
      flushParagraph();
    case _State.codeBlock:
      result.add(
          CodeBlockNode(language: pendingLanguage, value: _codeValue(accum)));
    case _State.literalBlock:
      result.add(CodeBlockNode(language: null, value: _codeValue(accum)));
    case _State.passthrough:
      result.add(RawBlockNode(format: 'html', value: accum.join('\n')));
    case _State.quoteBlock:
      result.add(BlockquoteNode(parseBlocks(accum.join('\n'))));
    case _State.normal:
      break;
  }
  flushList();

  return result;
}

/// Code/literal value: lines joined, trailing newline ensured (empty
/// stays empty).
String _codeValue(List<String> accum) {
  if (accum.isEmpty) return '';
  final v = accum.join('\n');
  return v.endsWith('\n') ? v : '$v\n';
}

/// Splits on `\n`; one trailing newline leaves no extra empty line.
/// Carriage returns are NOT stripped (block content keeps them, like
/// the reference — only blank/heading/list classification trims).
List<String> splitLines(String text) {
  if (text.isEmpty) return [];
  final lines = text.split('\n');
  if (lines.last.isEmpty) lines.removeLast();
  return lines;
}

/// Whether [line] is [ch] repeated at least [minLen] times (trailing
/// whitespace ignored; leading whitespace disqualifies).
bool isDelim(String line, int ch, int minLen) {
  final trimmed = line.trimRight();
  if (trimmed.length < minLen) return false;
  for (var i = 0; i < trimmed.length; i++) {
    if (trimmed.codeUnitAt(i) != ch) return false;
  }
  return true;
}

/// Whether [line] is `'''` (three or more single quotes).
bool isThematicBreak(String line) => isDelim(line, 0x27, 3);

/// A parsed heading line: level 1–6 plus trimmed content.
class HeadingHit {
  const HeadingHit(this.level, this.content);
  final int level;
  final String content;
}

/// Parses `= text` … `====== text`: up to six `=`, then exactly one
/// space. Seven or more `=` is NOT a heading (no space follows the
/// sixth) — it stays paragraph text, like the reference. The spec's
/// "clamp to 6" is what Python does instead (known divergence).
HeadingHit? parseHeadingLine(String line) {
  var count = 0;
  while (count < 6 && count < line.length && line.codeUnitAt(count) == 0x3D) {
    count++;
  }
  if (count == 0) return null;
  if (count < line.length && line.codeUnitAt(count) == 0x20) {
    return HeadingHit(count, line.substring(count + 1).trim());
  }
  return null;
}

/// A parsed list line: nesting level, ordering, trimmed content.
class ListHit {
  const ListHit(this.level, this.ordered, this.content);
  final int level;
  final bool ordered;
  final String content;
}

/// Parses `* text` / `** text` … (unordered) and `. text` / `.. text`
/// … (ordered). The level is the marker count, uncapped.
ListHit? parseListLine(String line) {
  if (line.isEmpty) return null;
  final marker = line.codeUnitAt(0);
  if (marker != 0x2A && marker != 0x2E) return null;
  var count = 0;
  while (count < line.length && line.codeUnitAt(count) == marker) {
    count++;
  }
  if (count < line.length && line.codeUnitAt(count) == 0x20) {
    return ListHit(count, marker == 0x2E, line.substring(count + 1).trim());
  }
  return null;
}

/// Nests flat [items] into list blocks (at most one: the buffer holds
/// a single ordered/unordered run).
List<BlockNode> buildListNodes(List<_ListItemAccum> items) {
  if (items.isEmpty) return [];
  final list = buildNestedList(items, items[0].ordered, 1);
  return list == null ? [] : [list];
}

/// Builds the [ListNode] for [level]: each level item becomes a
/// [ListItemNode] holding an inline-parsed paragraph plus any deeper
/// run recursed as a nested list. A run that SKIPS a level (1 → 3)
/// vanishes — the deep call finds nothing at level 2 — exactly like
/// the reference.
ListNode? buildNestedList(List<_ListItemAccum> items, bool ordered, int level) {
  final children = <ListChildNode>[];
  var i = 0;
  while (i < items.length) {
    final item = items[i];
    if (item.level < level) break;
    if (item.level == level) {
      final liChildren = <BlockNode>[
        ParagraphNode(parseInlines(item.raw)),
      ];
      final jStart = i + 1;
      var j = jStart;
      while (j < items.length && items[j].level > level) {
        j++;
      }
      if (j > jStart) {
        final nestedItems = items.sublist(jStart, j);
        final nested =
            buildNestedList(nestedItems, nestedItems[0].ordered, level + 1);
        if (nested != null) liChildren.add(nested);
      }
      children.add(ListItemNode(liChildren));
      i = j;
    } else {
      // Deeper than level: already consumed by a nested call.
      i++;
    }
  }
  if (children.isEmpty) return null;
  return ListNode(ordered: ordered, start: 1, tight: true, children: children);
}
