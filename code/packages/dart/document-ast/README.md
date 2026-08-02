# Document AST (Dart)

An immutable Dart implementation of the format-neutral document intermediate
representation defined by [TE00](../../../specs/TE00-document-ast.md).

The package exposes a sealed `Node` hierarchy with stable snake_case `type`
discriminators. It includes the universal block and inline nodes plus the
stabilized task-list, table, and strikethrough model shared by the established
implementation lanes. Child lists are defensively copied and unmodifiable,
heading levels are validated, and node equality is structural.

```dart
import 'package:coding_adventures_document_ast/document_ast.dart';

final document = DocumentNode([
  HeadingNode(1, [const TextNode('Document AST')]),
  ParagraphNode([
    const TextNode('Hello '),
    EmphasisNode([const TextNode('world')]),
  ]),
]);

print(document.type); // document
```

This package intentionally contains only data types. Parsing, rendering, I/O,
and format-specific behavior belong in downstream packages.

Run the package checks with:

```sh
dart pub get
dart analyze
dart test
```
