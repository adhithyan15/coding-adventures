import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  defaultCurriculumRoot as defaultRoot,
  loadChapterPolicy,
  loadEverything,
  loadTrackChapters,
} from "./loader.js";
import { policyTableWidth } from "./narration-cli.js";
import { buildCurriculumGapReport } from "./report.js";
import { renderGentleRamp } from "./gentle-ramp.js";

interface GentleRampOptions {
  root?: string;
  format: "json" | "text";
}

function parseOptions(args: string[]): GentleRampOptions {
  const options: GentleRampOptions = { format: "text" };
  for (let index = 0; index < args.length; index += 1) {
    const flag = args[index];
    const value = args[index + 1];
    if (flag !== "--root" && flag !== "--format") throw new Error(`unknown argument '${flag}'`);
    if (!value || value.startsWith("--")) throw new Error(`${flag} requires a value`);
    if (flag === "--root") options.root = resolve(value);
    else if (value === "json" || value === "text") options.format = value;
    else throw new Error("--format must be 'json' or 'text'");
    index += 1;
  }
  return options;
}

/** Print only the prioritized ramp backlog, without the megabyte-scale full report. */
export function runGentleRampReport(args = process.argv.slice(2)): number {
  let options: GentleRampOptions;
  try {
    options = parseOptions(args);
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    return 2;
  }

  const { registry, lessons, books, curricula, spine } = loadEverything(options.root);
  const report = buildCurriculumGapReport({
    registry,
    lessons,
    books,
    modality: { maxLinearisableTableColumns: policyTableWidth(options.root ?? defaultRoot()) },
    trackChapters: loadTrackChapters(options.root),
    chapterPolicy: loadChapterPolicy(options.root),
    curricula,
    spine,
  });
  if (!report.gentleRamp) {
    process.stderr.write("gentle-ramp: chapter policy was not loaded; cannot measure the ramp\n");
    return 2;
  }
  process.stdout.write(
    options.format === "json"
      ? `${JSON.stringify(report.gentleRamp, null, 2)}\n`
      : `${renderGentleRamp(report.gentleRamp).join("\n")}\n`,
  );
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runGentleRampReport());
}
