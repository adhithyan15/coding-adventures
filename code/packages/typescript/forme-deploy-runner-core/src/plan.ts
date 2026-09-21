import type {
  DeployFileEntry,
  DeployManifest,
  DeployPlan,
  DeployPlanEntry,
} from "./types.js";
import { canonicalDeployManifest, parseDeployManifest } from "./manifest.js";

const PLAN_MANIFESTS = new WeakMap<DeployPlan, string>();

export function createDeployPlan(
  currentInput: unknown,
  previousInput?: unknown,
): DeployPlan {
  const current = parseDeployManifest(currentInput);
  const previous = previousInput === undefined
    ? undefined
    : parseDeployManifest(previousInput);
  const allPaths = new Set([
    ...Object.keys(current.files),
    ...Object.keys(previous?.files ?? {}),
  ]);
  const entries: DeployPlanEntry[] = [];
  let created = 0;
  let updated = 0;
  let skipped = 0;
  let deleted = 0;
  for (const outputPath of [...allPaths].sort(compareText)) {
    const currentEntry = current.files[outputPath];
    const previousEntry = previous?.files[outputPath];
    if (currentEntry === undefined && previousEntry !== undefined) {
      deleted += 1;
      entries.push(Object.freeze({ outputPath, action: "delete", previous: previousEntry }));
    } else if (currentEntry !== undefined && previousEntry === undefined) {
      created += 1;
      entries.push(Object.freeze({ outputPath, action: "create", current: currentEntry }));
    } else if (currentEntry !== undefined && previousEntry !== undefined) {
      if (sameEntry(currentEntry, previousEntry)) {
        skipped += 1;
        entries.push(Object.freeze({ outputPath, action: "skip", current: currentEntry, previous: previousEntry }));
      } else {
        updated += 1;
        entries.push(Object.freeze({ outputPath, action: "update", current: currentEntry, previous: previousEntry }));
      }
    }
  }
  const plan = Object.freeze({
    entries: Object.freeze(entries),
    summary: Object.freeze({ created, updated, skipped, deleted }),
  });
  PLAN_MANIFESTS.set(plan, canonicalDeployManifest(current));
  return plan;
}

export function assertDeployPlanForManifest(plan: DeployPlan, manifest: DeployManifest): void {
  const expected = PLAN_MANIFESTS.get(plan);
  if (expected === undefined) throw new TypeError("deploy plan was not produced by createDeployPlan");
  if (expected !== canonicalDeployManifest(manifest)) {
    throw new TypeError("deploy plan does not belong to the supplied current manifest");
  }
}

function sameEntry(a: DeployFileEntry, b: DeployFileEntry): boolean {
  return a.sha256 === b.sha256 &&
    a.sizeBytes === b.sizeBytes &&
    a.contentType === b.contentType &&
    a.source === b.source &&
    a.route === b.route &&
    a.lastmod === b.lastmod;
}

function compareText(a: string, b: string): number {
  return a < b ? -1 : a > b ? 1 : 0;
}
