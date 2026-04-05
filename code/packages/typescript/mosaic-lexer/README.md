# @coding-adventures/mosaic-lexer

Tokenizes `.mosaic` source files using the grammar-driven lexer.

## What Is Mosaic?

Mosaic is a UI component description language. A `.mosaic` file declares one
component with typed data slots and a visual node tree. There is no imperative
logic — Mosaic compiles forward-only to target platforms (React, Web Components,
SwiftUI, Compose, and more).

## Position in the Compiler Pipeline

```
mosaic source text
       │
       ▼
  mosaic-lexer   ◄── this package
       │
       ▼
  mosaic-parser
       │
       ▼
  mosaic-analyzer
       │
       ▼
    mosaic-vm
    /        \
emit-react  emit-webcomponent
```

## Usage

```typescript
import { tokenizeMosaic } from "@coding-adventures/mosaic-lexer";

const tokens = tokenizeMosaic(`
  component Button {
    slot label: text;
    slot disabled: bool = false;

    Row {
      Text { content: @label; }
    }
  }
`);

console.log(tokens[0]); // { type: "KEYWORD", value: "component", line: 2, column: 3 }
```

## Token Types

| Token      | Example        | Description                                      |
|------------|----------------|--------------------------------------------------|
| `KEYWORD`  | `component`    | Reserved word (see list below)                   |
| `IDENT`    | `Button`       | Component or property name; allows hyphens       |
| `STRING`   | `"./btn.msc"`  | Double-quoted string literal                     |
| `NUMBER`   | `42`, `-3.14`  | Integer or decimal                               |
| `DIMENSION`| `16dp`, `100%` | Number with unit suffix                          |
| `COLOR_HEX`| `#2563eb`      | Hex color: `#rgb`, `#rrggbb`, `#rrggbbaa`        |
| `LBRACE`   | `{`            | Open block                                       |
| `RBRACE`   | `}`            | Close block                                      |
| `LANGLE`   | `<`            | Open generic type bracket                        |
| `RANGLE`   | `>`            | Close generic type bracket                       |
| `COLON`    | `:`            | Type annotation separator                        |
| `SEMICOLON`| `;`            | Statement terminator                             |
| `COMMA`    | `,`            | Separator                                        |
| `DOT`      | `.`            | Enum namespace separator (e.g., `align.center`)  |
| `EQUALS`   | `=`            | Default value assignment                         |
| `AT`       | `@`            | Slot reference sigil (e.g., `@title`)            |
| `EOF`      | *(synthetic)*  | End of input                                     |

Keywords: `component slot import from as text number bool image color node list true false when each`

## Dependencies

- `@coding-adventures/grammar-tools` — TokenGrammar type and grammar parser
- `@coding-adventures/lexer` — generic `grammarTokenize` engine
- `@coding-adventures/directed-graph` — used transitively by lexer

## Development

```bash
bash BUILD
```
