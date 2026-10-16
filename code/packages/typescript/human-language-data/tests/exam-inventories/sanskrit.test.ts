import { createHash } from "node:crypto";
import { existsSync, readdirSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot, loadExamInventory } from "../../src/loader.js";

describe("the committed Sanskrit A1 inventory", () => {
  const inventory = loadExamInventory("sanskrit", "A1");

  it("reconstructs the exact pre-migration inventory from 164 direct point owners", () => {
    const core = join(defaultCurriculumRoot(), "core");
    const aggregate = join(core, "exam-inventory-sanskrit-a1.json");
    const owners = join(core, "exam-inventory-sanskrit-a1.d");
    const rendered = `${JSON.stringify(inventory, null, 2)}\n`;

    expect(existsSync(aggregate)).toBe(false);
    expect(readdirSync(owners)).toHaveLength(165);
    expect(inventory.points).toHaveLength(164);
    expect(Buffer.byteLength(rendered)).toBe(132_030);
    expect(createHash("sha256").update(rendered).digest("hex")).toBe(
      "29466e81d660629bf7e00f5a9d6f290fdcb7b8dbdd7ac489d8054b1c20d69119",
    );
  });
});
