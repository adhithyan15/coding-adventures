/// Inline content parser: phase 2 of AsciiDoc parsing.
/// Raw inline strings become [InlineNode] trees.
/// Priority order (longer markers win over shorter ones):
///  1. Two trailing spaces + `\n` → [HardBreakNode]
///  2. `\` + `\n` → [HardBreakNode]
///  3. `\n` → [SoftBreakNode]
///  4. `` ` `` → [CodeSpanNode] (verbatim)
///  5. `**...**` → [StrongNode] (unconstrained)
///  6. `__...__` → [EmphasisNode] (unconstrained)
///  7. `*...*` → [StrongNode] (constrained; AsciiDoc `*` is strong!)
///  8. `_..._` → [EmphasisNode] (constrained)
///  9. `link:url[text]` → [LinkNode]
/// 10. `image:url[alt]` → [ImageNode]
/// 11. `<<anchor,text>>` → [LinkNode]
/// 12. `https://` / `http://` → [LinkNode] (with `[text]`) or [AutolinkNode]
/// 13. Default → text buffer.
/// The scanner works over UTF-8 bytes, exactly like the reference, so
/// every index, slice, and advance matches the Rust lane bit for bit.
/// Dart strings are always valid Unicode, so the reference's
/// defensive invalid-UTF-8 fallback is unreachable by construction and
/// is omitted (no dead branches).
library;

import 'dart:convert';

import 'package:coding_adventures_document_ast/document_ast.dart';

/// Parses raw inline AsciiDoc into [InlineNode]s.
List<InlineNode> parseInlines(String raw) {
  if (raw.isEmpty) {
    return [];
  }
  final src = utf8.encode(raw);
  var pos = 0;
  final nodes = <InlineNode>[];
  final textBuf = StringBuffer();

  void flushText() {
    if (textBuf.isNotEmpty) {
      nodes.add(TextNode(textBuf.toString()));
      textBuf.clear();
    }
  }

  String slice(int start, int end) => utf8.decode(src.sublist(start, end));

  while (pos < src.length) {
    // Hard break: two trailing spaces before \n (strict bound: the
    // newline must exist, so trailing spaces at EOF stay literal).
    if (src[pos] == 0x20 &&
        pos + 2 < src.length &&
        src[pos + 1] == 0x20 &&
        src[pos + 2] == 0x0A) {
      flushText();
      nodes.add(const HardBreakNode());
      pos += 3;
      continue;
    }

    // Hard break: backslash before \n.
    if (src[pos] == 0x5C && pos + 1 < src.length && src[pos + 1] == 0x0A) {
      flushText();
      nodes.add(const HardBreakNode());
      pos += 2;
      continue;
    }

    // Soft break: plain newline.
    if (src[pos] == 0x0A) {
      flushText();
      nodes.add(const SoftBreakNode());
      pos += 1;
      continue;
    }

    // Code span: `...` (verbatim).
    if (src[pos] == 0x60) {
      final hit = tryCodeSpan(src, pos);
      if (hit != null) {
        flushText();
        nodes.add(hit.node);
        pos += hit.advance;
        continue;
      }
    }

    // Strong unconstrained: **...**
    if (pos + 1 < src.length && src[pos] == 0x2A && src[pos + 1] == 0x2A) {
      final hit = tryMarker(src, pos, _starStar, (inner) => StrongNode(inner));
      if (hit != null) {
        flushText();
        nodes.add(hit.node);
        pos += hit.advance;
        continue;
      }
    }

    // Emphasis unconstrained: __...__
    if (pos + 1 < src.length && src[pos] == 0x5F && src[pos + 1] == 0x5F) {
      final hit =
          tryMarker(src, pos, _underUnder, (inner) => EmphasisNode(inner));
      if (hit != null) {
        flushText();
        nodes.add(hit.node);
        pos += hit.advance;
        continue;
      }
    }

    // Strong constrained: *...* (AsciiDoc: * is strong!)
    if (src[pos] == 0x2A) {
      final hit = tryConstrained(src, pos, 0x2A, (inner) => StrongNode(inner));
      if (hit != null) {
        flushText();
        nodes.add(hit.node);
        pos += hit.advance;
        continue;
      }
    }

    // Emphasis constrained: _..._
    if (src[pos] == 0x5F) {
      final hit =
          tryConstrained(src, pos, 0x5F, (inner) => EmphasisNode(inner));
      if (hit != null) {
        flushText();
        nodes.add(hit.node);
        pos += hit.advance;
        continue;
      }
    }

    // link:url[text]
    if (_matchesAt(src, pos, _linkPrefix)) {
      final hit = tryLinkMacro(src, pos);
      if (hit != null) {
        flushText();
        nodes.add(hit.node);
        pos += hit.advance;
        continue;
      }
    }

    // image:url[alt]
    if (_matchesAt(src, pos, _imagePrefix)) {
      final hit = tryImageMacro(src, pos);
      if (hit != null) {
        flushText();
        nodes.add(hit.node);
        pos += hit.advance;
        continue;
      }
    }

    // Cross-reference: <<anchor,text>> or <<anchor>>.
    if (pos + 1 < src.length && src[pos] == 0x3C && src[pos + 1] == 0x3C) {
      final hit = tryCrossRef(src, pos);
      if (hit != null) {
        flushText();
        nodes.add(hit.node);
        pos += hit.advance;
        continue;
      }
    }

    // URL with optional [text] suffix.
    if (_matchesAt(src, pos, _httpsPrefix) ||
        _matchesAt(src, pos, _httpPrefix)) {
      final hit = tryUrl(src, pos);
      if (hit != null) {
        flushText();
        nodes.add(hit.node);
        pos += hit.advance;
        continue;
      }
    }

    // Default: consume one UTF-8 character into the text buffer.
    final chLen = utf8CharLen(src[pos]);
    textBuf.write(slice(pos, pos + chLen));
    pos += chLen;
  }

  flushText();
  return nodes;
}

/// A successful inline match: the node plus bytes to advance.
class InlineHit {
  const InlineHit(this.node, this.advance);
  final InlineNode node;
  final int advance;
}

final _starStar = utf8.encode('**');
final _underUnder = utf8.encode('__');
final _linkPrefix = utf8.encode('link:');
final _imagePrefix = utf8.encode('image:');
final _httpsPrefix = utf8.encode('https://');
final _httpPrefix = utf8.encode('http://');
final _crossClose = utf8.encode('>>');

bool _matchesAt(List<int> src, int pos, List<int> prefix) {
  if (pos + prefix.length > src.length) return false;
  for (var i = 0; i < prefix.length; i++) {
    if (src[pos + i] != prefix[i]) return false;
  }
  return true;
}

/// Byte index of the first [needle] at or after [start], else -1.
int _indexOf(List<int> src, int start, List<int> needle) {
  outer:
  for (var i = start; i + needle.length <= src.length; i++) {
    for (var j = 0; j < needle.length; j++) {
      if (src[i + j] != needle[j]) continue outer;
    }
    return i;
  }
  return -1;
}

/// Byte index of the first [byte] at or after [start], else -1.
int _indexOfByte(List<int> src, int start, int byte) {
  for (var i = start; i < src.length; i++) {
    if (src[i] == byte) return i;
  }
  return -1;
}

String _slice(List<int> src, int start, int end) =>
    utf8.decode(src.sublist(start, end));

/// Tries a code span at [pos]: N backticks, verbatim content, the same
/// N backticks. A single space on BOTH sides is stripped (only then).
InlineHit? tryCodeSpan(List<int> src, int pos) {
  if (src[pos] != 0x60) return null;
  var numTicks = 0;
  while (pos + numTicks < src.length && src[pos + numTicks] == 0x60) {
    numTicks++;
  }
  final marker = List.filled(numTicks, 0x60);
  final found = _indexOf(src, pos + numTicks, marker);
  if (found == -1) return null;
  final idx = found - pos - numTicks;
  var start = pos + numTicks;
  var end = found;
  if (end - start >= 2 && src[start] == 0x20 && src[end - 1] == 0x20) {
    start++;
    end--;
  }
  final advance = numTicks + idx + numTicks;
  return InlineHit(CodeSpanNode(_slice(src, start, end)), advance);
}

/// Tries an unconstrained `**` / `__` span at [pos].
InlineHit? tryMarker(List<int> src, int pos, List<int> marker,
    InlineNode Function(List<InlineNode>) wrap) {
  final mlen = marker.length;
  if (pos + mlen > src.length) return null;
  if (!_matchesAt(src, pos, marker)) return null;
  final found = _indexOf(src, pos + mlen, marker);
  if (found == -1) return null;
  final idx = found - pos - mlen;
  final inner = parseInlines(_slice(src, pos + mlen, found));
  return InlineHit(wrap(inner), mlen + idx + mlen);
}

/// Tries a constrained single-char span at [pos]: the opener must not
/// be doubled, boundaries must be non-word chars (or string edges),
/// and the content must be non-empty.
InlineHit? tryConstrained(List<int> src, int pos, int ch,
    InlineNode Function(List<InlineNode>) wrap) {
  if (src[pos] != ch) return null;
  if (pos + 1 < src.length && src[pos + 1] == ch) return null;
  if (pos > 0 && isWordChar(src[pos - 1])) return null;
  var end = pos + 1;
  while (end < src.length) {
    if (src[end] == ch) {
      if (end + 1 < src.length && src[end + 1] == ch) {
        end++;
        continue;
      }
      if (end + 1 < src.length && isWordChar(src[end + 1])) {
        end++;
        continue;
      }
      if (end == pos + 1) return null;
      final inner = parseInlines(_slice(src, pos + 1, end));
      return InlineHit(wrap(inner), end - pos + 1);
    }
    end++;
  }
  return null;
}

/// Tries `link:url[text]` at [pos]. The URL runs to the first `[`
/// (even across spaces); an empty label reuses the URL as text.
InlineHit? tryLinkMacro(List<int> src, int pos) {
  if (!_matchesAt(src, pos, _linkPrefix)) return null;
  final after = pos + 5;
  final bracket = _indexOfByte(src, after, 0x5B);
  if (bracket == -1) return null;
  if (bracket == after) return null;
  final url = _slice(src, after, bracket);
  final close = _indexOfByte(src, bracket + 1, 0x5D);
  if (close == -1) return null;
  final label = _slice(src, bracket + 1, close);
  final advance = 5 + (bracket - after) + 1 + (close - bracket - 1) + 1;
  final children = label.isEmpty ? [TextNode(url)] : parseInlines(label);
  return InlineHit(
      LinkNode(destination: url, title: null, children: children), advance);
}

/// Tries `image:url[alt]` at [pos].
InlineHit? tryImageMacro(List<int> src, int pos) {
  if (!_matchesAt(src, pos, _imagePrefix)) return null;
  final after = pos + 6;
  final bracket = _indexOfByte(src, after, 0x5B);
  if (bracket == -1) return null;
  if (bracket == after) return null;
  final url = _slice(src, after, bracket);
  final close = _indexOfByte(src, bracket + 1, 0x5D);
  if (close == -1) return null;
  final alt = _slice(src, bracket + 1, close);
  final advance = 6 + (bracket - after) + 1 + (close - bracket - 1) + 1;
  return InlineHit(ImageNode(destination: url, title: null, alt: alt), advance);
}

/// Tries `<<anchor,text>>` or `<<anchor>>` at [pos].
InlineHit? tryCrossRef(List<int> src, int pos) {
  if (!_matchesAt(src, pos, [0x3C, 0x3C])) return null;
  final after = pos + 2;
  final found = _indexOf(src, after, _crossClose);
  if (found == -1) return null;
  final inner = _slice(src, after, found);
  final advance = 2 + (found - after) + 2;
  final comma = inner.indexOf(',');
  final anchor = '#${(comma == -1 ? inner : inner.substring(0, comma)).trim()}';
  final label = (comma == -1 ? inner : inner.substring(comma + 1)).trim();
  return InlineHit(
      LinkNode(destination: anchor, title: null, children: [TextNode(label)]),
      advance);
}

/// Tries an `http(s)://` URL at [pos]. The URL ends at a space,
/// newline, or `[` (nothing else — tabs and carriage returns are part
/// of the URL, exactly like the reference). A `[text]` suffix makes a
/// link; otherwise the bare URL becomes an autolink.
InlineHit? tryUrl(List<int> src, int pos) {
  var urlEnd = src.length;
  for (var i = pos; i < src.length; i++) {
    if (src[i] == 0x20 || src[i] == 0x0A || src[i] == 0x5B) {
      urlEnd = i;
      break;
    }
  }
  if (urlEnd == pos) return null;
  final url = _slice(src, pos, urlEnd);
  if (urlEnd < src.length && src[urlEnd] == 0x5B) {
    final close = _indexOfByte(src, urlEnd + 1, 0x5D);
    if (close != -1) {
      final label = _slice(src, urlEnd + 1, close);
      final advance = (urlEnd - pos) + 1 + (close - urlEnd - 1) + 1;
      final children = label.isEmpty ? [TextNode(url)] : parseInlines(label);
      return InlineHit(
          LinkNode(destination: url, title: null, children: children), advance);
    }
  }
  return InlineHit(
      AutolinkNode(destination: url, isEmail: false), urlEnd - pos);
}

/// ASCII letters, digits, and underscore.
bool isWordChar(int byte) =>
    (byte >= 0x30 && byte <= 0x39) ||
    (byte >= 0x41 && byte <= 0x5A) ||
    (byte >= 0x61 && byte <= 0x7A) ||
    byte == 0x5F;

/// Byte length of the UTF-8 character starting with [firstByte].
int utf8CharLen(int firstByte) {
  if (firstByte & 0xF0 == 0xF0) return 4;
  if (firstByte & 0xE0 == 0xE0) return 3;
  if (firstByte & 0xC0 == 0xC0) return 2;
  return 1;
}
