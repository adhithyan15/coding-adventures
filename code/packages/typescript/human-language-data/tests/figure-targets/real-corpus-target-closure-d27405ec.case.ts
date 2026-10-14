import { describe, expect, it } from "vitest";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { loadFilmstripTargetCountPins } from "../filmstrip-target-count-pins.js";
import { realCorpusTargets as targets } from "./real-corpus-fixture.js";

describe("real-corpus figure-target closure", () => {
  it("draws every switched-on track exactly the letters its ductus cites", () => {
    const counts: Record<string, number> = {};
    for (const target of targets) {
      const prefix = target.lessonId.split("-")[0]!;
      counts[prefix] = (counts[prefix] ?? 0) + 1;
    }
    const pins = loadFilmstripTargetCountPins(
      join(dirname(fileURLToPath(import.meta.url)), "..", "filmstrip-target-counts"),
    );
    const byTrack: Record<string, number> = Object.fromEntries(
      Object.entries(counts).map(([prefix, count]) => [
        targets.find((target) => target.lessonId.startsWith(`${prefix}-`))!.output.split("/", 1)[0]!,
        count,
      ]),
    );
    // A pinned track may draw nothing at all (latin, whose one strip was on a
    // composition), and a pin of 0 still has to hold.
    for (const track of Object.keys(pins)) byTrack[track] ??= 0;
    expect(byTrack).toEqual(pins);
  });

});
