/// AsciiDoc parser — converts AsciiDoc text to a Document AST.
/// Spec: TE03 — AsciiDoc Parser.
/// Two phases: block structure ([parseBlocks]), then inline content.
/// `*bold*` is strong (NOT emphasis — the opposite of Markdown).
library;

import 'package:coding_adventures_document_ast/document_ast.dart';

import 'src/block_parser.dart';

export 'package:coding_adventures_document_ast/document_ast.dart';

/// Parses AsciiDoc [text] into a Document AST [DocumentNode].
/// Lenient: unclosed blocks run to EOF, unclosed markup stays literal.
DocumentNode parse(String text) {
  return DocumentNode(parseBlocks(text));
}
