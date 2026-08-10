import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  defaultCurriculumRoot as defaultRoot,
  loadChapterPolicy,
  loadEverything,
  loadTrackChapters,
} from "./loader.js";
import { policyTableWidth } from "./narration-cli.js";
import { buildCurriculumGapReport, renderCurriculumGapReport } from "./report.js";
import { renderStrandSummary, summarizeStrands } from "./strands.js";

interface ReportOptions {
  root?: string;
  format: "json" | "text";
}

function parseOptions(args: string[]): ReportOptions {
  const options: ReportOptions = { format: "text" };
  for (let index = 0; index < args.length; index += 1) {
    const flag = args[index];
    if (flag !== "--root" && flag !== "--format") {
      throw new Error(`unknown argument '${flag}'`);
    }
    const value = args[index + 1];
    if (!value) throw new Error(`${flag} requires a value`);
    if (flag === "--root") options.root = resolve(value);
    else if (value === "json" || value === "text") options.format = value;
    else throw new Error(`--format must be 'json' or 'text'`);
    index += 1;
  }
  return options;
}

export function runCurriculumGapReport(args = process.argv.slice(2)): number {
  let options: ReportOptions;
  try {
    options = parseOptions(args);
  } catch (error) {
    process.stderr.write(`${error instanceof Error ? error.message : String(error)}\n`);
    return 2;
  }
  const { registry, lessons, books, curricula, spine } = loadEverything(options.root);
  // The report's drivable percentages and the committed narration export must be
  // computed at the same table width, or the report will advertise a car-friendly
  // corpus the export cannot actually deliver. One policy file, read by both.
  const report = buildCurriculumGapReport({
    registry,
    lessons,
    books,
    modality: { maxLinearisableTableColumns: policyTableWidth(options.root ?? defaultRoot()) },
    // HL05 gates run here, report-only. The ledgers and the policy are loaded from the
    // same root as everything else so the published counts and the committed chapter
    // ledgers can never be measured against different files.
    trackChapters: loadTrackChapters(options.root),
    chapterPolicy: loadChapterPolicy(options.root),
    // Without these the `levels` section and the HL09 §3.1 level gate are both
    // silently absent — which is how the CLI managed to never once print the level
    // figures after HL-C10 shipped them.
    curricula,
    spine,
  });
  // HL10 §2, report-only. Appended rather than folded into CurriculumGapReport
  // because the strand model measures the SPINE, not the lesson corpus, and
  // merging the two would make a spine defect look like a lesson defect.
  const policy = loadChapterPolicy(options.root);
  const strands = summarizeStrands(spine, policy.maxNewAtomsPerChapter);

  const json = `${JSON.stringify({ ...report, strands }, null, 2)}\n`;
  const text = `${renderCurriculumGapReport(report)}${renderStrandSummary(strands).join("\n")}\n`;
  process.stdout.write(options.format === "json" ? json : text);
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runCurriculumGapReport());
}
