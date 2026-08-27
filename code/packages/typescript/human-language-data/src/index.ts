// @coding-adventures/human-language-data
//
// The machine-readable bridge from the Human Languages curriculum (Markdown
// lessons + concept taxonomy) to the cross-language dataset the Engram deck
// generator and companion app consume. See code/specs/HL01-*.

export * from "./types.js";
export * from "./constants.js";
export * from "./cousins.js";
export * from "./exam-inventory.js";
export * from "./assessment.js";
export * from "./artifact-presence.js";
export * from "./assessment-artifacts.js";
export * from "./task-shapes.js";
export * from "./sound-tags.js";
export * from "./writing-stages.js";
export { splitFrontmatter, type Frontmatter } from "./frontmatter.js";
export {
  declaredStrands,
  strandDefects,
  nodeSizeDefects,
  summarizeStrands,
  renderStrandSummary,
  NODE_CONCEPT_TARGET,
  type StrandDefect,
  type NodeSizeDefect,
  type StrandCount,
  type StrandSummary,
} from "./strands.js";
export {
  topologicalOrder,
  cellGraphDefects,
  cellCoverage,
  renderCellCoverage,
  type CellGraphDefect,
  type CellCoverage,
} from "./grammar-cells.js";
export {
  buildRootLedger,
  renderRootLedger,
  type RootEntry,
  type RootLedger,
  type RootNamespace,
} from "./root-ledger.js";
export {
  PERSON_LABELS,
  FULL_GRID_ROWS,
  personRowCount,
  lessonInfoDump,
  measureInfoDump,
  renderInfoDump,
  type InfoDumpKind,
  type InfoDumpFinding,
  type InfoDumpReport,
} from "./info-dump.js";
export {
  declaredLessonBudgetUnits,
  measureLessonBudgets,
  renderLessonBudgets,
  type LessonBudgetKind,
  type LessonBudgetPolicy,
  type LessonBudgetFinding,
  type LessonBudgetExcess,
  type LessonBudgetReport,
} from "./lesson-budgets.js";
export {
  termsUsedIn,
  measureMetalanguage,
  renderMetalanguage,
  type MetalanguageUse,
  type MetalanguageReport,
} from "./metalanguage.js";
export { parseBodyBlocks, parseLesson, buildDataset, type ParsedLesson } from "./parse.js";
export {
  parseLessonActivityValue,
  normalizeActivityResponse,
  activityContractErrors,
  compileLessonActivity,
  compileLessonActivities,
  activityAnswerIsCorrect,
  type ParsedActivityValue,
} from "./activity.js";
export {
  fnv1a64,
  canonicalLessonSource,
  canonicalLessonHash,
  combineLessonHashes,
  combineChapterHash,
  canonicalChapterHash,
  type LessonHashEntry,
} from "./hash.js";
export {
  etymologyRootNode,
  etymologyFigureSource,
  renderEtymologyRouteFigure,
  renderFigure,
  type FigureKind,
  type FigureTarget,
  type GeneratedFigure,
} from "./figure.js";
export {
  renderInlineMarkdown,
  renderBookAnswerKey,
  renderBookChapter,
  renderBookGlossary,
  renderBookIndex,
  renderReferenceAppendix,
  bookVoice,
  bookBlockTitle,
  type BookAnswerKeyTarget,
  type BookGenerationTarget,
  type BookGlossaryTarget,
  type BookIndexTarget,
  type BookIndexChapter,
  type BookReferenceAppendixTarget,
  type GeneratedBookChapter,
} from "./book.js";
export {
  allConcepts,
  conceptsByLanguage,
  languagesForConcept,
  coverageByLanguage,
} from "./queries.js";
export { validate, hasErrors, summarize, type ValidateInput } from "./validate.js";
export {
  validateCurriculum,
  type CurriculumValidationInput,
} from "./curriculum.js";
export {
  orderedCurriculumLessonIds,
  extensionsForSegment,
  nextCurriculumLesson,
  mixedCurriculumFrontier,
  type ExtensionRelation,
  type AttachedExtension,
  type CurriculumFrontierStep,
  type MixedCurriculumFrontier,
} from "./plans.js";
export {
  defaultCurriculumRoot,
  trackScript,
  loadTaxonomy,
  loadLanguageRegistry,
  loadSoundTagRegistry,
  loadTaskShapeInventory,
  listTaskShapeInventories,
  loadCurriculumSpine,
  loadLanguageCurricula,
  loadTrackChapters,
  loadChapterPolicy,
  loadMetalanguage,
  loadBookCorpus,
  loadLessons,
  loadModalityManifest,
  modalityManifestById,
  loadScripts,
  loadLetterLedgers,
  loadEverything,
  loadTrackLessons,
} from "./loader.js";
// HL21: a ledger at `X.json` may instead live as the directory `X.d/`, so that
// many authors can append to it at once without colliding on one file.
export {
  // Exported alongside `readLedgerFile` on purpose: a consumer that catches
  // must be able to narrow BY TYPE. Without this the only way to tolerate a
  // malformed ledger from outside the package is `catch {}` or matching on
  // message text — the two patterns this change removes internally.
  LedgerParseError,
  SHARD_DIR_SUFFIX,
  isAbsentErrno,
  isSharded,
  listShardNames,
  // The guarded door for a one-file ledger, exported so that a consumer outside
  // this package has the same option its own modules do. A bare
  // `JSON.parse(readFileSync(...))` on a curriculum ledger skips four controls;
  // there should be somewhere else to go.
  readLedgerFile,
  readMaybeSharded,
  readShards,
  shardDirectoryFor,
  type ReadLedgerOptions,
  type Shard,
} from "./shard.js";
export {
  buildTrackProgress,
  renderTrackProgressCard,
  renderTrackProgressTable,
  type GeneratedBookChapterRef,
  type TrackProgress,
} from "./track-progress.js";
export {
  MODALITIES,
  MODALITY_SIGNS,
  SIGHT_CUES,
  SIGHT_CUE_RULES,
  DEFAULT_LINEARISABLE_TABLE_COLUMNS,
  modalityRank,
  requiredChannels,
  unionModalities,
  lessonText,
  tableRowColumns,
  widestTableColumns,
  hasPageArtifact,
  matchedSightCues,
  deriveLessonModality,
  modalityFindings,
  lessonModalities,
  orderChapterLessons,
  drivablePrefix,
  summarizeModality,
  type SightCueContext,
  type SightCueRule,
  type SightCueAnchor,
  type Modality,
  type ModalityOptions,
  type ModalityReasonCode,
  type ModalityFinding,
  type LessonModality,
  type ChapterModality,
  type TrackModality,
  type ModalitySummary,
} from "./modality.js";
export {
  TABLE_REFUSAL_MESSAGES,
  splitTableRow,
  isTableRowLine,
  isDelimiterCell,
  findMarkdownTables,
  speakableInline,
  collapseSpaces,
  endSentence,
  linariseTable,
  linariseTables,
  hasUnspeakableTable,
  type MarkdownTable,
  type TableRefusalReason,
  type LinearisedTable,
  type RefusedTable,
  type TableSpeech,
  type TableSpeechOptions,
} from "./speech.js";
export {
  PROMPT_RESPONSE_SECONDS,
  MANUAL_CUE_ACTIONS,
  parseNarrationCue,
  splitNarrationCues,
  pairRomanization,
  narrationTitle,
  narrateLesson,
  narrateChapter,
  narrationChapters,
  renderLessonNarrationText,
  renderChapterNarrationText,
  type NarrationSegment,
  type NarrationCue,
  type NarrationPause,
  type NarrationRepeat,
  type NarrationPrompt,
  type NarrationSpeech,
  type NarrationTable,
  type NarrationTableSkipped,
  type NarrationActivity,
  type NarrationBlock,
  type NarrationNotice,
  type NarrationFinding,
  type NarrationOptions,
  type RomanizationPair,
  type LessonNarration,
  type ChapterNarration,
} from "./narration.js";
export {
  MODALITY_MANIFEST_DIR,
  MODALITY_MANIFEST_VERSION,
  modalityCorpusHash,
  buildModalityManifest,
  mergeModalityManifests,
  serializeModalityManifest,
  type ModalityManifest,
  type ModalityManifestFeatures,
  type ModalityManifestPolicy,
  type ModalityManifestLesson,
  type ModalityManifestChapter,
  type ModalityManifestTrack,
  type ModalityManifestSummary,
} from "./modality-manifest.js";
export {
  DURATION_THRESHOLD_SECONDS,
  estimateLessonDuration,
  buildCurriculumGapReport,
  renderCurriculumGapReport,
  type DurationEstimate,
  type CurriculumGapReport,
  type CurriculumGapReportInput,
} from "./report.js";
export {
  CHAPTER_GATE_CODES,
  runChapterGates,
  runPatternGates,
  type ChapterGateCode,
  type ChapterFinding,
  type ChapterGateInput,
  type ChapterGateReport,
  type TrackChapterCoverage,
} from "./chapters.js";
export {
  CEFR_LEVELS,
  levelRank,
  levelsUpTo,
  lessonSpineNodes,
  deriveLessonLevel,
  summarizeLevels,
  lessonsUpToLevel,
  type CefrLevel,
  type LessonLevel,
  type LevelSummary,
  type TrackLevelCoverage,
} from "./levels.js";
export {
  coreVerbConcepts,
  verbCoverage,
  type TrackVerbCoverage,
  type VerbCoverageReport,
} from "./verbs.js";
export {
  measureRamp,
  measureScriptRamp,
  type RampReport,
  type RampViolation,
  type ChapterRampViolation,
  type TrackRampCoverage,
  type ScriptRampReport,
  type ScriptRampViolation,
  type ScriptSystemViolation,
  type TrackScriptRamp,
} from "./ramp.js";
export {
  measureContinuity,
  REINFORCEMENT_WINDOWS,
  type ContinuityReport,
  type OrderDefect,
  type ReinforcementDefect,
  type ForwardReference,
  type TrackContinuity,
  type WindowName,
} from "./continuity.js";
export {
  GENTLE_RAMP_PRIORITIES,
  summarizeGentleRamp,
  renderGentleRamp,
  type GentleRampFindingKind,
  type GentleRampFinding,
  type TrackGentleRamp,
  type GentleRampReport,
  type GentleRampInput,
} from "./gentle-ramp.js";
export {
  runLevelGate,
  LEVEL_VOCABULARY,
  type LevelGateReport,
  type TrackLevelAttainment,
  type LevelBlocker,
} from "./level-gate.js";
export { runValidate } from "./cli.js";
export { runCurriculumGapReport } from "./report-cli.js";
export { runGentleRampReport } from "./gentle-ramp-cli.js";
export { runCompletionPlan } from "./plan-cli.js";
export {
  measureGlyphCoverage,
  renderGlyphCoverage,
  scriptWrappers,
  mappedCharacters,
  type BookFonts,
  type GlyphGap,
  type GlyphCoverageReport,
} from "./glyph-coverage.js";
export {
  measureLiteralMarkup,
  renderLiteralMarkup,
  type LiteralMarkupFinding,
  type LiteralMarkupReport,
} from "./literal-markup.js";
export {
  buildCompletionPlan,
  renderCompletionPlan,
  CERTIFIABLE_LEVELS,
  TASK_SHAPE_LEVELS,
  KIND_PRIORITY,
  TRANCHE_SIZE,
  type CompletionPlan,
  type CompletionPlanInput,
  type InventoryPresence,
  type PlanProjection,
  type WorkItem,
  type WorkKind,
} from "./completion-plan.js";
export { generatedBookOutputs, runBookGeneration } from "./book-cli.js";
export {
  FIGURE_CONFIG_PATH,
  FIGURE_HASH_MANIFEST_PATH,
  safeFigureOutput,
  generatedFigureOutputs,
  runFigureGeneration,
} from "./figure-cli.js";
export {
  generatedModalityOutputs,
  generatedModalityOutputsFromLessons,
  runModalityManifest,
} from "./modality-cli.js";

export {
  validateLetterLedger,
  summarizeLetterLedger,
  type LetterLedger,
  type LedgerLetter,
  type LedgerUnlock,
  type LedgerIssue,
  type LedgerSummary,
} from "./letter-ledger.js";

export {
  measureScriptClosure,
  type ScriptClosureReport,
  type ClosureViolation,
  type TrackClosure,
} from "./script-closure.js";
