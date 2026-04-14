# @coding-adventures/ir-optimizer

Generic IR optimizer pipeline for compiler packages.

It currently ports the Python optimizer stack:

- `DeadCodeEliminator`
- `ConstantFolder`
- `PeepholeOptimizer`

```ts
import { IrOptimizer } from "@coding-adventures/ir-optimizer";

const result = IrOptimizer.defaultPasses().optimize(program);
console.log(result.instructionsEliminated);
```
