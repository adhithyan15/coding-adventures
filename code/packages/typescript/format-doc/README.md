# @coding-adventures/format-doc

Backend-neutral document algebra for pretty-printers.

This package defines the semantic `Doc` IR plus a width-aware realization pass
that turns a `Doc` tree into a `DocLayoutTree`. It does not print strings on
its own. The first concrete output path is:

```text
Doc -> DocLayoutTree -> PaintScene -> paint-vm-ascii
```

## Pipeline

```text
AST printer
  -> Doc
  -> layoutDoc(...)
  -> DocLayoutTree
  -> backend bridge
```

## Usage

```ts
import {
  concat,
  group,
  hardline,
  indent,
  layoutDoc,
  line,
  softline,
  text,
} from "@coding-adventures/format-doc";

const doc = group(
  concat([
    text("items("),
    indent(
      concat([
        softline(),
        text("alpha,"),
        line(),
        text("beta"),
      ])
    ),
    softline(),
    text(")"),
    hardline(),
  ])
);

const layout = layoutDoc(doc, { printWidth: 10, indentWidth: 2 });
```

## What it exports

- `Doc` node types
- combinators like `text`, `concat`, `group`, `indent`, `line`, `softline`, `hardline`
- `layoutDoc(doc, options)` for width-aware realization
- `DocLayoutTree`, `DocLayoutLine`, and `DocLayoutSpan`

## Related packages

- `@coding-adventures/format-doc-to-paint` — convert `DocLayoutTree` to `PaintScene`
- future language formatter packages — compile AST nodes into `Doc`
