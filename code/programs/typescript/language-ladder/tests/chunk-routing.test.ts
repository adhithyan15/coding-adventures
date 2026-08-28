import { describe, expect, it } from "vitest";

import { isHandwritingToolsModuleId } from "../chunk-routing";

describe("handwriting-tools chunk boundary", () => {
  it.each([
    "/repo/code/packages/typescript/script-ductus/src/strokes.ts",
    "/repo/code/packages/typescript/script-ductus/src/strokes/tamil.ts",
    "/repo/code/packages/typescript/script-ductus/src/strokes/tamil/U-B85.ts",
    "/repo/code/packages/typescript/script-ductus/src/strokes/tamil/vowels/U-B85.ts",
    "C:\\repo\\code\\packages\\typescript\\script-ductus\\src\\strokes\\tamil\\U-B85.ts",
  ])(
    "keeps facade and arbitrarily nested stroke owners lazy: %s",
    (moduleId) => {
      expect(isHandwritingToolsModuleId(moduleId)).toBe(true);
    },
  );

  it.each([
    "/repo/code/packages/typescript/script-ductus/src/ductusview.ts",
    "/repo/code/packages/typescript/script-ductus/src/truetype.ts?worker",
  ])("keeps the other handwriting-only modules lazy: %s", (moduleId) => {
    expect(isHandwritingToolsModuleId(moduleId)).toBe(true);
  });

  it.each([
    "/repo/code/packages/typescript/script-ductus/src/scriptdata.ts",
    "/repo/code/packages/typescript/script-ductus/src/strokes-extra/tamil/U-B85.ts",
    "/repo/code/packages/typescript/script-ductus/src/strokes/tamil/../scriptdata.ts",
    "/repo/code/packages/typescript/script-ductus/src/strokes/tamil/U-B85.tsx",
    "/repo/code/packages/typescript/other-script-ductus/src/strokes/tamil/U-B85.ts",
  ])("does not escape or widen the handwriting boundary: %s", (moduleId) => {
    expect(isHandwritingToolsModuleId(moduleId)).toBe(false);
  });
});
