# @coding-adventures/nib-formatter

Format Nib source into a stable canonical layout using the shared document
algebra stack.

This package is the first language-specific formatter built on top of:

- `@coding-adventures/format-doc`
- `@coding-adventures/format-doc-std`
- `@coding-adventures/format-doc-to-paint`
- `@coding-adventures/paint-vm-ascii`

The full execution path is:

```text
Nib source
  -> nib-parser AST
  -> Doc
  -> DocLayoutTree
  -> PaintScene
  -> paint-vm-ascii
  -> formatted text
```

## Usage

```ts
import { formatNib, printNibSourceToDoc } from "@coding-adventures/nib-formatter";

const ugly = "fn main(){let total:u8=sum(very_long_name,another_name,third_name);}";

const doc = printNibSourceToDoc(ugly);
const formatted = formatNib(ugly, { printWidth: 24 });
```

## Exports

- `printNibDoc(ast)` — lower a parsed Nib AST into `Doc`
- `printNibSourceToDoc(source)` — parse source and lower it to `Doc`
- `formatNibAst(ast, options)` — run the full `Doc -> paint -> ASCII` pipeline
- `formatNib(source, options)` — parse and format source in one call

## Notes

- v1 canonicalizes syntax and spacing for the full Nib grammar
- v1 does not preserve comments because the current Nib lexer removes them
- formatted output has no trailing newline and no trailing whitespace
