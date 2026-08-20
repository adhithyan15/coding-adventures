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
  listAssessmentContracts,
  listExamInventories,
  loadChapterPolicy,
  loadEverything,
  loadExamInventory,
  loadTrackChapters,
} from "./loader.js";
import { policyTableWidth } from "./narration-cli.js";
import { buildCurriculumGapReport } from "./report.js";
import {
  buildCompletionPlan,
  renderCompletionPlan,
  type ExamCoverageSummary,
  type InventoryPresence,
} from "./completion-plan.js";
import { measureExamCoverage } from "./exam-inventory.js";
import { isExamInventoryComplete } from "./exam-inventory.js";
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

  // Deduped on (language, level). `listExamInventories` does not, so two files
  // declaring the same pair -- which the `spanish -> es` naming convention makes
  // easy to create by accident -- produced two `exam-point` items carrying the
  // IDENTICAL id, double-counted the projection, and shrank the `exam-inventory`
  // backlog. `WorkItem.id` is documented as stable and derivable.
  const seenInventory = new Set<string>();
  const inventories = listExamInventories(options.root).flatMap<InventoryPresence>((entry) => {
    const level = CEFR_LEVELS.find((candidate) => candidate === entry.level);
    if (!level) return [];
    const key = `${entry.language}/${level}`;
    if (seenInventory.has(key)) return [];
    seenInventory.add(key);
    return [{ language: entry.language, level }];
  });

  // Coverage is measured here rather than in `buildCompletionPlan`, which stays
  // pure over report data.
  //
  // AN UNREADABLE INVENTORY IS NOT AN ABSENT ONE, and an earlier version of this
  // comment claimed it resurfaced as an `exam-inventory` item. It did not.
  // `listExamInventories` lists any file that parses and declares a string
  // language/level; `loadExamInventory` is far stricter. A file in the gap
  // between them was listed as PRESENT -- so no `exam-inventory` item -- and threw
  // on load -- so no `exam-point` item. The track vanished from both families
  // while the report asserted its inventory existed. Renaming one file toward the
  // `spanish -> es` code convention the loader already uses is enough to trigger
  // it, with no corruption at all.
  //
  // So failures are COLLECTED: named on stderr, removed from the presence list so
  // the `exam-inventory` item genuinely does come back, and counted separately in
  // the projection. Unmeasured must never be reportable as clean.
  const examCoverage: ExamCoverageSummary[] = [];
  const unreadable: { language: string; level: CefrLevel; reason: string }[] = [];
  const readable: InventoryPresence[] = [];
  const partial: InventoryPresence[] = [];
  for (const entry of inventories) {
    try {
      const inventory = loadExamInventory(entry.language, entry.level, options.root);
      const coverage = measureExamCoverage(inventory, lessons);
      (isExamInventoryComplete(inventory) ? readable : partial).push(entry);
      examCoverage.push({
        language: entry.language,
        level: entry.level,
        enumerated: coverage.enumerated,
        covered: coverage.covered,
      });
    } catch (error) {
      // Bound and filtered. A bare `catch {}` swallows a TypeError from a future
      // refactor of `measureExamCoverage` exactly like a missing file -- which
      // would turn this whole feature off and print a report whose two lines
      // contradict each other, with nothing in CI to notice.
      const cause = error as NodeJS.ErrnoException;
      const expected =
        error instanceof SyntaxError ||
        cause?.code === "ENOENT" ||
        (error instanceof Error && error.message.startsWith("exam inventory:"));
      if (!expected) throw error;
      unreadable.push({ ...entry, reason: error instanceof Error ? error.message : String(error) });
    }
  }
  for (const bad of unreadable) {
    process.stderr.write(`plan: ${bad.language} ${bad.level} inventory exists but could not be read -- ${bad.reason}\n`);
  }

  const plan = buildCompletionPlan({
    levelGate: report.levelGate,
    scriptClosure: report.scriptClosure,
    assessmentContracts: listAssessmentContracts(options.root),
    inventories: readable,
    partialInventories: partial,
    examCoverage,
    unreadableInventories: unreadable.length,
    ceiling: options.ceiling,
    headSize: options.headSize,
  });

  process.stdout.write(
    options.format === "json" ? `${JSON.stringify(plan, null, 2)}\n` : `${renderCompletionPlan(plan).join("\n")}\n`,
  );
  // Non-zero when a target we can SEE could not be measured. The queue is still
  // printed and still usable; the exit code is what stops it being mistaken for a
  // complete answer by anything automated.
  return unreadable.length > 0 ? 1 : 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exit(runCompletionPlan());
}
