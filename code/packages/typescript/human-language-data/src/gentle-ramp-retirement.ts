import { readdirSync } from "node:fs";
import { resolve } from "node:path";

export const RETIRED_GENTLE_RAMP_SNAPSHOT_ENTRY = "gentle-ramp-snapshots";

/**
 * Generated gentle-ramp snapshots duplicated facts that are already derived
 * from canonical lessons, curricula, chapter policy, and chapter ledgers.
 * Keep the retired path absent so no consumer can quietly make that copy
 * authoritative again.
 */
export function assertGentleRampSnapshotsRetired(root: string): void {
  const core = resolve(root, "core");
  const resurrected = readdirSync(core, { withFileTypes: true })
    .filter((entry) => entry.name.toLowerCase() === RETIRED_GENTLE_RAMP_SNAPSHOT_ENTRY)
    .map((entry) => entry.name);
  if (resurrected.length > 0) {
    throw new Error(
      `retired gentle-ramp snapshot path resurrected: ${resurrected.join(", ")}; ` +
        "derive the report from canonical curriculum sources instead",
    );
  }
}
