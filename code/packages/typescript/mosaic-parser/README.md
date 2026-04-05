# @coding-adventures/mosaic-parser

Parses `.mosaic` source text into an ASTNode tree using the grammar-driven approach.

## Position in the Compiler Pipeline

```
mosaic source text
       │
       ▼
  mosaic-lexer
       │
       ▼
  mosaic-parser  ◄── this package
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
import { parseMosaic } from "@coding-adventures/mosaic-parser";
import type { ASTNode } from "@coding-adventures/mosaic-parser";

const ast = parseMosaic(`
  component Button {
    slot label: text;
    slot disabled: bool = false;

    Row {
      Text { content: @label; }
    }
  }
`);

console.log(ast.ruleName); // "file"
```

## AST Structure

The root is an `ASTNode` with `ruleName: "file"`. Each node has:
- `ruleName` — the grammar rule that produced this node
- `children` — array of child `ASTNode`s or leaf `Token`s

The 18 grammar rules and their `ruleName` values:

| ruleName              | What it represents                              |
|-----------------------|-------------------------------------------------|
| `file`                | Top-level: imports + component                  |
| `import_decl`         | `import X from "..."` or `import X as Y from …` |
| `component_decl`      | `component Name { ... }`                        |
| `slot_decl`           | `slot name: type [= default];`                  |
| `slot_type`           | Primitive keyword, component name, or list type |
| `list_type`           | `list<slot_type>`                               |
| `default_value`       | Literal default for a slot                      |
| `node_tree`           | Root node element                               |
| `node_element`        | `Name { ... }`                                  |
| `node_content`        | One item inside a node                          |
| `property_assignment` | `name: value;`                                  |
| `property_value`      | RHS of a property assignment                    |
| `slot_ref`            | `@slotName`                                     |
| `enum_value`          | `namespace.member`                              |
| `child_node`          | Nested node element                             |
| `slot_reference`      | `@slotName;` (as a child, not property value)   |
| `when_block`          | `when @bool { ... }`                            |
| `each_block`          | `each @list as item { ... }`                    |

## Dependencies

- `@coding-adventures/mosaic-lexer` — tokenizes source into a token stream
- `@coding-adventures/grammar-tools` — ParserGrammar type
- `@coding-adventures/parser` — generic GrammarParser engine
- `@coding-adventures/state-machine` — used by parser engine
- `@coding-adventures/lexer` — used transitively by mosaic-lexer
- `@coding-adventures/directed-graph` — used transitively

## Development

```bash
bash BUILD
```
