import { describe, expect, it } from "vitest";
import { compileToIir, executeOnLangVm, TranslationError } from "../src/index.js";

describe("Brainfuck LANG VM integration", () => {
  it("compiles to LANG VM IR", () => {
    const mod = compileToIir("++.");
    expect(mod.language).toBe("brainfuck");
    expect(mod.getFunction("main")?.instructions.some((i) => i.op === "io_out")).toBe(true);
  });
  it("executes IO and loops", () => {
    expect(executeOnLangVm("+++++.").output).toBe(String.fromCharCode(5));
    expect(executeOnLangVm(",.", "A").output).toBe("A");
    const result = executeOnLangVm("++[>++<-]>.");
    expect(result.output).toBe(String.fromCharCode(4));
    expect(result.memory.get(1)).toBe(4);
    expect(result.vm.loopIterations("main", "loop_0_start")).toBe(2);
  });
  it("reports unmatched brackets", () => {
    expect(() => compileToIir("[")).toThrow(TranslationError);
    expect(() => compileToIir("]")).toThrow(TranslationError);
  });
});
