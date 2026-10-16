import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot, loadExamInventory } from "../../src/loader.js";

describe("the committed Sanskrit A1 inventory", () => {
  const inventory = loadExamInventory("sanskrit", "A1");

  it("loads exclusively from direct point owners", () => {
    const core = join(defaultCurriculumRoot(), "core");
    const aggregate = join(core, "exam-inventory-sanskrit-a1.json");
    const owners = join(core, "exam-inventory-sanskrit-a1.d");
    const ownerNames = readdirSync(owners).filter(
      (name) => name !== "_meta.json",
    );

    expect(existsSync(aggregate)).toBe(false);
    expect(ownerNames).toHaveLength(inventory.points.length);
    expect(new Set(inventory.points.map((point) => point.id)).size).toBe(
      inventory.points.length,
    );
  });
});
