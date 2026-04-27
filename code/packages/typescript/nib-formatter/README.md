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

Comments and blank lines now flow through the source-based path too:

```ts
const formattedWithComments = formatNib(
  "// header\nfn main(){let x:u4=5; // keep\n// done\n}",
);
```

## Exports

- `printNibDoc(ast)` — lower a parsed Nib AST into `Doc`
- `printNibSourceToDoc(source)` — parse source and lower it to `Doc`
- `formatNibAst(ast, options)` — run the full `Doc -> paint -> ASCII` pipeline
- `formatNib(source, options)` — parse and format source in one call

## Notes

- v1 canonicalizes syntax and spacing for the full Nib grammar
- source-based entry points preserve line comments and blank lines using the
  lexer/parser `preserveSourceInfo` path
- AST-only entry points preserve node-attached trivia, but cannot recover EOF
  comments unless the caller carries the token stream separately
- formatted output has no trailing newline and no trailing whitespace
- the execution path remains `Doc -> LayoutTree -> PaintScene -> paint-vm-ascii`
