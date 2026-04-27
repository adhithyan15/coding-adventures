# document-ast (Java)

Universal document IR (intermediate representation) types for the coding-adventures project.

## What it does

Defines the Document AST type hierarchy — the format-agnostic IR for structured documents. Block nodes form the structural skeleton; inline nodes carry prose content.

### Block nodes
DocumentNode, HeadingNode, ParagraphNode, CodeBlockNode, BlockquoteNode, ListNode, ListItemNode, TaskItemNode, ThematicBreakNode, RawBlockNode, TableNode, TableRowNode, TableCellNode

### Inline nodes
TextNode, EmphasisNode, StrongNode, StrikethroughNode, CodeSpanNode, LinkNode, ImageNode, AutolinkNode, RawInlineNode, HardBreakNode, SoftBreakNode

## Usage

```java
import com.codingadventures.documentast.*;

var doc = new BlockNode.DocumentNode(List.of(
    new BlockNode.HeadingNode(1, List.of(new InlineNode.TextNode("Hello"))),
    new BlockNode.ParagraphNode(List.of(new InlineNode.TextNode("World")))
));
```

Uses Java sealed interfaces for exhaustive pattern matching.

## Layer

TE00 (text/language layer — document IR). Leaf package, zero dependencies.
