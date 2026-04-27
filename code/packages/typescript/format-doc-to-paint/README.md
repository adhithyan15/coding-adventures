# @coding-adventures/format-doc-to-paint

Paint bridge for `@coding-adventures/format-doc`.

This package consumes the `DocLayoutTree` produced by `layoutDoc()` and turns
it into a `PaintScene`. The first concrete rendering path is then:

```text
Doc -> DocLayoutTree -> PaintScene -> paint-vm-ascii
```

## Usage

```ts
import { concat, group, indent, line, softline, text } from "@coding-adventures/format-doc";
import { docToPaintScene } from "@coding-adventures/format-doc-to-paint";
import { renderToAscii } from "@coding-adventures/paint-vm-ascii";

const doc = group(
  concat([
    text("foo("),
    indent(concat([softline(), text("bar,"), line(), text("baz")])),
    softline(),
    text(")"),
  ])
);

const scene = docToPaintScene(doc, { printWidth: 10, indentWidth: 2 });
const out = renderToAscii(scene, { scaleX: 1, scaleY: 1 });
```

## API

- `docLayoutToPaintScene(layout)` — convert a `DocLayoutTree` to `PaintScene`
- `docToPaintScene(doc, options)` — convenience helper over `layoutDoc()`
