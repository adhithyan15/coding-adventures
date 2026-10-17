// A conservative eligibility screen for activating a missing assessment.json.
// The final contract and artifact gate remain authoritative: this only keeps
// obviously premature contracts out of the actionable completion-plan head.
import { join } from "node:path";
import { artifactExists } from "./artifact-presence.js";
import { CEFR_LEVELS, type CefrLevel } from "./levels.js";

export interface AssessmentPrerequisiteStatus {
  language: string;
  missing: string[];
}

/**
 * Measure the canonical source files needed before a track can publish its
 * seven-rung contract. Alternate mock filenames can be declared in the final
 * contract, but this screen deliberately errs on the side of a blocked item.
 */
export function measureAssessmentPrerequisites(
  root: string,
  languages: readonly string[],
  taskShapes: ReadonlyArray<{ language: string; level: CefrLevel }>,
  minimumFullMocksPerLevel: number,
): AssessmentPrerequisiteStatus[] {
  if (!Number.isInteger(minimumFullMocksPerLevel) || minimumFullMocksPerLevel < 1) {
    throw new Error("assessment prerequisites: minimum mock count must be a positive integer");
  }
  const haveShapes = new Set(taskShapes.map((entry) => `${entry.language}/${entry.level}`));
  return languages.map((language) => {
    // `join` must never be allowed to turn a registry entry into a path walk.
    if (!/^[a-z][a-z0-9-]*$/.test(language)) {
      throw new Error(`assessment prerequisites: unsafe track id '${language}'`);
    }
    const trackRoot = join(root, language);
    // Check every parent we traverse, not just the final file. `artifactExists`
    // refuses symlinks and distinguishes a missing file from an I/O fault.
    if (!artifactExists(trackRoot)) {
      throw new Error(`assessment prerequisites: missing track directory '${language}'`);
    }
    const shapeDir = join(trackRoot, "task-shapes");
    const mockRoot = join(trackRoot, "mocks");
    const haveShapeDir = artifactExists(shapeDir);
    const haveMockRoot = artifactExists(mockRoot);
    const missing: string[] = [];
    for (const level of CEFR_LEVELS) {
      const slug = level.toLowerCase();
      if (!haveShapeDir || !haveShapes.has(`${language}/${level}`)
        || !artifactExists(join(shapeDir, `${slug}.json`))) {
        missing.push(`task-shapes/${slug}.json`);
      }
      const mockDir = join(mockRoot, slug);
      if (!haveMockRoot || !artifactExists(mockDir)) {
        missing.push(`mocks/${slug}/ (timed papers, rubric, answer keys)`);
        continue;
      }
      const rubric = artifactExists(join(mockDir, "rubric.md"));
      for (let number = 1; number <= minimumFullMocksPerLevel; number += 1) {
        const stem = `mock-${number}`;
        const paper = artifactExists(join(mockDir, `${stem}.md`))
          || artifactExists(join(mockDir, `${stem}-paper.md`));
        if (!paper) missing.push(`mocks/${slug}/${stem}.md or ${stem}-paper.md`);
        if (!artifactExists(join(mockDir, `${stem}-answer-key.md`))) {
          missing.push(`mocks/${slug}/${stem}-answer-key.md`);
        }
        if (!rubric && !artifactExists(join(mockDir, `${stem}-rubric.md`))) {
          missing.push(`mocks/${slug}/rubric.md or ${stem}-rubric.md`);
        }
      }
    }
    return { language, missing };
  });
}
