import { describe, expect, it } from "vitest";
import { compileTetrad, emitTetrad, parseTetrad, runTetrad, tokenizeTetrad } from "../src/index.js";

describe("tetrad-runtime", () => {
  it("parses", () => {
    expect(tokenizeTetrad("fn inc(x) { return x + 1 } # c").some((t) => t.value === "fn")).toBe(true);
    expect(parseTetrad("fn inc(x) { return x + 1 } let y = inc(2)").forms).toHaveLength(2);
  });
  it("compiles and runs", () => {
    expect(compileTetrad("fn main() { let x = 40 return x + 2 }").getFunction("main")?.instructions.some((i) => i.op === "add")).toBe(true);
    expect(runTetrad("fn main() { let x = 40 return x + 2 }")).toBe(42);
    expect(runTetrad("fn main() { return 250 + 10 }", true)).toBe(4);
  });
  it("emits artifacts", () => {
    expect(emitTetrad("fn main() { return 42 }", "jvm").body).toContain("language=tetrad");
  });
});
