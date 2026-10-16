import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { it } from "vitest";
import type { LessonBudgetKind } from "../../../src/lesson-budgets.js";
import { expectLanguageLessonBudgets } from "../assert-language-corpus.js";

const OWNER_NAME = /^(idiom|sense|culture-claim)--(SA-[A-Z0-9-]+)\.json$/;

function directUnitOwners(): Record<LessonBudgetKind, string[]> {
  const directory = join(dirname(fileURLToPath(import.meta.url)), "lesson-content-units.d");
  const units: Record<LessonBudgetKind, string[]> = {
    idiom: [],
    sense: [],
    "culture-claim": [],
  };
  const folded = new Set<string>();
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    if (!entry.isFile()) {
      throw new Error(`lesson-content unit owner '${entry.name}' must be a regular file`);
    }
    const match = OWNER_NAME.exec(entry.name);
    if (match === null) throw new Error(`unexpected lesson-content unit owner '${entry.name}'`);
    const lower = entry.name.toLowerCase();
    if (folded.has(lower)) {
      throw new Error(`lesson-content unit owner case-fold collision: '${entry.name}'`);
    }
    folded.add(lower);

    const [kind, unitId] = match.slice(1) as [LessonBudgetKind, string];
    const path = join(directory, entry.name);
    const raw = readFileSync(path, "utf8");
    const owner = JSON.parse(raw) as Record<string, unknown>;
    if (
      Object.keys(owner).sort().join(",") !== "kind,unitId" ||
      owner.kind !== kind ||
      owner.unitId !== unitId
    ) {
      throw new Error(`lesson-content unit owner '${entry.name}' must bind its filename identity`);
    }
    if (raw !== `${JSON.stringify(owner, null, 2)}\n`) {
      throw new Error(`lesson-content unit owner '${entry.name}' must use canonical JSON`);
    }
    units[kind].push(unitId);
  }
  return units;
}

// Every canonical lesson participates in complete-measurement and budget
// checks. Stable unit identities have direct owners, so adding one unit adds
// one file and deleting a declaration leaves an unmatched owner.
it("derives exact Sanskrit lesson-content budgets without shared counters", () =>
  expectLanguageLessonBudgets("sanskrit", {
    unitPrefix: "SA",
    units: directUnitOwners(),
  }));
