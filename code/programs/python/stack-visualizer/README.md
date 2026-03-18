# Stack Visualizer

Terminal UI that visually walks through every stage of the computing stack:
source code, tokens, AST, bytecode, and execution.

Built with [Textual](https://textual.textualize.io/).

## Interface Layout

```
+-------------------+-------------------+-------------------+
|                   |                   |                   |
|      Source       |      Tokens       |       AST         |
|                   |                   |                   |
|                   |                   |                   |
+-------------------+-------------------+-------------------+
|                   |                                       |
|     Bytecode      |            Execution                  |
|                   |                                       |
|                   |                                       |
+-------------------+---------------------------------------+
|  [ Step ]  [ Run ]  [ Reset ]           Target: [v math] |
+-----------------------------------------------------------+
```

## Features

- **Step-through** — advance one pipeline stage at a time to see how
  source becomes tokens, tokens become an AST, and so on.
- **Run** — execute all stages in sequence automatically.
- **Reset** — clear the current run and start over.
- **Target switching** — choose which source program to visualize.

## Spec

Full specification: [specs/11-stack-visualizer.md](../../specs/11-stack-visualizer.md)
