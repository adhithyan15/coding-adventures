// ---------------------------------------------------------------------------
// plan-cli.ts — print the computed work queue.
//
// `report-cli` answers "what is the state of the corpus". This answers the only
// question that follows it: "what do I do next". They are deliberately separate
// binaries — the report is long and diagnostic, and an agent picking up the next
// item should not have to read 100 lines of modality tables to find it.
//
// See `code/specs/HL15-the-completion-plan.md`.
// ---------------------------------------------------------------------------
import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  defaultCurriculumRoot as defaultRoot,
  listExamInventories,
  loadChapterPolicy,
  loadEverything,
  loadTrackChapters,
} from "./loader.js";
import { policyTableWidth } from "./narration-cli.js";
import { buildCurriculumGapReport } from "./report.js";
import { buildCompletionPlan, renderCompletionPlan, type InventoryPresence } from "./completion-plan.js";
import { CEFR_LEVELS, type CefrLevel } from "./levels.js";

interface PlanOptions {
  root?: string;
  format: "json" | "text";
  ceiling: CefrLevel;
  headSize: number;
}

function parseOptions(args: string[]): PlanOptions {
  const options: PlanOptions = { format: "text", ceiling: "C2", headSize: 25 };
  for (let index = 0; index < args.length; index += 1) {
    const flag = args[index];
    const value = args[index + 1];
    if (flag !== "--root" && flag !== "--format" && flag !== "--ceiling" && flag !== "--head") {
      throw new Error(`unknown argument '${flag}'`);
    }
    // A missing value and a value that is itself the next FLAG are the same
    // typo. Without the second check `--root --format json` silently takes
    // `--format` as the root and dies with an ENOENT stack trace instead of the
    // clean exit-2 path every other parse error uses.
    if (!value || value.startsWith("--")) throw new Error(`${flag} requires a value`);
    if (flag === "--root") options.root = resolve(value);
    else if (flag === "--format") {
      if (value !== "json" && value !== "text") throw new Error("--format must be 'json' or 'text'");
      options.format = value;
    } else if (flag === "--ceiling") {
      const level = CEFR_LEVELS.find((candidate) => candidate === value);
      if (!level) throw new Error(`--ceiling must be one of ${CEFR_LEVELS.join(", ")}`);
      options.ceiling = level;
    } else {
      // A head size of zero is legal and means "projection only". A negative or
      // non-numeric one is a typo, and slicing on NaN would silently return an
      // EMPTY head that looks exactly like "there is no work left".
      const size = Number(value);
      if (!Number.isInteger(size) || size < 0) throw new Error("--head must be a non-negative integer");
      options.headSize = size;
    }
    index += 1;
  }
  return options;
}

export function runCompletionPlan(args = process.argv.slice(2)): number {
  let options: PlanOptions;
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

  // `levelGate` is OPTIONAL on `CurriculumGapReport`: it is `undefined` when its
  // inputs were not supplied, which is a different fact from "every track
  // passed". Refuse rather than plan against a missing section — an absent gate
  // yields an EMPTY queue, and an empty queue reads exactly like victory. This
  // is the same trap `report-cli` fell into when it silently omitted `levels`
  // from every run for the life of that feature.
  if (!report.levelGate) {
    process.stderr.write("plan: the level gate did not run; cannot compute a queue\n");
    return 2;
  }

  const inventories = listExamInventories(options.root).flatMap<InventoryPresence>((entry) => {
    const level = CEFR_LEVELS.find((candidate) => candidate === entry.level);
    return level ? [{ language: entry.language, level }] : [];
  });

  const plan = buildCompletionPlan({
    levelGate: report.levelGate,
    scriptClosure: report.scriptClosure,
    inventories,
    ceiling: options.ceiling,
    headSize: options.headSize,
  });

  process.stdout.write(
    options.format === "json" ? `${JSON.stringify(plan, null, 2)}\n` : `${renderCompletionPlan(plan).join("\n")}\n`,
  );
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exit(runCompletionPlan());
}
