// Data model for the Human Languages data layer (spec: code/specs/HL01-*).
//
// The whole point of this package is to turn the curriculum's Markdown lessons
// into something a program can reason about *across* languages. The unit of that
// reasoning is the CONCEPT — a language-independent idea ("the greeting you say
// on meeting someone") — and each language's word for it is a REALIZATION. These
// types are that model.

/**
 * A writing-system identifier. **Open by design** — this is a plain string, not a
 * closed union, so teaching a new script (Gujarati, Hebrew, Greek, …) never
 * requires a code change: you drop in a `data/scripts/<script>.json` and point a
 * track at it. Values in use today: `latin`, `devanagari`, `bengali`, `gurmukhi`,
 * `tamil`, `telugu`, `kannada`, `malayalam`, `arabic`.
 */
export type Script = string;

/** Grammatical gender tag on a noun. `null` when the language/word carries none. */
export type Gender = "masc" | "fem" | "neut" | null;

/** One entry in `concepts/taxonomy.json` — a canonical universal concept. */
export interface TaxonomyConcept {
  family: string;
  gloss: string;
  /** Expected in every track once it declares parity. */
  core: boolean;
  /** Pre-canonical tags this id replaced, for traceability. */
  retires?: string[];
  notes?: string;
}

export interface Taxonomy {
  version: number;
  concepts: Record<string, TaxonomyConcept>;
}

/**
 * One language's realization of a concept — the row the app turns into a card.
 * Derived from a single lesson's frontmatter (plus a couple of best-effort
 * heuristics where the field isn't authored yet).
 */
export interface Realization {
  concept: string; // canonical or namespaced concept id
  language: string; // track slug: "spanish", "telugu", …
  lessonId: string; // e.g. ES-C01-dia
  chapter: number;
  type: string; // word | phrase | practice | practice-mix | review | writing
  headword: string; // "día" / "నమస్కారం"
  gloss: string; // "day (el día — masculine)"
  romanization: string; // "DEE-ah"; = headword for Latin script; "" if unknown
  script: Script;
  gender: Gender;
  sounds: string[]; // ids into pronunciation-reference.md
  roots: string[]; // etymological roots, for indexing
  etymologyHook: string; // ≤120-char memory anchor; "" if not authored yet
}

/** A concept plus every language that realizes it — the cross-language join. */
export interface Concept {
  id: string;
  family: string;
  gloss: string;
  core: boolean;
  /** True when language-local (namespaced id, not in the taxonomy). */
  namespaced: boolean;
  realizations: Realization[];
}

/** The whole parsed curriculum, ready for querying. */
export interface Dataset {
  taxonomy: Taxonomy;
  concepts: Concept[];
  byLanguage: Record<string, Realization[]>;
  languages: string[];
}

// ---- Shared curriculum spine (HL04) ---------------------------------------

export type CurriculumStage = "pre-A1" | "A1" | "A2" | "B1" | string;

export interface LanguageDefinition {
  id: string;
  name: string;
  family: string;
  script: Script;
  status: "active" | "planned" | string;
  /** Languages whose history, vocabulary, or structure gives this track a bridge. */
  bridges: string[];
}

export interface LanguageRegistry {
  version: number;
  /** Authored order used as the default cross-language walk. */
  languages: LanguageDefinition[];
}

/**
 * HL10 section 2: the eight parallel ladders the curriculum runs.
 *
 * Before this existed there was one organising line -- FUNCTION, "I can greet
 * someone" -- and grammar, culture and etymology rode inside whichever lesson
 * happened to need them. That is workable at 188 lessons and unfalsifiable at
 * 5,000: you cannot ask whether the grammar ramp is gentle when grammar is not
 * a thing the data model knows about.
 *
 * Universality differs by strand, and section 4 of HL10 turns that into a
 * contract other tracks depend on:
 *   FUNCTION, TEXT              fully universal, node for node
 *   GRAMMAR, SOUND, ETYMOLOGY   universal SLOTS, filled per language
 *   CULTURE, IDIOM              universal HOOKS, content entirely local
 */
export const CURRICULUM_STRANDS = [
  "FUNCTION",
  "GRAMMAR",
  "LEXICON",
  "SOUND",
  "ETYMOLOGY",
  "CULTURE",
  "IDIOM",
  "TEXT",
] as const;

export type CurriculumStrand = (typeof CURRICULUM_STRANDS)[number];

export interface SpineNode {
  id: string;
  stage: CurriculumStage;
  /**
   * Which of the eight ladders this node advances. Exactly one -- a node that
   * genuinely serves two is two nodes, because a rung you can reach by two
   * different routes cannot be ordered against the rest of either ladder.
   */
  strand: CurriculumStrand;
  canDo: string;
  prerequisites: string[];
  core: boolean;
  /** Canonical concept ids taught inside this communicative ability. */
  concepts: string[];
}

export interface CurriculumSpine {
  version: number;
  stages: CurriculumStage[];
  /** The declared strand vocabulary. Authoritative over CURRICULUM_STRANDS at load time. */
  strands?: CurriculumStrand[];
  strandNote?: string;
  nodes: SpineNode[];
}

/** How strongly a language-specific extension participates in the local path. */
export type CurriculumExtensionKind =
  | "required"
  | "supporting"
  | "reference"
  | "not-applicable";

/** Open so a track can name a genuinely language-specific concern. */
export type CurriculumExtensionCategory =
  | "script"
  | "grammar"
  | "register"
  | "culture-pragmatics"
  | "etymology"
  | "consolidation"
  | "language-specific"
  | string;

/** A grammar, script, register, or other local node attached to one path segment. */
export interface CurriculumExtensionNode {
  id: string;
  stage: CurriculumStage;
  kind: CurriculumExtensionKind;
  category: CurriculumExtensionCategory;
  canDo: string;
  /** Extension ids and/or shared spine-node ids. */
  prerequisites: string[];
  /** Existing micro-lessons that realize this extension. */
  lessons: string[];
}

/**
 * One contiguous visit to a shared ability in a language's real local order.
 *
 * A node may occur in several segments. That is deliberate: a track can greet,
 * move on, and later return for a formal greeting without flattening its actual
 * prerequisite path into one fictional chapter.
 */
export interface CurriculumPathSegment {
  id: string;
  spine_node: string;
  /** Exact prerequisite-safe lesson order inside this segment. */
  lessons: string[];
  /** Extension ids classified by their relationship to the shared material. */
  before: string[];
  inline: string[];
  after: string[];
}

/** Coverage ledger for one shared node in one language. */
export interface SpineRealizationMap {
  /** Segment ids in their authored local order. */
  segments: string[];
  /** Canonical concepts deliberately absent from the current track corpus. */
  omits: string[];
  /** Canonical concepts this track deliberately teaches under another node. */
  relocates: Record<string, string>;
}

/** One track's executable realization of the shared spine. */
export interface LanguageCurriculum {
  version: number;
  language: string;
  /** The local, prerequisite-safe walk; shared nodes may repeat. */
  path: CurriculumPathSegment[];
  /** Every shared node is explicit, including empty/planned coverage. */
  spine: Record<string, SpineRealizationMap>;
  extensions: CurriculumExtensionNode[];
  /**
   * Track-local tags that satisfy a language-neutral spine concept.
   *
   * The spine names a concept once for all 22 tracks (`TENSE-BACKSHIFT`); a
   * track names its lessons in its own terms (`ES-REPORT-BACKSHIFT`). This lets
   * the second answer the first without retagging the lesson and discarding the
   * specific name, which carries information the spine name does not.
   */
  conceptAliases?: Record<string, string[]>;
}

// ---------------------------------------------------------------------------
// HL05 — the chapter capability layer
// ---------------------------------------------------------------------------
//
// A chapter used to be nothing but an integer stamped on each lesson. Nothing in
// the data model knew what a chapter was FOR, so nothing could check that
// finishing one left the reader able to do anything. These types are that missing
// promise, made explicit and therefore checkable.
//
// The distinction that matters: `curriculum.json`'s `omits`/`relocates` ledgers are
// recomputed CACHES — a validator derives them and errors on drift. A chapter
// capability is authored INTENT. No validator may rewrite it.

/** How a chapter proves its promise. */
export type ChapterPayoffKind = "dialogue" | "task" | "production";

/**
 * The thing the reader can actually do at the end of a chapter.
 *
 * `assesses` is the load-bearing field. Without it a payoff could satisfy its
 * chapter by exercising a single word, letting the chapter claim a capability it
 * never delivered — which is the exact failure the representativeness rule exists
 * to catch (HL05).
 */
export interface ChapterPayoff {
  /** Lesson id that delivers the payoff — normally a practice/practice-mix/pattern. */
  lesson: string;
  kind: ChapterPayoffKind;
  /** One line describing the payoff, for the chapter opening and the gap report. */
  summary: string;
  /** Knowledge atoms the payoff exercises. */
  assesses: string[];
}

/** One chapter's authored promise. */
export interface ChapterCapability {
  chapter: number;
  /** Printed chapter name. Canonical here; book-generation.json derives it. */
  title: string;
  /** LaTeX label, e.g. "ch:first-words". Canonical here. */
  label: string;
  /** One first-person sentence, in the reader's terms. */
  canDo: string;
  /** Shared spine nodes this chapter realizes; may be empty for local-only work. */
  spineNodes: string[];
  payoff: ChapterPayoff;
  /** Optional HL06 figure ids. */
  figures?: string[];
}

/** One track's chapter capability ledger (`<track>/chapters.json`). */
export interface TrackChapters {
  version: number;
  language: string;
  chapters: ChapterCapability[];
}

/**
 * Tunable policy for the HL05 payoff rule and the HL08 gentle-ramp budgets.
 *
 * These live in `core/chapter-policy.json` rather than as constants at a call site
 * precisely so they can be tightened as the corpus matures without hunting through
 * code — the same reasoning that put the five-minute budget in the lesson schema.
 */
/**
 * HL10 section 5.1 -- one individually teachable slot of a verb paradigm.
 *
 * Language-neutral by contract. No id or gloss in the universal inventory may
 * name a form from any particular language, because the other 21 tracks fill
 * these same slots; a track lacking one declares an omission rather than
 * leaving a hole.
 */
export interface GrammarSlot {
  id: string;
  kind: "finite" | "imperative" | "compound" | "non-finite";
  gloss: string;
  mood?: string;
  tense?: string;
  person?: string;
  conjugation?: string;
  polarity?: string;
  form?: string;
}

export interface GrammarSlotInventory {
  version: number;
  note?: string;
  counts?: Record<string, number>;
  dimensions?: Record<string, string[]>;
  slots: GrammarSlot[];
}

/** One language's filling of a universal slot, plus where it sits in the ramp. */
export interface GrammarCell {
  id: string;
  slot: string;
  /**
   * Cells that must be taught first. This is what makes the inventory a ramp
   * rather than a list: singular before plural one person at a time, and the
   * present subjunctive behind the present indicative 1SG its stem comes from.
   */
  prerequisites: string[];
  conjugationEnding?: string;
  spanishName?: string;
  /** False where the language keeps a form only for recognition. */
  productive?: boolean;
  receptiveOnlyBecause?: string;
}

/**
 * One verb's one cell that deviates from the regular pattern.
 *
 * Kept separate from the regular cells because a learner never meets "the
 * irregular verbs" as a category. They meet the regular row, then the one verb
 * that breaks it, one cell at a time -- so every overlay hangs off the regular
 * cell it deviates from and the DAG gains depth rather than breadth.
 */
export interface GrammarCellOverlay {
  id: string;
  verb: string;
  kind: string;
  conjugation: string;
  /** The regular cell this one breaks. Always a real id in `cells`. */
  deviatesFrom: string;
  prerequisites: string[];
  note: string;
}

export interface TrackGrammarCells {
  version: number;
  language: string;
  note?: string;
  conjugationClasses?: Record<string, string>;
  counts?: Record<string, unknown>;
  cells: GrammarCell[];
  overlays?: GrammarCellOverlay[];
}

/** HL10 section 7.5 -- one word for talking about language, and when it is earned. */
export interface MetalanguageTerm {
  id: string;
  term: string;
  stage: string;
  order: number;
  /** What the learner must already be able to DO before the term is named. */
  introduceAfter: string;
  /**
   * False for a word an adult reader already owns -- word, sound, past, plural.
   * True is the actionable set, and deliberately includes noun/verb/adjective:
   * for a reader who never studied grammar, "a doing word" lands and "verb"
   * does not.
   */
  technical?: boolean;
  /**
   * What a lesson must say instead, until the term is introduced.
   *
   * The field that makes the rule actionable rather than merely restrictive: a
   * gate can tell an author what to write, not only what not to.
   */
  plainAlternative: string;
}

export interface MetalanguageInventory {
  version: number;
  note?: string;
  measuredAt?: string;
  corpusFinding?: string;
  universality?: string;
  stages?: string[];
  counts?: Record<string, number>;
  terms: MetalanguageTerm[];
}

export interface ChapterPolicy {
  version: number;
  /** Minimum share of a chapter's introduced atoms its payoff must assess (0..1). */
  payoffRepresentativeness: number;
  /** HL08: most knowledge atoms one lesson may introduce. */
  maxNewAtomsPerLesson: number;
  /** HL08: most a whole chapter may introduce, so splitting cannot game the rule. */
  maxNewAtomsPerChapter: number;
  /**
   * HL08: widest Markdown table the narration export will read aloud.
   *
   * A table beyond this width marks its lesson `sight`, so raising or lowering this
   * number moves lessons in and out of the drivable course. Optional so that a policy
   * file written before the lineariser existed still loads, falling back to the
   * lineariser's own measured default.
   */
  maxLinearisableTableColumns?: number;
  /**
   * HL10 section 2.2: the per-strand ceilings.
   *
   * All optional, so a policy file written before HL10 still loads and the gates
   * that read them simply do not run. Each measures a burden the atom budget
   * cannot see.
   */
  /** Most paradigm CELLS one lesson may introduce. A cell is `hablo`, not the six-form table. */
  maxNewGrammarCellsPerLesson?: number;
  maxNewIdiomsPerLesson?: number;
  /** Polysemy is not vocabulary: each sense of `quedar` is its own atom and its own lesson. */
  maxNewSensesPerLesson?: number;
  maxNewCultureClaimsPerLesson?: number;
  /** The info-dump gate. One rule statement per lesson. */
  maxRuleStatementsPerLesson?: number;
  /** No dead ends: an introduced atom some later lesson never requires or practises. */
  minDownstreamReach?: number;
  /** An etymon must be cashed in by at least this many later lessons to earn its place. */
  rootLedgerMinReuse?: number;
  /**
   * HL08: most NEW target-script glyphs one lesson may put in front of the reader.
   *
   * Separate from `maxNewAtomsPerLesson` because the two measure different burdens and
   * a lesson can pass one while failing the other badly: `HI-W01-shirorekha-na-ma`
   * declares ONE atom and shows TWELVE new Devanagari glyphs. Vocabulary is what a
   * learner must mean; script is what they must decode before meaning starts.
   *
   * Optional so a policy file written before the script ramp existed still loads.
   */
  maxNewGlyphsPerLesson?: number;
  /**
   * HL08: most distinct WRITING SYSTEMS one lesson may open at once.
   *
   * One, by the project owner's rule: "sometimes you can't introduce more than one
   * script at a time." This only bites where a track's script is genuinely plural —
   * Japanese, whose hiragana/katakana/kanji share one `script` id today.
   */
  maxNewScriptSystemsPerLesson?: number;
}

/** One authored chapter from an existing LaTeX book. */
export interface BookChapter {
  language: string;
  chapter: number;
  slug: string;
  title: string;
  /** Curriculum-root-relative path, suitable for provenance and diagnostics. */
  source: string;
  /** Original LaTeX. Kept losslessly so future renderers do not scrape PDFs. */
  tex: string;
}

/** The authored LaTeX layer that surrounds and sequences the short lessons. */
export interface LanguageBook {
  language: string;
  entrypoint: string;
  chapters: BookChapter[];
}

export interface BookCorpus {
  books: LanguageBook[];
}

// ---- Typed lesson body (HL04 schema v2) ----------------------------------

/**
 * The stable vocabulary of lesson-body sections.
 *
 * `writing` is the one member that is **detachable**: it teaches the hand and
 * nothing later in the lesson depends on it, so an output view that cannot use a
 * hand (the future dictation edition — see HL08) may skip exactly that section
 * and still deliver the rest of the lesson. Every other block type is load-bearing
 * prose. Detachability is why modality is derived per block and not only per
 * lesson; see `modality.ts`.
 *
 * The book prints a `writing` block like any other section. Detachable means
 * "a non-visual renderer may set this aside", never "this is optional content".
 */
export type LessonBlockType =
  | "warmup"
  | "input"
  | "notice"
  | "pronunciation"
  | "script"
  | "writing"
  | "etymology"
  | "grammar"
  | "culture-pragmatics"
  | "guided-production"
  | "comprehension"
  | "fluency"
  | "recall"
  | "unknown";

/** One named substitution point in a productive `pattern` lesson. */
export interface LessonPatternSlot {
  /** Author-order is significant: renderers introduce slots in this order. */
  name: string;
  /** Knowledge atoms the reader may substitute without learning anything new. */
  fillers: string[];
}

/**
 * Knowledge made available or assessed at one lesson-body boundary.
 *
 * Schema-v2 authors place this metadata in the first line after a level-two
 * heading. Keeping it beside the prose lets the validator follow the same
 * order that the book and app render instead of treating every lesson-level
 * introduction as if it were available from the opening sentence.
 */
export interface LessonBlockKnowledge {
  introduces: string[];
  assesses: string[];
}

/** The first executable activity contract supported by HL-V03. */
export type LessonActivityKind = "text";

export interface LessonActivityFeedback {
  correct: string;
  incorrect: string;
}

/**
 * One authored retrieval activity attached to a typed lesson-body block.
 *
 * Authors store this as compact JSON in an `hl-activity` comment immediately
 * after the block's `hl-knowledge` directive. The parser removes the comment
 * from learner copy while keeping this typed value in the canonical AST.
 */
export interface LessonActivity {
  id: string;
  kind: LessonActivityKind;
  /** Non-empty subset of the containing block's assessed knowledge atoms. */
  assesses: string[];
  prompt: string;
  /** Canonical display answer. */
  answer: string;
  /** Additional authored responses accepted after deterministic normalization. */
  accepted: string[];
  feedback: LessonActivityFeedback;
  responseSeconds: number;
}

/** A runtime-ready activity with every accepted response resolved up front. */
export interface CompiledLessonActivity extends LessonActivity {
  blockIndex: number;
  blockType: LessonBlockType;
  blockTitle: string;
  /** Canonical answer followed by authored variants, all normalized and unique. */
  acceptedResponses: string[];
}

export interface LessonBodyBlock {
  type: LessonBlockType;
  /** Original level-two heading, kept for book/app presentation. */
  title: string;
  /** Display Markdown between headings, excluding the parsed metadata directive. */
  markdown: string;
  /** Authored `hl-knowledge` directive, omitted by legacy lesson bodies. */
  knowledge?: LessonBlockKnowledge;
  /** Present when an author attempted a directive whose shape is invalid. */
  knowledgeDirectiveError?: string;
  /** Ordered executable retrieval contracts authored at this block boundary. */
  activities?: LessonActivity[];
  /** Present when one or more `hl-activity` directives are invalid or misplaced. */
  activityDirectiveErrors?: string[];
  /** One explicit HL16 writing-stage claim for this evidence block. */
  writingStage?: string;
  /** Present when an author attempted a malformed or misplaced stage directive. */
  writingStageDirectiveError?: string;
}

// ---- Script / character-breakdown data (data/scripts/<script>.json) ----
//
// Deliberately GENERAL, so one schema can teach any writing system. It has to
// describe three structurally different families without special-casing them:
//
//   • alphabet  (latin, greek, cyrillic)   — letters spell vowels + consonants.
//   • abugida   (devanagari, telugu, …)     — consonants carry an inherent vowel;
//                                             a vowel *mark* changes it; consonants
//                                             stack into conjuncts.
//   • abjad     (arabic, hebrew)            — consonants only; vowels are optional
//                                             diacritic *marks*; written right-to-
//                                             left; letters take contextual FORMS.
//
// The shared vocabulary: a script has DIRECTION, a SYSTEM, a set of LETTERS (each
// optionally with positional FORMS), and a set of MARKS (vowel signs OR harakat/
// niqqud). Every letter/mark carries the component decomposition + typical stroke
// order that is the whole point — "learn to write it, piece by piece."
//
// A fourth family, LOGOGRAPHIC (Chinese Hanzi), fits the same model without any
// special-casing: a "letter" is a character or a radical, its `components` are the
// radicals/strokes it's built from, `strokeOrder` is well-defined (and for Chinese
// actually authoritative), and its `sound` carries tone-marked pinyin (with an
// optional structured `tone`). No forms, no marks — those fields are simply omitted.

/** Which way the script runs. Abjads (Arabic, Hebrew) are `rtl`. */
export type Direction = "ltr" | "rtl";

/**
 * The structural family. Open string — the union members are hints, not a limit,
 * so a system we haven't named yet still validates.
 */
export type WritingSystem =
  | "alphabet"
  | "abugida"
  | "abjad"
  | "logographic"
  | "syllabary"
  | string;

/** A letter's role within its system. Open string (logographs, radicals, …). */
export type LetterRole =
  | "consonant"
  | "vowel"
  | "independent-vowel" // abugida word-initial vowels
  | "letter" // alphabet letters with no vowel/consonant split needed
  | "logograph" // a whole-word/morpheme character (Chinese)
  | "radical" // a recurring component of logographs
  | "other"
  | string;

/** Contextual letter forms, for cursive/abjad scripts (Arabic). */
export interface LetterForms {
  isolated?: string;
  initial?: string;
  medial?: string;
  final?: string;
}

/**
 * Where a letter's `strokeOrder` came from.
 *
 * A stroke order cannot be read out of a font — no font table records which way
 * the hand moved — so an order that is more than "the parts, roughly in order"
 * has to trace to a real teaching source. `variation` is not optional politeness:
 * for scripts with no national standard (every Indic script, Arabic, Hebrew) it
 * is the honest statement that this is ONE attested order, not THE order.
 */
export interface StrokeOrderSource {
  citation: string;
  url: string;
  /** How standardised the order is, and where it varies. */
  variation?: string;
}

/** One base letter of the script (a letter, a character, or a radical). */
export interface Letter {
  glyph: string; // the citation form
  sound: string; // romanization / phonetic value (tone-marked pinyin for Chinese)
  role: LetterRole;
  /** Tonal languages (Mandarin, Vietnamese, …): the canonical tone, e.g. "1".."4" | "neutral". */
  tone?: string;
  /** Abugida: the vowel a bare consonant carries (e.g. Devanagari "a"). */
  inherentVowel?: string;
  /** Cursive/abjad scripts: how the letter looks by position in a word. */
  forms?: LetterForms;
  /** The literal "pieces" of the character, for paper practice. */
  components: string[];
  /**
   * Typical handwriting order — conventional, not canonical.
   *
   * CAREFUL: this is a list of the letter's PARTS in writing order. It is NOT a
   * count of pen-down runs. Three named parts can be three separate strokes, or
   * one continuous stroke whose parts merely have names — Tamil ம is the latter.
   * Never let the wording (or the count) imply a pen lift that no authored pen
   * path supports; say "without lifting" explicitly where a path proves it.
   */
  strokeOrder: string[];
  strokeOrderNote: string;
  /**
   * How many times the pen leaves the paper, when a verified pen path says so.
   * Absent means "not verified" — which is not the same as "none", and must not
   * be inferred from `strokeOrder.length`.
   */
  penLifts?: number;
  /**
   * Provenance of the stroke ORDER, where the order is claimed rather than
   * sketched. Required whenever `penLifts` is present; without both fields the
   * UI labels `strokeOrder` as part order only and makes no lift claim.
   */
  strokeOrderSource?: StrokeOrderSource;
  notes?: string;
}

/**
 * A mandatory joined shape for an underlying sequence of existing letters.
 *
 * `sequence` preserves the text learners type and edit. `displayGlyph` is only
 * the precomposed presentation-form outline used to verify the authored pen
 * path; it must never replace the underlying letters in curriculum text.
 */
export interface Ligature {
  sequence: string;
  displayGlyph: string;
  sound: string;
  role: LetterRole;
  forms: LetterForms;
  components: string[];
  strokeOrder: string[];
  strokeOrderNote: string;
  penLifts?: number;
  strokeOrderSource?: StrokeOrderSource;
  notes?: string;
}

/**
 * A vowel sign / diacritic that attaches to a letter — an abugida mātrā, or the
 * harakat/niqqud of an abjad. `null`-free: alphabets simply have no marks.
 */
export interface Mark {
  mark: string;
  sound: string;
  role: "vowel-sign" | "diacritic" | "nasal" | "virama" | "other";
  attachesAs: string;
  example?: { base: string; combined: string; sound: string };
  /** Multiple carrier examples when one combining sign composes several glyphs. */
  examples?: Array<{
    base: string;
    combined: string;
    sound: string;
    carrier: string;
    note?: string;
  }>;
  /** Source-backed order for writing a carrier and then this attached sign. */
  compositionOrder?: string[];
  compositionSource?: StrokeOrderSource;
}

/**
 * One tone of a tonal language's inventory.
 *
 * `Letter.tone` above records which tone a *character* carries. That is enough to
 * label a glyph and nothing else: it cannot say what tone 3 sounds like, and it
 * cannot say that tone is lexical at all. Those are properties of the sound system,
 * not of any one character, so they live here beside `letters` rather than inside
 * them.
 *
 * This is the one place the model genuinely had to grow for Mandarin. Every
 * previously taught script encodes its pronunciation facts *segmentally* — a letter,
 * a vowel sign, a diacritic — and a segment is always attached to a glyph. Tone is
 * suprasegmental: it rides on a whole syllable, it is phonemic (`mā` "mother" and
 * `mà` "scold" are different words), and it can be changed by the neighbouring
 * syllable without changing the spelling at all. None of that fits in `Letter`.
 */
export interface Tone {
  /** Stable id, referenced from a lesson's `sounds:` list. */
  id: string;
  /** Conventional number: "1".."4" for Mandarin, plus "neutral". */
  tone: string;
  name: string;
  /** The pinyin diacritic, or "" for the unmarked neutral tone. */
  mark: string;
  /** Chao pitch letters, e.g. "55", "35", "214", "51". */
  contour: string;
  description: string;
  /** A romanized syllable a learner can hear the tone on, without needing a glyph. */
  exampleSyllable?: string;
  exampleGloss?: string;
}

/**
 * A rule that changes a tone in context without changing the writing.
 *
 * Segmental scripts have nothing structurally like this. The closest analogue in
 * this corpus — Arabic's sun-letter assimilation — still surfaces in the spoken
 * form of a *written* sequence; third-tone sandhi changes the pitch of a syllable
 * purely because of what follows it, and the pinyin a dictionary prints is the
 * unchanged citation tone. A learner told only the citation tones will say the
 * commonest greeting in the language wrong, so the rule has to be data, not prose.
 */
export interface ToneSandhiRule {
  id: string;
  description: string;
  /** Citation form, as a dictionary prints it. */
  citation: string;
  /** What a speaker actually says. */
  spoken: string;
}

export interface ScriptData {
  script: Script; // the id, matches the filename
  name: string; // human name, e.g. "Devanagari"
  font: string; // vendored Noto path
  direction: Direction;
  system: WritingSystem;
  letters: Letter[];
  /** Obligatory contextual shapes composed from existing letters, not new rows. */
  ligatures?: Ligature[];
  marks?: Mark[];
  /** Tonal languages only: the tone inventory the `Letter.tone` numbers index into. */
  tones?: Tone[];
  /** Tonal languages only: context rules that change a tone without changing spelling. */
  toneSandhi?: ToneSandhiRule[];
  /** How letters combine: abugida conjuncts, Arabic joining, ligatures, etc. */
  combination?: string;
  /** Set true when the inventory is complete enough to enforce glyph coverage. */
  complete?: boolean;
  notes?: string;
}

// ---- Validation ----

export interface Issue {
  level: "error" | "warning" | "info";
  code: string;
  message: string;
  lessonId?: string;
}
