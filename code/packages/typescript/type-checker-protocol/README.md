# @coding-adventures/type-checker-protocol

Shared type-checker contract and generic checker framework for compiler
toolchains written in TypeScript.

This package gives TypeScript-hosted compilers in the repo a common semantic
analysis shape:

- `TypeChecker<ASTIn, ASTOut>` for the public contract
- `TypeCheckResult<AST>` and `TypeErrorDiagnostic` for result reporting
- `GenericTypeChecker<AST>` for reusable AST-node dispatch, diagnostics, and
  hook registration

## Example

```ts
import {
  GenericTypeChecker,
  type TypeCheckResult,
} from "@coding-adventures/type-checker-protocol";

interface ToyNode {
  kind: string;
  text: string;
}

class ToyChecker extends GenericTypeChecker<ToyNode> {
  constructor() {
    super();
    this.registerHook("node", "literal", (node) => {
      node.text = node.text.trim();
    });
  }

  protected run(ast: ToyNode): void {
    this.dispatch("node", ast);
  }

  protected nodeKind(node: ToyNode): string | null {
    return node.kind;
  }
}

const result: TypeCheckResult<ToyNode> = new ToyChecker().check({
  kind: "literal",
  text: "  42  ",
});
```
