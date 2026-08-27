import { resolve } from "node:path";
import { pathToFileURL } from "node:url";
import {
  defaultCurriculumRoot as defaultRoot,
  loadChapterPolicy,
  loadAssessmentPolicy,
  loadBookFonts,
  loadEverything,
  loadMainFontCharset,
  loadGrammarSlots,
  loadMetalanguage,
  loadTrackChapters,
  loadTrackGrammarCells,
} from "./loader.js";
import { policyTableWidth } from "./narration-cli.js";
import { buildCurriculumGapReport, renderCurriculumGapReport } from "./report.js";
import { renderStrandSummary, summarizeStrands } from "./strands.js";
import { cellCoverage, renderCellCoverage } from "./grammar-cells.js";
import { buildRootLedger, renderRootLedger } from "./root-ledger.js";
import { measureInfoDump, renderInfoDump } from "./info-dump.js";
import { measureLessonBudgets, renderLessonBudgets } from "./lesson-budgets.js";
import { measureMetalanguage, renderMetalanguage } from "./metalanguage.js";
import { measureLiteralMarkup, renderLiteralMarkup } from "./literal-markup.js";
import { measureGlyphCoverage, renderGlyphCoverage } from "./glyph-coverage.js";

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
    assessmentPolicy: loadAssessmentPolicy(options.root),
    // Without these the `levels` section and the HL09 §3.1 level gate are both
    // silently absent — which is how the CLI managed to never once print the level
    // figures after HL-C10 shipped them.
    curricula,
    spine,
  });
  // HL10 §2 and §5, both report-only. Appended rather than folded into
  // CurriculumGapReport because they measure the SPINE and the GRAMMAR
  // inventory, not the lesson corpus; merging them in would make a spine defect
  // look like a lesson defect.
  const policy = loadChapterPolicy(options.root);
  const strands = summarizeStrands(spine, policy.maxNewAtomsPerChapter);
  const slots = loadGrammarSlots(options.root);
  const spanishCells = loadTrackGrammarCells("spanish", options.root);
  // The cell budget measures a burden the atom budget cannot see: three cells in
  // one lesson is three atoms and looks compliant, while being the six-form
  // table arriving all at once.
  const cells = cellCoverage(
    spanishCells,
    lessons.filter((lesson) => lesson.language === "spanish"),
    policy.maxNewGrammarCellsPerLesson,
  );

  // HL10 §6.2. Etymology is the corpus's signature, but a root is only useful
  // if it is spent again -- so the ledger counts payoffs, not mentions.
  const rootLedger = buildRootLedger(lessons, policy.rootLedgerMinReuse ?? 3);

  // HL10 §7.3. The owner's "never info dump" made checkable. The prose turned
  // out to be fine; the dumps live in paradigm tables.
  const infoDump = measureInfoDump(lessons, policy.maxRuleStatementsPerLesson ?? 1);

  // HL10 sections 5.5, 7.1 and 7.2. These use explicit declarations because
  // prose heuristics cannot distinguish a lexical item from a new sense, or a
  // useful phrase from an idiom. Unannotated legacy lessons remain visible as
  // unmeasured debt instead of being silently certified as clean.
  const lessonBudgets = measureLessonBudgets(lessons, {
    idioms: policy.maxNewIdiomsPerLesson ?? 1,
    senses: policy.maxNewSensesPerLesson ?? 1,
    cultureClaims: policy.maxNewCultureClaimsPerLesson ?? 2,
  });

  // HL10 §7.5. The hidden prerequisite: a book that says "the first-person
  // singular present indicative" has spent six technical terms on one form.
  const metalanguage = measureMetalanguage(lessons, loadMetalanguage(options.root));

  // HL-C217. Authoring markup that survived escaping into reader-facing text --
  // the one class the rest of this suite cannot see, because the output is both
  // safe and reproducible and simply says the wrong thing. Source layer only
  // here; the rendered layer is checked in the test suite, which already has the
  // generator's output in hand and does not need the report to rebuild it.
  const literalMarkup = measureLiteralMarkup(lessons);

  // HL-C214/HL-C223. Will every character actually render? Twice a tranche has
  // been merged-ready and failed CI on a character absent from the book's font,
  // because every other gate reads the corpus and none of them opens a font.
  const glyphs = measureGlyphCoverage(loadBookFonts(options.root), loadMainFontCharset(options.root));

  const json = `${JSON.stringify(
    { ...report, strands, grammarCells: cells, rootLedger: rootLedger.summary, infoDump: infoDump.summary, lessonBudgets: lessonBudgets.summary, metalanguage: metalanguage.summary, literalMarkup: literalMarkup.summary, glyphCoverage: glyphs.summary },
    null,
    2,
  )}\n`;
  const text = [
    renderCurriculumGapReport(report),
    renderStrandSummary(strands).join("\n"),
    "",
    renderCellCoverage(spanishCells, slots, cells).join("\n"),
    "",
    renderRootLedger(rootLedger).join("\n"),
    "",
    renderInfoDump(infoDump).join("\n"),
    "",
    renderLessonBudgets(lessonBudgets).join("\n"),
    "",
    renderMetalanguage(metalanguage).join("\n"),
    renderLiteralMarkup(literalMarkup).join("\n"),
    renderGlyphCoverage(glyphs).join("\n"),
    "",
  ].join("\n");
  process.stdout.write(options.format === "json" ? json : text);
  return 0;
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  process.exit(runCurriculumGapReport());
}
