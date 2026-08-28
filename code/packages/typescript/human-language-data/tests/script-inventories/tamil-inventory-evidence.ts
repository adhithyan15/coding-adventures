import { createHash } from "node:crypto";
import { expect } from "vitest";
import { scriptEntryId } from "../../src/script-shards.js";
import type { Letter, Mark } from "../../src/types.js";
import type { ScriptEvidenceContext } from "./helpers.js";

export interface TamilInventoryEvidenceOptions {
  readonly name: string;
  readonly section: "letters" | "marks";
  readonly id: string;
  readonly digest: string;
  readonly assert?: (
    entry: Letter | Mark,
    context: ScriptEvidenceContext,
  ) => void;
}

function glyphFromId(id: string): string {
  if (!/^U-[0-9A-F]+(?:-U-[0-9A-F]+)*$/.test(id)) {
    throw new Error(
      `invalid Tamil inventory evidence id ${JSON.stringify(id)}`,
    );
  }
  return id
    .slice(2)
    .split("-U-")
    .map((hex) => String.fromCodePoint(Number.parseInt(hex, 16)))
    .join("");
}

export function tamilInventoryEvidence(options: TamilInventoryEvidenceOptions) {
  return {
    name: options.name,
    assert(context: ScriptEvidenceContext): void {
      const glyph = glyphFromId(options.id);
      const inventory = context.scripts.tamil!;
      const entries =
        options.section === "letters"
          ? inventory.letters
          : (inventory.marks ?? []);
      const entry = entries.find((candidate) =>
        options.section === "letters"
          ? (candidate as Letter).glyph === glyph
          : (candidate as Mark).mark === glyph,
      );
      expect(entry, `${options.section}/${options.id}`).toBeDefined();
      expect(scriptEntryId(glyph)).toBe(options.id);
      expect(
        createHash("sha256").update(JSON.stringify(entry)).digest("hex"),
      ).toBe(options.digest);
      expect(
        context.missingByScript.get("tamil.json")?.has(glyph) ?? false,
      ).toBe(false);
      expect(context.affected.get(glyph) ?? 0).toBe(0);
      options.assert?.(entry!, context);
    },
  };
}
