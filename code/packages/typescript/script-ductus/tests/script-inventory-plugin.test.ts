import { describe, expect, it } from "vitest";
import { join, resolve } from "node:path";
import {
  RESOLVED_SCRIPT_INVENTORIES_ID,
  SCRIPT_INVENTORIES_ID,
  loadScriptInventoryModule,
  resolveScriptInventoryId,
  scriptInventoryModuleIdsForShardPath,
} from "../script-inventory-plugin.ts";

const repoRoot = resolve(import.meta.dirname, "../../../..");
const curriculumRoot = join(repoRoot, "code", "learning", "human-languages");

describe("script inventory virtual module", () => {
  it("resolves one fixed public id and rejects path-like variants", () => {
    expect(resolveScriptInventoryId(SCRIPT_INVENTORIES_ID)).toBe(
      RESOLVED_SCRIPT_INVENTORIES_ID,
    );
    expect(resolveScriptInventoryId(`${SCRIPT_INVENTORIES_ID}/../../outside`)).toBeNull();
    expect(resolveScriptInventoryId("virtual:some-other-inventories")).toBeNull();
  });

  it("emits exactly three named shard-backed inventories and watches every shard", () => {
    const watched: string[] = [];
    const source = loadScriptInventoryModule(
      RESOLVED_SCRIPT_INVENTORIES_ID,
      curriculumRoot,
      (path) => watched.push(path),
    );
    expect(source).not.toBeNull();
    expect(source).toContain("export const japanese =");
    expect(source).toContain("export const persoArabic =");
    expect(source).toContain("export const urduNastaliq =");
    expect(source).not.toContain("import.meta.glob");
    expect(watched.some((path) => path.endsWith("japanese.d/_meta.json"))).toBe(true);
    expect(watched.some((path) => path.endsWith("perso-arabic.d/_meta.json"))).toBe(true);
    expect(watched.some((path) => path.endsWith("urdu-nastaliq.d/_meta.json"))).toBe(true);
  });

  it("invalidates only the fixed module for selected in-root shard paths", () => {
    expect(
      scriptInventoryModuleIdsForShardPath(
        join(curriculumRoot, "data/scripts/perso-arabic.d/letters/0010-U-627.json"),
        curriculumRoot,
      ),
    ).toEqual([RESOLVED_SCRIPT_INVENTORIES_ID]);
    expect(
      scriptInventoryModuleIdsForShardPath(
        join(curriculumRoot, "data/scripts/arabic.d/letters/0010-U-627.json"),
        curriculumRoot,
      ),
    ).toEqual([]);
    expect(
      scriptInventoryModuleIdsForShardPath(
        join(curriculumRoot, "../outside/perso-arabic.d/letters/0010-U-627.json"),
        curriculumRoot,
      ),
    ).toEqual([]);
  });
});
