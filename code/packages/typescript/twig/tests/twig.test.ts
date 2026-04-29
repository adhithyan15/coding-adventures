import { describe, expect, it } from "vitest";
import { compileTwig, emitTwig, formatTwigValue, parseTwig, runTwig, tokenizeTwig } from "../src/index.js";

describe("twig", () => {
  it("parses", () => {
    expect(tokenizeTwig("(+ 1 2) ; ignored\n#t nil")).toEqual(["(", "+", "1", "2", ")", "#t", "nil"]);
    expect(parseTwig("(+ 1 2) #f nil")).toHaveLength(3);
  });
  it("compiles and runs", () => {
    expect(compileTwig("(+ 1 2)").getFunction("main")?.instructions.some((i) => i.op === "call_builtin")).toBe(true);
    expect(runTwig("(+ 40 2)")).toEqual(["", 42]);
    expect(runTwig("(if #f 1 42)")).toEqual(["", 42]);
    expect(runTwig("(define answer 41) (define (inc x) (+ x 1)) (print (inc answer))")).toEqual(["42\n", null]);
  });
  it("handles lists and emits", () => {
    const [stdout, value] = runTwig("(let ((x 1) (y 2)) (begin (print (cons x y)) (pair? (cons x y))))");
    expect(stdout).toBe("(1 . 2)\n");
    expect(value).toBe(true);
    expect(formatTwigValue(["cons", 1, null])).toBe("(1 . nil)");
    expect(emitTwig("(+ 1 2)", "wasm").body).toContain("language=twig");
  });
});
