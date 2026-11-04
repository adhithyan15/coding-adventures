import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { compileTetrad, emitTetrad, parseTetrad, runTetrad, tokenizeTetrad } from "../src/index.js";

describe("tetrad-runtime", () => {
  it("parses", () => {
    assert.equal(tokenizeTetrad("fn inc(x) { return x + 1 } # c").some((t) => t.value === "fn"), true);
    assert.equal(parseTetrad("fn inc(x) { return x + 1 } let y = inc(2)").forms.length, 2);
  });
  it("compiles and runs", () => {
    assert.equal(compileTetrad("fn main() { let x = 40 return x + 2 }").getFunction("main")?.instructions.some((i) => i.op === "add"), true);
    assert.equal(runTetrad("fn main() { let x = 40 return x + 2 }"), 42);
    assert.equal(runTetrad("fn main() { return 250 + 10 }", true), 4);
  });
  it("emits artifacts", () => {
    assert.match(emitTetrad("fn main() { return 42 }", "jvm").body, /language=tetrad/);
  });
});
