import { describe, expect, it } from "vitest";
import { compileDartmouthBasic, emitDartmouthBasic, parseBasicLines, runDartmouthBasic, tokenizeBasicExpr } from "../src/index.js";

describe("dartmouth-basic-ir-compiler", () => {
  it("parses lines and expressions", () => {
    expect(parseBasicLines("20 PRINT A\n10 LET A = 5").map((l) => l.number)).toEqual([10, 20]);
    expect(tokenizeBasicExpr("A1 + 2 * (B - 1)")).toEqual(["A1", "+", "2", "*", "(", "B", "-", "1", ")"]);
  });
  it("compiles and runs", () => {
    const result = compileDartmouthBasic("10 LET A = 5\n20 PRINT A\n30 END");
    expect(result.module.language).toBe("dartmouth-basic");
    expect(result.varNames).toEqual(["A"]);
    expect(runDartmouthBasic('10 A = 40\n20 PRINT "READY"\n30 PRINT A + 2\n40 END')).toBe("READY\n42\n");
  });
  it("runs control flow and emits", () => {
    const source = ["10 FOR I = 1 TO 3", "20 PRINT I", "30 NEXT I", "40 IF I = 4 THEN 60", "50 GOTO 70", "60 PRINT 99", "70 END"].join("\n");
    expect(runDartmouthBasic(source)).toBe("1\n2\n3\n99\n");
    expect(emitDartmouthBasic("10 PRINT 1\n20 END", "clr").body).toContain("language=dartmouth-basic");
  });
});
