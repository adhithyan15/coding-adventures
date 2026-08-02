import { describe, expect, it } from "vitest";
import { LANGUAGE_STORAGE_KEY, loadLanguages, normalizeLanguageSelection, saveLanguages } from "../src/languagestore.ts";

class MemoryStorage {
  value: string | null = null;
  getItem(key: string): string | null { return key === LANGUAGE_STORAGE_KEY ? this.value : null; }
  setItem(key: string, value: string): void { if (key === LANGUAGE_STORAGE_KEY) this.value = value; }
}

describe("language selection", () => {
  const available = ["spanish", "persian", "urdu"];

  it("keeps registry order, removes unknowns and duplicates", () => {
    expect(normalizeLanguageSelection(["urdu", "unknown", "urdu", "spanish"], available))
      .toEqual(["spanish", "urdu"]);
  });

  it("never leaves the learner with an empty mix", () => {
    expect(normalizeLanguageSelection([], available)).toEqual(available);
  });

  it("round-trips through storage", () => {
    const storage = new MemoryStorage();
    expect(saveLanguages(storage, ["persian", "urdu"], available)).toEqual(["persian", "urdu"]);
    expect(loadLanguages(storage, available)).toEqual(["persian", "urdu"]);
  });
});
