import { describe, expect, it } from "vitest";

import {
  isBookLedgerModuleId,
  isHandwritingToolsModuleId,
} from "../chunk-routing";

describe("book-ledgers chunk boundary", () => {
  it.each([
    "\0virtual:human-language-ledger/chapters/spanish",
    "\0virtual:human-language-ledger/book-hashes/tamil",
    "virtual:human-language-ledger/book-hashes/marwari-legacy",
    "virtual:human-language-ledger\\book-hashes\\hindi",
  ])("keeps one reconstructed child per track lazy: %s", (moduleId) => {
    expect(isBookLedgerModuleId(moduleId)).toBe(true);
  });

  it.each([
    "virtual:human-language-ledgers",
    "virtual:human-language-ledger/book-hashes/../spanish",
    "virtual:human-language-ledger/book-hashes/spanish/0001.json",
    "/repo/learning/human-languages/core/generated-book-hashes/spanish.d/0001.json",
    "virtual:human-language-ledger/curriculum/spanish",
  ])(
    "does not widen the lazy group to shard or unrelated modules: %s",
    (moduleId) => {
      expect(isBookLedgerModuleId(moduleId)).toBe(false);
    },
  );
});

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
