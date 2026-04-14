import { describe, expect, it } from "vitest";

import {
  GenericTypeChecker,
  type TypeCheckResult,
  type TypeChecker,
} from "../src/index.js";

interface SimpleNode {
  kind: string;
  value?: string;
}

class SuccessChecker implements TypeChecker<SimpleNode, SimpleNode> {
  check(ast: SimpleNode): TypeCheckResult<SimpleNode> {
    return {
      typedAst: { ...ast, value: ast.value ?? "ok" },
      errors: [],
      ok: true,
    };
  }
}

class RuleDrivenChecker extends GenericTypeChecker<SimpleNode> {
  constructor() {
    super();
    this.registerHook("node", "literal", (node) => {
      node.value = "checked";
    });
    this.registerHook("node", "broken", (node) => {
      this.error(`bad node: ${node.kind}`, node);
    });
  }

  protected run(ast: SimpleNode): void {
    this.dispatch("node", ast);
  }

  protected nodeKind(node: SimpleNode): string | null {
    return node.kind;
  }

  protected override locate(subject: unknown): [number, number] {
    void subject;
    return [7, 9];
  }
}

describe("type-checker-protocol", () => {
  it("supports simple protocol-shaped checkers", () => {
    const result = new SuccessChecker().check({ kind: "literal" });
    expect(result.ok).toBe(true);
    expect(result.errors).toEqual([]);
    expect(result.typedAst.value).toBe("ok");
  });

  it("dispatches full nodes through the generic checker hooks", () => {
    const result = new RuleDrivenChecker().check({ kind: "literal" });
    expect(result.ok).toBe(true);
    expect(result.typedAst.value).toBe("checked");
  });

  it("reports diagnostics through the shared error helper", () => {
    const result = new RuleDrivenChecker().check({ kind: "broken" });
    expect(result.ok).toBe(false);
    expect(result.errors).toEqual([
      { message: "bad node: broken", line: 7, column: 9 },
    ]);
  });

  it("ignores unhandled node kinds cleanly", () => {
    const result = new RuleDrivenChecker().check({ kind: "unknown", value: "x" });
    expect(result.ok).toBe(true);
    expect(result.typedAst.value).toBe("x");
  });
});
