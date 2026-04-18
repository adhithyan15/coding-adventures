# @coding-adventures/format-doc-text

Plain-text backend for `@coding-adventures/format-doc`.

This package consumes the line/span layout produced by `layoutDoc()` and turns
it into a string. It is the first backend for the document algebra stack.

## Usage

```ts
import { concat, group, indent, line, softline, text } from "@coding-adventures/format-doc";
import { renderDocToText } from "@coding-adventures/format-doc-text";

const doc = group(
  concat([
    text("foo("),
    indent(concat([softline(), text("bar,"), line(), text("baz")])),
    softline(),
    text(")"),
  ])
);

const out = renderDocToText(doc, { printWidth: 10, indentWidth: 2 });
// foo(
//   bar,
//   baz
// )
```

## API

- `renderLayoutToText(layout)` — serialize a realized `LayoutDocument`
- `renderDocToText(doc, options)` — convenience helper over `layoutDoc()`
