import {
  cpSync,
  mkdtempSync,
  readFileSync,
  rmSync,
  writeFileSync,
} from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { defaultCurriculumRoot } from "../src/loader.js";
import {
  SCRIPT_OWNER_EVIDENCE_CONFIGS,
  checkScriptOwnerEvidence,
  scriptOwnerEvidenceRelativePath,
} from "../src/script-owner-evidence.js";

const corpus = defaultCurriculumRoot();

function fixture(): string {
  const root = mkdtempSync(join(tmpdir(), "hl-script-owner-evidence-"));
  cpSync(
    join(corpus, "data", "scripts", "japanese.d"),
    join(root, "data", "scripts", "japanese.d"),
    { recursive: true },
  );
  cpSync(
    join(corpus, "data", "script-owner-evidence", "japanese"),
    join(root, "data", "script-owner-evidence", "japanese"),
    { recursive: true },
  );
  return root;
}

function withFixture(run: (root: string) => void): void {
  const root = fixture();
  try {
    run(root);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function inventory(root: string): string {
  return join(root, "data", "scripts", "japanese.d", "letters", "0010-U-3042.json");
}

function evidence(root: string): string {
  return join(root, "data", "script-owner-evidence", "japanese", "letters", "U-3042.json");
}

describe("per-owner script inventory evidence", () => {
  it.each(SCRIPT_OWNER_EVIDENCE_CONFIGS)("matches every $script owner on the real corpus", (options) => {
    expect(() => checkScriptOwnerEvidence(corpus, options)).not.toThrow();
  });

  it("detects mutation of one otherwise-valid inventory owner", () => {
    withFixture((root) => {
      const path = inventory(root);
      const value = JSON.parse(readFileSync(path, "utf8")) as Record<string, unknown>;
      value.sound = "changed";
      writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
      expect(() => checkScriptOwnerEvidence(root, { language: "japanese", script: "japanese" }))
        .toThrow(/U-3042.*bytes differ/);
    });
  });

  it("detects deletion on either side", () => {
    withFixture((root) => {
      rmSync(inventory(root));
      expect(() => checkScriptOwnerEvidence(root, { language: "japanese", script: "japanese" }))
        .toThrow(/missing \[U-3042\]/);
    });
    withFixture((root) => {
      rmSync(evidence(root));
      expect(() => checkScriptOwnerEvidence(root, { language: "japanese", script: "japanese" }))
        .toThrow(/unexpected \[U-3042\]/);
    });
  });

  it("binds the evidence filename to its embedded glyph", () => {
    withFixture((root) => {
      const path = evidence(root);
      const value = JSON.parse(readFileSync(path, "utf8")) as Record<string, unknown>;
      value.glyph = "い";
      writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, "utf8");
      expect(() => checkScriptOwnerEvidence(root, { language: "japanese", script: "japanese" }))
        .toThrow(/claims 'U-3042'.*glyph is 'U-3044'/);
    });
  });

  it("gives two same-script additions disjoint evidence paths", () => {
    expect(scriptOwnerEvidenceRelativePath("tamil", "letter", "ஶ"))
      .not.toBe(scriptOwnerEvidenceRelativePath("tamil", "letter", "ஜ"));
  });
});
