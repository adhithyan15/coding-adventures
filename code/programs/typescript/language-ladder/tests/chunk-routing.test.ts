import { describe, expect, it } from "vitest";

import {
  isBookLedgerModuleId,
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
