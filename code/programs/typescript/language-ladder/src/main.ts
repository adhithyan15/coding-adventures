// main.ts — the thin DOM shell. It wires the pure view-models from core.ts and
// drill.ts to the page. Two modes:
//   • Browse   — a grid of letters + a "break it apart / write it" detail panel.
//   • Practice — a recall drill: see a sound, pick the glyph, get scored.
//
// Deliberately framework-free vanilla DOM. All the interesting logic lives in
// core.ts / drill.ts (and is unit-tested there); the ONLY randomness lives here,
// in the UI, so the pure modules stay deterministic and testable.

import { SCRIPTS, verifiedLetterFont } from "@coding-adventures/script-ductus";
import {
  buildScriptView,
  scriptSummary,
  handwritingHeading,
  type LetterView,
  type ScriptSummary,
} from "./core.ts";
import {
  buildDrillQuestion,
  checkAnswer,
  record,
  accuracy,
  emptyScore,
  type DrillQuestion,
  type Score,
} from "./drill.ts";
import {
  initStates,
  pickNext,
  reviewIn,
  masteredCount,
  type ItemState,
} from "./scheduler.ts";
import { buildPool, type PoolEntry } from "./interleave.ts";
import {
  isSyllabary,
  consonantGroups,
  unlockedConsonantCount,
  unlockedLetterIndices,
} from "./syllabary.ts";
import { buildSyllableMatrix } from "./matrix.ts";
import { crossScriptSiblings, type Sibling } from "./siblings.ts";
import type { Letter } from "./types.ts";
import {
  bundledLessonIds,
  indicesByLanguage,
  loadBundledLessons,
  nextDue,
  type Lesson,
} from "./lessons.ts";
import {
  applyAnswer,
  type Progress,
} from "./sessionplan.ts";
import { pickNext as pickReviewCell, makeRng, cellKey, type GridCell } from "./quiz.ts";
import { confusions } from "./mistakes.ts";
import { loadReview, saveReview } from "./reviewstore.ts";
import { clearProgress, removableStorage } from "./reset.ts";
import {
  scriptsById,
  scriptOf,
  type ScriptIntro,
} from "./scriptintro.ts";
import { LANGUAGE_CHAIN } from "./sequence.ts";
import type { SessionStep } from "./session.ts";
import {
  crossLanguageConcepts,
  datasetFromLessons,
  unlockedIndices,
  type ConceptCard,
} from "./concepts.ts";
import {
  LANGUAGE_CURRICULA,
  LANGUAGE_REGISTRY,
  MAPPED_LANGUAGE_IDS,
  SPINE_CONCEPTS,
  curriculumForLanguage,
  languageName,
  loadCurriculumPlans,
  mappedLessonIds,
  mixedCurriculumFrontier,
  spineNodeById,
} from "./curriculum.ts";
import {
  completeFrontierLesson,
  eligibleReviewGrid,
  loadLearnProgress,
  localPathProgress,
  mixedReviewReady,
  saveLearnProgress,
  type LearnCompletion,
} from "./learnprogress.ts";
import {
  activityAnswerIsCorrect,
  focusedActivity,
  focusedCheckKind,
  meaningAnswerIsCorrect,
} from "./focused.ts";
import { loadLanguages, saveLanguages } from "./languagestore.ts";
import { lessonSections } from "./lessonbody.ts";
import { generatedFigureUrl } from "./figures.ts";
import { bookHashStatus, whenBookHashesReady } from "./bookhashes.ts";
// Per-atom mastery (HL10 §10.1). The scheduler still runs on lessons; this
// records what the learner actually holds, atom by atom, so a later slice can
// schedule from it. Recording first and scheduling second is deliberate: the
// record has to exist and be trustworthy before anything is allowed to depend
// on it.
import { practiseAll } from "./atommastery.ts";
import { type ReviewPick, refreshesOf, reviewPicks } from "./atomschedule.ts";
import { type SynthesisDrill, piecesUsed, synthesisDrill } from "./synthesisdrill.ts";
import { buildVoiceScript, type NarrationLesson } from "./voicescript.ts";
import { type VoiceHandle, browserSpeech, playVoiceScript } from "./voiceplayer.ts";
import { loadNarration } from "./narration-sources.ts";
import { browserStorage as masteryStorage, loadMastery, saveMastery } from "./masterystore.ts";
import { parseFont, boundsOf, type Font } from "@coding-adventures/script-ductus";
import {
  ductusFilmstrip,
  ductusFor,
  isSafeName,
  type SvgNode,
} from "@coding-adventures/script-ductus";
import tamilFontUrl from "../../../../learning/human-languages/_fonts/NotoSansTamil-Static.ttf?url";
import naskhFontUrl from "../../../../learning/human-languages/_fonts/NotoNaskhArabic-Static.ttf?url";
import taxonomyJson from "../../../../learning/human-languages/concepts/taxonomy.json";
import type { Taxonomy } from "@coding-adventures/human-language-data/src/types.ts";
import {
  browserStorage,
  emptyProgress,
  fromSaved,
  loadProgress,
  saveProgress,
  toSaved,
} from "./progress.ts";
import "./styles.css";

const app = document.getElementById("app");
if (!app) throw new Error("missing #app root");

type Mode = "learn" | "browse" | "practice" | "lessons" | "concepts";
let mode: Mode = "learn";

// --- the curriculum session (HL03 phase 6) ----------------------------------
//
// The whole app walks each selected language's authored local path. Progress is
// independent: a correct focused check advances only that language and makes
// only that lesson eligible for mixed review. The shared spine groups frontiers
// that happen to be ready together; it never overrides local prerequisites.

// --- lesson review state ----------------------------------------------------
//
// The letter drills above schedule GLYPHS. This schedules LESSONS — the ~670
// written chapters — using the very same Leitner machinery, because
// scheduler.ts is generic over a numeric index and never cared what an item is.
// The one new thing is that this state SURVIVES: it is keyed by lesson id and
// written to localStorage (see progress.ts), so the app finally remembers you.
const REVIEW_STORAGE = browserStorage();
// Filled on the first corpus load. It cannot be built at module load any more:
// the id list comes from the lesson-source map, which is deliberately lazy so
// its ~27 kB of paths stay out of the eager chunk (HL-C110).
let BUNDLED_LESSON_IDS: Set<string> | null = null;

// The learner's per-atom record, loaded once and written back on every answer.
// Kept beside the other stores rather than inside them: this is a third,
// genuinely different thing (see masterystore.ts).
let MASTERY = loadMastery(masteryStorage());

/**
 * Credit one answer against every atom it assessed.
 *
 * `assesses` is authored per activity, so this is the exact set the learner was
 * just tested on — not the whole lesson, which would credit atoms they never
 * had to produce.
 */
function recordAtomAnswer(atoms: readonly string[] | undefined, correct: boolean): void {
  if (!atoms || atoms.length === 0) return;
  MASTERY = practiseAll(MASTERY, atoms, correct, Date.now());
  saveMastery(masteryStorage(), MASTERY);
}
let LESSONS: Lesson[] = [];
let LESSON_IDS: string[] = [];
// Which tracks are offered is a question about which plans EXIST, not about
// what they contain, so it is answered from the plan file list (see
// MAPPED_LANGUAGE_IDS) and stays synchronous even though the plans themselves
// arrive with the corpus.
const AVAILABLE_LANGUAGE_IDS = LANGUAGE_CHAIN.filter((language) =>
  MAPPED_LANGUAGE_IDS.includes(language),
);
let selectedLanguages = loadLanguages(REVIEW_STORAGE, AVAILABLE_LANGUAGE_IDS);
// Consolidation lessons — chapter practice, mixed drills, dialogues, reviews —
// are not atomic concepts: their headword is a placeholder ("(practice)"), they
// carry no roots, and they exist to REVISIT earlier lessons (`reviews_of`).
// That kind of consolidation is exactly what the review quiz is for, so keep
// these out of the teaching spine — the learner should walk real words and
// grammar, one concept at a time, not land on "(practice)".
const CONSOLIDATION_TYPES = new Set(["practice", "practice-mix", "review"]);
let CONCEPT_LESSONS: Lesson[] = [];
const SHARED_CONCEPTS = new Set(SPINE_CONCEPTS);
// Filled by `installCurriculumPlans()` once the lazy plans land, and BEFORE any
// lesson is installed — `installLessons` is only ever reached through
// `refreshCorpus`, which awaits the plans first.
let ALL_MAPPED_LESSON_IDS = new Set<string>();
// Learn mode is now admitted by the explicit per-track maps. Namespaced and
// not-yet-mapped legacy material remains available in Lessons mode.
let MAPPED_SPINE_LESSONS: Lesson[] = [];

// Script metadata is indexed once. A local map's explicit script extension,
// rather than a global concept position, decides where its introduction appears.
const SCRIPTS_BY_ID = scriptsById(SCRIPTS);

// --- the Learn-mode review quiz (HL03 phase 6, slice 6b-2) ------------------
//
// The second of the app's two mechanisms. The teaching sweep (6b-1) walks the
// curriculum forward; this quizzes BACKWARD over everything covered so far — a
// randomised, SRS-weighted draw across the (concept × language) grid, so what
// you keep missing resurfaces and what you have mastered fades. The draw and the
// state math live in the tested engine (quiz.ts `pickNext`, sessionplan.ts
// `applyAnswer`); this only presents a question and threads the answer back.
//
// A cell is asked as "<meaning> — in <language>?" and the options are the SAME
// concept in OTHER languages (plus the answer): the cross-language look-alikes
// the interleaving is meant to expose (mixing up merci and mercy, dhanya across
// the Dravidian languages). The confusion the learner actually makes — which
// wrong word they picked — is logged and surfaced in "what I keep confusing".
// (Option count reuses the practice-mode OPTION_COUNT, defined below.)
//
// Look up a lesson's word by its cell so a logged confusion (stored as a cellKey)
// can be shown as the actual word, not an opaque id.
let LESSON_BY_ID = new Map<string, Lesson>();
// Restore the review's SRS state + answer log from localStorage so the quiz
// remembers you between visits (reusing the same storage port progress.ts owns).
// A missing, corrupt, or wrong-version blob restores as empty — never throws.
const restoredReview = loadReview(REVIEW_STORAGE);
let reviewProgress: Progress = restoredReview.progress;
let reviewSession = restoredReview.session; // advances once per answered question — the SRS clock
let reviewCell: GridCell | null = null; // the question currently on screen
let reviewOptions: GridCell[] = []; // its answer options (one is `reviewCell`)
let reviewChosen: string | null = null; // cellKey of the picked option; null = unanswered
// Restored by `installCurriculumPlans()`: what counts as "completed" is pruned
// against the authored paths (see learnprogress.ts), so it cannot be restored
// before those paths are in hand. Empty until then, which is exactly the state
// the loading frame renders.
let learnCompletion: LearnCompletion = new Map();
type FocusedAttempt = { lessonId: string; state: "check" | "wrong" | "correct" };
let focusedAttempt: FocusedAttempt | null = null;
let learnNotice: string | null = null;

// "Reset progress" is a two-click confirm: the first click ARMS it (so a stray
// tap can't wipe everything), the second executes. This flag is that arming.
let resetArmed = false;

// Constant for the page's lifetime: lesson indices grouped by language, and the
// round-robin pool over those groups. Computing them once is why consecutive
// reviews can walk across languages cheaply.
let LESSON_GROUPS: number[][] = [];
let LESSON_POOL: PoolEntry[] = [];

// Cross-language cards: one concept, several languages. Built once — the join
// walks every lesson, and neither the curriculum nor the taxonomy changes while
// the page is open.
let CONCEPT_CARDS: ConceptCard[] = [];
/** Which concept card is expanded; null = none. */
let openConcept: string | null = null;
let savedProgress = loadProgress(browserStorage());
let lessonSchedule: ItemState[] = fromSaved(LESSON_IDS, savedProgress);
let lessonSession = savedProgress.session;
let lessonIndex: number | null = null;
let lessonRevealed = false;
/** Rotating position in the interleaved order — see pickLesson(). */
let lessonCursor = -1;
let fullCorpusLoaded = false;
let corpusLoading = true;
let corpusError: string | null = null;

/** Rebuild every derived lesson index after a lazy corpus tranche arrives. */
function installLessons(incoming: readonly Lesson[]): void {
  const merged = new Map(LESSONS.map((lesson) => [lesson.id, lesson]));
  for (const lesson of incoming) merged.set(lesson.id, lesson);
  LESSONS = [...merged.values()].sort((a, b) => a.id.localeCompare(b.id));
  LESSON_IDS = LESSONS.map((lesson) => lesson.id);
  CONCEPT_LESSONS = LESSONS.filter((lesson) => !CONSOLIDATION_TYPES.has(lesson.type));
  MAPPED_SPINE_LESSONS = CONCEPT_LESSONS.filter(
    (lesson) => ALL_MAPPED_LESSON_IDS.has(lesson.id) && SHARED_CONCEPTS.has(lesson.concept),
  );
  LESSON_BY_ID = new Map(LESSONS.map((lesson) => [lesson.id, lesson]));
  LESSON_GROUPS = indicesByLanguage(LESSONS);
  LESSON_POOL = buildPool(LESSON_GROUPS.map((group) => group.length));
  CONCEPT_CARDS = crossLanguageConcepts(
    datasetFromLessons(taxonomyJson as unknown as Taxonomy, LESSONS),
  );
  lessonSchedule = fromSaved(LESSON_IDS, savedProgress);
  lessonIndex = null;
  lessonCursor = -1;
}

/** Current Learn mode needs completed material plus one frontier per path. */
function learnLessonIds(): Set<string> {
  const ids = new Set<string>();
  for (const completed of learnCompletion.values()) {
    for (const id of completed) ids.add(id);
  }
  for (const step of mixedCurriculumFrontier(selectedLanguages, learnCompletion).steps) {
    ids.add(step.lessonId);
  }
  return ids;
}

/** Guards the one-time restore below: a later refresh must not re-read storage
 * over progress the learner has made since. */
let learnProgressRestored = false;

/**
 * Take delivery of the lazily-fetched per-track plans.
 *
 * Everything the plans decide is derived here, in one place, so there is a
 * single moment after which the app's plan-dependent state is real: the set of
 * lesson ids the shared spine admits, and the restored Learn progress (which is
 * pruned against those very paths, so it cannot be restored any earlier).
 *
 * Idempotent — `loadCurriculumPlans()` memoises the fetch, and recomputing from
 * the same plans yields the same answers, so a corpus refresh may call it again
 * without disturbing anything.
 */
async function installCurriculumPlans(): Promise<void> {
  await loadCurriculumPlans();
  ALL_MAPPED_LESSON_IDS = mappedLessonIds(LANGUAGE_CURRICULA.map((item) => item.language));
  if (!learnProgressRestored) {
    learnCompletion = loadLearnProgress(REVIEW_STORAGE, LANGUAGE_CURRICULA);
    learnProgressRestored = true;
  }
}

async function loadLearnCorpus(): Promise<void> {
  await installCurriculumPlans();
  BUNDLED_LESSON_IDS ??= new Set(await bundledLessonIds());
  const known = BUNDLED_LESSON_IDS;
  const missing = [...learnLessonIds()].filter(
    (id) => known.has(id) && !LESSON_BY_ID.has(id),
  );
  if (missing.length > 0) installLessons(await loadBundledLessons(missing));
}

async function loadFullCorpus(): Promise<void> {
  if (fullCorpusLoaded) return;
  // Lessons mode can be the FIRST thing a visitor opens, so it must not assume
  // the Learn path already pulled the plans in: `installLessons` indexes the
  // mapped-spine subset out of them.
  await installCurriculumPlans();
  installLessons(await loadBundledLessons());
  fullCorpusLoaded = true;
}

async function refreshCorpus(load: () => Promise<void>): Promise<void> {
  corpusLoading = true;
  corpusError = null;
  render();
  try {
    await load();
  } catch (error) {
    corpusError = error instanceof Error ? error.message : String(error);
  } finally {
    corpusLoading = false;
    render();
  }
}

/** Persist the current lesson schedule. Silent on failure — see progress.ts. */
function persistLessons(): void {
  savedProgress = toSaved(LESSON_IDS, lessonSchedule, lessonSession);
  saveProgress(browserStorage(), savedProgress);
}
let currentScript = 0;
let currentLetter = 0;
// Browse layout for the syllabaries: the flat "list" of tiles, or the
// consonant × vowel "matrix" that makes the abugida's regularity visible.
let browseLayout: "list" | "matrix" = "list";

// Practice state
type Scope = "script" | "mixed";
let scope: Scope = "script"; // drill the current script, or all scripts interleaved
let score: Score = emptyScore();
let question: DrillQuestion | null = null;
let chosen: number | null = null; // which option the learner picked (null = unanswered)
// Spaced-repetition state: the scheduler decides WHICH letter to ask next, so
// missed letters resurface sooner and mastered ones fade back. One session tick
// per answered question (see scheduler.ts). Rebuilt when the scope/script changes.
let schedule: ItemState[] = [];
let sessionTick = 0;
// In "mixed" scope, `schedule` indexes a combined pool spanning every script;
// `pool[i]` maps that index back to (scriptIndex, letterIndex). Empty in "script"
// scope, where the schedule index IS the letter index of the current script.
let pool: PoolEntry[] = [];
// Which script + schedule-index the CURRENT question belongs to (they diverge in
// mixed scope, where the schedule index is a pool index, not a letter index).
let questionScript = 0;
let scheduleIndex = 0;

const OPTION_COUNT = 4;

/** Resolve a schedule index to a concrete (script, letter), per scope. */
function resolve(idx: number): PoolEntry {
  if (scope === "mixed") return pool[idx] ?? { scriptIndex: 0, letterIndex: 0 };
  return { scriptIndex: currentScript, letterIndex: idx };
}

// The slow-unlock gate for the Dravidian syllabaries. Drilling 350 syllables at
// once is the opposite of learning to read; instead the drill opens ONE
// consonant's vowel row (ka kā ki … kō) and unlocks the next consonant only once
// the current row is mastered — the "ka, ki, ku … kha, khi, khu" build-up. Only
// active in "script" scope on a syllabary; null (no gating) everywhere else, so
// the alphabets and Mixed mode are untouched. In script scope the schedule index
// IS the letter index, so `schedule` lines up 1:1 with `letters`.
interface SyllabaryGate {
  indices: number[]; // the letter indices currently drillable
  set: Set<number>; // same, for O(1) distractor filtering
  unlocked: number; // how many consonants are open
  total: number; // how many consonants in all
}
function syllabaryGate(): SyllabaryGate | null {
  if (scope !== "script") return null;
  const letters = SCRIPTS[currentScript]!.letters;
  if (!isSyllabary(letters)) return null;
  const groups = consonantGroups(letters);
  const unlocked = unlockedConsonantCount(groups, schedule);
  const indices = unlockedLetterIndices(groups, unlocked);
  return { indices, set: new Set(indices), unlocked, total: groups.length };
}

// --- shared chrome ----------------------------------------------------------

const SUBTITLES: Record<Mode, string> = {
  learn:
    "Walk each language's local path one short lesson at a time, then mix only what you have independently unlocked.",
  browse:
    "Pick a script and a letter to see its pieces and stroke order — for pen-and-paper practice.",
  practice:
    "Recall drill: read the sound, pick the matching letter. Wrong answers are the confusable ones.",
  lessons: "Spaced review across the whole curriculum, interleaved by language.",
  concepts: "One idea, side by side, in every language that has it.",
};

function renderHeader(): HTMLElement {
  const header = el("header", "header");
  const h1 = el("h1", "");
  h1.textContent = "Language Ladder";
  const sub = el("p", "sub");
  sub.textContent = SUBTITLES[mode];
  header.append(h1, sub, renderModeToggle());
  return header;
}

function renderModeToggle(): HTMLElement {
  const wrap = el("div", "modes");
  const LABELS: Record<Mode, string> = {
    learn: "Learn",
    browse: "Browse",
    practice: "Practice",
    lessons: "Lessons",
    concepts: "Concepts",
  };
  (["learn", "browse", "practice", "lessons", "concepts"] as Mode[]).forEach((m) => {
    const b = el("button", "mode" + (m === mode ? " mode--active" : ""));
    b.textContent = LABELS[m];
    b.setAttribute("aria-pressed", String(m === mode));
    b.onclick = () => {
      if (mode === m) return;
      void activateMode(m);
    };
    wrap.appendChild(b);
  });
  return wrap;
}

async function activateMode(nextMode: Mode): Promise<void> {
  mode = nextMode;
  if ((mode === "lessons" || mode === "concepts") && !fullCorpusLoaded) {
    await refreshCorpus(loadFullCorpus);
  }
  if (mode === "practice") startPractice();
  if (mode === "lessons") pickLesson();
  render();
}

/** In Practice, choose per-script drilling or all scripts interleaved. */
function renderScopeToggle(): HTMLElement {
  const wrap = el("div", "scopes");
  const label = el("span", "scopes__label");
  label.textContent = "Practice:";
  wrap.appendChild(label);
  (
    [
      ["script", "This script"],
      ["mixed", "Mixed (all scripts)"],
    ] as [Scope, string][]
  ).forEach(([s, text]) => {
    const b = el("button", "scope" + (s === scope ? " scope--active" : ""));
    b.textContent = text;
    b.setAttribute("aria-pressed", String(s === scope));
    b.onclick = () => {
      if (scope === s) return;
      scope = s;
      startPractice();
      render();
    };
    wrap.appendChild(b);
  });
  return wrap;
}

/** A tab per script. Hidden while practising a mixed (all-scripts) session. */
function renderTabs(): HTMLElement {
  const tabs = el("div", "tabs");
  SCRIPTS.forEach((data, i) => {
    const s = scriptSummary(data);
    const b = el("button", "tab" + (i === currentScript ? " tab--active" : ""));
    b.textContent = s.name;
    b.setAttribute("aria-pressed", String(i === currentScript));
    b.onclick = () => {
      currentScript = i;
      currentLetter = 0;
      if (mode === "practice") startPractice();
      render();
    };
    tabs.appendChild(b);
  });
  return tabs;
}

// --- browse mode ------------------------------------------------------------

function renderSummary(s: ScriptSummary): HTMLElement {
  const box = el("div", "summary");
  box.appendChild(kv("System", s.system));
  box.appendChild(kv("Direction", s.direction === "rtl" ? "right-to-left" : "left-to-right"));
  box.appendChild(kv("Letters", String(s.letterCount)));
  if (s.falseFriendCount > 0) {
    box.appendChild(kv("False friends", `${s.falseFriendCount} (look Latin, aren't)`));
  }
  if (!s.complete) {
    box.appendChild(kv("Status", "inventory in progress"));
  }
  return box;
}

function renderGrid(views: LetterView[], dir: "ltr" | "rtl"): HTMLElement {
  const grid = el("div", "grid");
  grid.dir = dir;
  views.forEach((v, i) => {
    const tile = el(
      "button",
      "tile" +
        (i === currentLetter ? " tile--active" : "") +
        (v.falseFriend ? " tile--ff" : "") +
        (v.special ? " tile--special" : ""),
    );
    const glyph = el("span", "tile__glyph");
    glyph.textContent = v.glyph;
    const sound = el("span", "tile__sound");
    sound.textContent = bareSound(v.sound);
    tile.append(glyph, sound);
    tile.title = v.sound;
    tile.onclick = () => {
      currentLetter = i;
      render();
    };
    grid.appendChild(tile);
  });
  return grid;
}

/**
 * The independent (word-initial) vowels as a small read-only strip — the letters
 * a word writes when it BEGINS with a vowel (అ a, ఆ ā), distinct from the vowel
 * signs that ride on a consonant in the grid below. Recognition only; each tile
 * shows the glyph and its ISO-15919 romanization.
 */
function renderIndependentVowels(vowels: Letter[]): HTMLElement {
  const wrap = el("div", "ivowels");
  const label = el("span", "ivowels__label");
  label.textContent = "Independent vowels (word-initial):";
  wrap.appendChild(label);
  const row = el("div", "ivowels__row");
  vowels.forEach((v) => {
    const tile = el("div", "ivowel");
    const glyph = el("span", "ivowel__glyph");
    glyph.textContent = v.glyph;
    const sound = el("span", "ivowel__sound");
    sound.textContent = v.sound;
    tile.append(glyph, sound);
    tile.title = v.sound;
    row.appendChild(tile);
  });
  wrap.appendChild(row);
  return wrap;
}

/**
 * Atomic vowel-free final consonants (Malayalam chillus) as a separate strip.
 * They are real teaching letters, but not cells in the generated consonant ×
 * vowel matrix, so keeping them here preserves that grid's all-syllable shape.
 */
function renderFinalConsonants(consonants: Letter[]): HTMLElement {
  const wrap = el("div", "ivowels");
  const label = el("span", "ivowels__label");
  label.textContent = "Final consonants (chillus):";
  wrap.appendChild(label);
  const row = el("div", "ivowels__row");
  consonants.forEach((consonant) => {
    const tile = el("div", "ivowel");
    const glyph = el("span", "ivowel__glyph");
    glyph.textContent = consonant.glyph;
    const sound = el("span", "ivowel__sound");
    sound.textContent = consonant.sound;
    tile.append(glyph, sound);
    tile.title = consonant.sound;
    row.appendChild(tile);
  });
  wrap.appendChild(row);
  return wrap;
}

/**
 * The script's own numerals as a small read-only strip (౦౧౨… = 0–9). Reading a
 * language means reading its numbers, and these are distinct glyphs, not Western
 * 0-9. Recognition only; each tile shows the glyph and its value.
 */
function renderNumerals(digits: Letter[]): HTMLElement {
  const wrap = el("div", "ivowels");
  const label = el("span", "ivowels__label");
  label.textContent = "Numerals (0–9):";
  wrap.appendChild(label);
  const row = el("div", "ivowels__row");
  digits.forEach((d) => {
    const tile = el("div", "ivowel");
    const glyph = el("span", "ivowel__glyph");
    glyph.textContent = d.glyph;
    const value = el("span", "ivowel__sound");
    value.textContent = d.sound;
    tile.append(glyph, value);
    tile.title = d.sound;
    row.appendChild(tile);
  });
  wrap.appendChild(row);
  return wrap;
}

/** For a syllabary, a "List / Matrix" switch — the flat grid, or the table. */
function renderBrowseLayoutToggle(): HTMLElement {
  const wrap = el("div", "layouts");
  const label = el("span", "layouts__label");
  label.textContent = "Layout:";
  wrap.appendChild(label);
  (
    [
      ["list", "List"],
      ["matrix", "Matrix"],
    ] as ["list" | "matrix", string][]
  ).forEach(([l, text]) => {
    const b = el("button", "layout" + (l === browseLayout ? " layout--active" : ""));
    b.textContent = text;
    b.setAttribute("aria-pressed", String(l === browseLayout));
    b.onclick = () => {
      if (browseLayout === l) return;
      browseLayout = l;
      render();
    };
    wrap.appendChild(b);
  });
  return wrap;
}

/**
 * The consonant × vowel table for a syllabary. Rows are consonants, columns are
 * the shared vowels; each cell is the syllable glyph + its romanization, and
 * clicking it selects that syllable so the existing detail panel breaks it apart.
 * The layout comes from the pure `buildSyllableMatrix`, so nothing here invents
 * an alignment — a ragged script simply has no matrix to show.
 */
function renderMatrix(letters: Letter[]): HTMLElement | null {
  const m = buildSyllableMatrix(letters);
  if (!m) return null;

  const scroll = el("div", "matrix-scroll");
  const table = el("table", "matrix") as HTMLTableElement;

  const thead = el("thead", "");
  const hrow = el("tr", "");
  hrow.appendChild(el("th", "matrix__corner")); // top-left, above the row labels
  m.vowels.forEach((v) => {
    const th = el("th", "matrix__vowel");
    th.textContent = v;
    hrow.appendChild(th);
  });
  thead.appendChild(hrow);
  table.appendChild(thead);

  const tbody = el("tbody", "");
  m.rows.forEach((row) => {
    const tr = el("tr", "");
    const rh = el("th", "matrix__consonant" + (row.special ? " matrix__consonant--special" : ""));
    // Mark the retroflex/alveolar rows (ḷ/ṟ/ṉ) so the confusable ones stand out
    // in the full grid, the same rows the tiles flag as special consonants.
    rh.textContent = row.special ? `★ ${row.label}` : row.label;
    if (row.special) rh.title = "Special consonant — tell it apart from the ordinary letter";
    tr.appendChild(rh);
    row.cells.forEach((cell) => {
      const td = el("td", "matrix__cell" + (cell.index === currentLetter ? " matrix__cell--active" : ""));
      const btn = el("button", "matrix__syllable");
      const glyph = el("span", "matrix__glyph");
      glyph.textContent = cell.glyph;
      const sound = el("span", "matrix__sound");
      sound.textContent = bareSound(cell.sound);
      btn.append(glyph, sound);
      btn.title = cell.sound;
      btn.onclick = () => {
        currentLetter = cell.index;
        render();
      };
      td.appendChild(btn);
      tr.appendChild(td);
    });
    tbody.appendChild(tr);
  });
  table.appendChild(tbody);

  scroll.appendChild(table);
  return scroll;
}

function renderDetail(v: LetterView, script: string, siblings: Sibling[] = []): HTMLElement {
  const d = el("div", "detail");
  const head = el("div", "detail__head");
  const big = el("div", "detail__glyph");
  big.textContent = v.glyph;
  const meta = el("div", "detail__meta");
  const name = el("div", "detail__sound");
  name.textContent = v.sound;
  const role = el("div", "detail__role");
  role.textContent = [v.role, v.tone && `tone ${v.tone}`, v.inherentVowel && `inherent vowel “${v.inherentVowel}”`]
    .filter(Boolean)
    .join(" · ");
  meta.append(name, role);
  if (v.falseFriend) {
    const badge = el("span", "badge");
    badge.textContent = "⚠ false friend";
    meta.appendChild(badge);
  }
  if (v.special) {
    const badge = el("span", "badge badge--special");
    badge.textContent = `★ special consonant`;
    meta.appendChild(badge);
  }
  head.append(big, meta);
  d.appendChild(head);
  d.appendChild(section("Break it apart — the pieces", listOf(v.components, "pieces")));
  // The retroflex/alveolar special consonants (ḷ/ṟ/ṉ) — flag how they differ
  // from the plain letter they're most confused with, the way false friends are.
  if (v.special) {
    const p = el("p", "detail__special");
    p.textContent = v.special.hint;
    d.appendChild(section(`Special letter — tell it apart from “${v.special.plain}”`, p));
  }
  // The same syllable in the sibling Dravidian scripts — Telugu కి next to
  // Kannada ಕಿ next to Malayalam കി. Seeing the three shapes for one sound is
  // how the cousins bootstrap each other (see siblings.ts). Only appears when
  // there is at least one sibling, i.e. only for the syllabary trio.
  if (siblings.length > 0) {
    const strip = el("div", "siblings");
    for (const s of siblings) {
      const item = el("div", "sibling");
      const g = el("div", "sibling__glyph");
      g.textContent = s.glyph;
      const label = el("div", "sibling__label");
      label.textContent = s.name;
      item.append(g, label);
      strip.appendChild(item);
    }
    d.appendChild(section(`Same sound, sister scripts — “${v.sound}” elsewhere`, strip));
  }
  // Only offer stroke order when we actually have it. The Dravidian syllabaries
  // are recognition-only (their ductus is a separate, paused effort), so showing
  // an empty "Write it" section would imply data we don't have.
  // A handful of letters go further: an authored, font-validated PEN PATH, so
  // the section becomes a stroke-by-stroke build-up instead of a list.
  // (`DUCTUS` admits no letter without a cited source.) Every other letter falls
  // back to a prose PART list whose heading explicitly says that numbering does
  // not claim separate strokes or any particular lifts.
  if (ductusFor(v.glyph, script)) {
    d.appendChild(section(handwritingHeading(v, true), renderDuctusSection(v, script)));
  } else if (v.strokeOrder.length > 0) {
    d.appendChild(section(handwritingHeading(v, false), orderedListOf(v.strokeOrder)));
  }
  if (v.notes) {
    const note = el("p", "detail__notes");
    note.textContent = v.notes;
    d.appendChild(section("Notes", note));
  }
  return d;
}

// --- practice mode ----------------------------------------------------------

/** Start (or restart) a practice session, per scope. */
function startPractice(): void {
  score = emptyScore();
  sessionTick = 0;
  if (scope === "mixed") {
    pool = buildPool(SCRIPTS.map((s) => s.letters.length));
    schedule = initStates(pool.length);
  } else {
    pool = [];
    schedule = initStates(SCRIPTS[currentScript]!.letters.length);
  }
  nextQuestion();
}

/** Let the scheduler choose the next item; the UI still randomises options. */
function nextQuestion(): void {
  // The scheduler decides WHICH item (spaced repetition, interleaved in mixed
  // scope); randomness is only for the distractors + answer position.
  const gate = syllabaryGate();
  let idx: number;
  if (gate) {
    // Only ask about UNLOCKED syllables: run the scheduler over just that slice.
    // pickNext returns the picked item's real `letterIndex` (initStates seeds it
    // to the schedule position and reviewIn preserves it), and every item in the
    // slice is an unlocked letter — so the return value is already a real index
    // into the full letters/views, no re-mapping needed.
    const picked = pickNext(
      gate.indices.map((i) => schedule[i]!),
      sessionTick,
    );
    idx = picked < 0 ? gate.indices[0]! : picked;
  } else {
    idx = pickNext(schedule, sessionTick);
    if (idx < 0) idx = 0;
  }
  scheduleIndex = idx;
  const { scriptIndex, letterIndex } = resolve(scheduleIndex);
  questionScript = scriptIndex;
  const views = buildScriptView(SCRIPTS[scriptIndex]!);
  const placeAt = randInt(Math.min(OPTION_COUNT, views.length));
  // Distractors come from the target's OWN script, so a Cyrillic prompt never
  // offers a Hebrew decoy — and on a gated syllabary, only from UNLOCKED
  // syllables, so a not-yet-introduced consonant never appears as a decoy.
  const chooser = gate
    ? (ranked: number[], count: number) =>
        chooseConfusableShuffled(ranked.filter((i) => gate.set.has(i)), count)
    : chooseConfusableShuffled;
  question = buildDrillQuestion(views, letterIndex, OPTION_COUNT, chooser, placeAt);
  chosen = null;
}

function renderPractice(): HTMLElement {
  // Options + reveal come from the QUESTION's script (may differ from the tab in
  // mixed scope).
  const views = buildScriptView(SCRIPTS[questionScript]!);
  const q = question!;
  const wrap = el("div", "practice");
  wrap.dataset.script = SCRIPTS[questionScript]!.script;

  wrap.appendChild(renderScopeToggle());

  // Score line + a spaced-repetition mastery read-out. On a gated syllabary the
  // read-out is over the UNLOCKED syllables, not all 350 — otherwise "mastered
  // 10 / 350" would read as no progress when you've in fact finished the first
  // row.
  const gate = syllabaryGate();
  const acc = accuracy(score);
  const scoreState = gate ? gate.indices.map((i) => schedule[i]!) : schedule;
  const mastered = masteredCount(scoreState);
  const scoreLine = el("div", "score");
  const scoreText = acc === null ? "Score: 0 / 0" : `Score: ${score.correct} / ${score.total}  ·  ${acc}%`;
  scoreLine.textContent = `${scoreText}   ·   mastered ${mastered} / ${scoreState.length}`;
  wrap.appendChild(scoreLine);

  // The slow-unlock cue: which consonant you're on, and how to open the next.
  if (gate) {
    const cue = el("div", "syllabary-cue");
    cue.textContent = `Learning consonant ${gate.unlocked} of ${gate.total} — master this vowel row to unlock the next.`;
    wrap.appendChild(cue);
  }

  // Prompt
  const prompt = el("div", "prompt");
  const label = el("div", "prompt__label");
  label.textContent = "Which letter makes this sound?";
  const sound = el("div", "prompt__sound");
  sound.textContent = q.promptSound;
  prompt.append(label, sound);
  if (scope === "mixed") {
    const tag = el("div", "prompt__script");
    tag.textContent = SCRIPTS[questionScript]!.name;
    prompt.appendChild(tag);
  }
  wrap.appendChild(prompt);

  // Options
  const opts = el("div", "options");
  q.options.forEach((opt, i) => {
    const b = el("button", "option");
    b.textContent = opt.glyph;
    if (chosen !== null) {
      b.disabled = true;
      if (i === q.answerIndex) b.classList.add("option--correct");
      else if (i === chosen) b.classList.add("option--wrong");
    }
    b.onclick = () => {
      if (chosen !== null) return; // already answered
      chosen = i;
      const correct = checkAnswer(q, i);
      score = record(score, correct);
      // Feed the answer to the scheduler at the SCHEDULE index (a pool index in
      // mixed scope, a letter index otherwise), and advance the session clock.
      schedule = reviewIn(schedule, scheduleIndex, correct, sessionTick);
      sessionTick += 1;
      render();
    };
    opts.appendChild(b);
  });
  wrap.appendChild(opts);

  // Reveal + next
  if (chosen !== null) {
    const correct = checkAnswer(q, chosen);
    const reveal = el("div", "reveal");
    const verdict = el("div", "reveal__verdict " + (correct ? "ok" : "no"));
    verdict.textContent = correct
      ? "✓ Correct"
      : `✗ Not quite — that sound is ${q.targetGlyph}`;
    reveal.appendChild(verdict);
    // show the answer's decomposition, reusing the browse detail
    reveal.appendChild(renderDetail(views[q.targetIndex]!, SCRIPTS[questionScript]!.script));
    const next = el("button", "next");
    next.textContent = "Next →";
    next.onclick = () => {
      nextQuestion();
      render();
    };
    reveal.appendChild(next);
    wrap.appendChild(reveal);
  }
  return wrap;
}

// --- top-level render -------------------------------------------------------

// --- lessons ----------------------------------------------------------------

/**
 * Choose the next lesson to review.
 *
 * Two ideas, both borrowed rather than invented. `pickNext` (scheduler.ts)
 * already picks the most-overdue item; `buildPool` (interleave.ts) already
 * round-robins across groups, so grouping lessons BY LANGUAGE and pooling them
 * gives cross-language interleaving for free — Spanish, then Tamil, then
 * French, rather than all of Spanish first. That mixing is the point: it forces
 * you to discriminate between languages instead of coasting inside one.
 */
function pickLesson(): void {
  lessonRevealed = false;
  if (LESSONS.length === 0) {
    lessonIndex = null;
    return;
  }
  // PREREQUISITE GATE, applied to the POOL rather than to the pick.
  //
  // The scheduler is generic over a numeric index and has no idea that "the
  // preterite of comer" presupposes "comer". But gating must happen *inside*
  // the rotation, not after it: picking and then rejecting collapses to serving
  // the one fallback lesson forever, because the same pick is rejected on every
  // turn. That is the 0.5.0 bug in a new costume — a review simulation caught
  // it serving one Arabic lesson 34 times in 40.
  //
  // Recomputed per pick because `seen` grows as you study; it is a single pass
  // over ~700 lessons, dwarfed by the render that follows.
  const selected = new Set(selectedLanguages);
  const open = new Set(
    unlockedIndices(LESSONS, seenLessonIds()).filter((index) =>
      selected.has(LESSONS[index]!.language),
    ),
  );

  // The scan itself is a pure function in lessons.ts (and tested there); this
  // only threads the cursor. LESSON_GROUPS / LESSON_POOL are computed once —
  // they are constant for the page's lifetime.
  const { index, cursor } = nextDue(
    LESSON_GROUPS,
    LESSON_POOL,
    lessonSchedule,
    lessonSession,
    lessonCursor,
    (i) => open.has(i),
  );
  lessonCursor = cursor;
  if (index !== null) {
    lessonIndex = index;
    return;
  }

  // Nothing due among the unlocked lessons: fall back to the most-overdue pick,
  // but only over the unlocked ones, so the mode is never a dead end AND never
  // a loop. `pickNext` reads `letterIndex`, which carries the real lesson index
  // through the filter.
  const openStates = lessonSchedule.filter((s) => open.has(s.letterIndex));
  lessonIndex =
    openStates.length > 0 ? pickNext(openStates, lessonSession) : null;
}

/**
 * Lesson ids the learner has actually reviewed.
 *
 * Keyed on REVIEW HISTORY, never on `dueAtSession` — fresh items are seeded
 * with the current session, so a due-based test reports the whole curriculum as
 * "seen" on any reload after the first. That bug shipped once; see progress.ts.
 */
function seenLessonIds(): ReadonlySet<string> {
  const out = new Set<string>();
  lessonSchedule.forEach((s, i) => {
    if (s.reps > 0 || s.lapses > 0 || s.box > 0) out.add(LESSON_IDS[i]!);
  });
  return out;
}

/** Grade the current lesson, advance the clock, and save. */
function gradeLesson(wasCorrect: boolean): void {
  if (lessonIndex === null) return;
  lessonSchedule = reviewIn(lessonSchedule, lessonIndex, wasCorrect, lessonSession);
  lessonSession += 1;
  persistLessons();
  pickLesson();
  render();
}

// --- learn mode — the curriculum session -----------------------------------
//
// This is the app's authored curriculum made visible. Each selected language
// contributes exactly its next prerequisite-safe local
// lesson. A focused check advances that language; independently completed
// shared lessons become the mixed review pool below. Everything enters the DOM
// through textContent — the corpus is repo-authored, but remains data.

/** The complete registry-backed language mixer shared by Learn and practice views. */
function renderLanguagePicker(): HTMLElement {
  const details = el("details", "language-picker") as HTMLDetailsElement;
  const summary = el("summary", "language-picker__summary");
  summary.textContent = `Languages · ${selectedLanguages.length} of ${AVAILABLE_LANGUAGE_IDS.length} selected`;
  details.appendChild(summary);

  const note = el("p", "muted language-picker__note");
  const selected = new Set(selectedLanguages);
  const selectedPlans = LANGUAGE_CURRICULA.filter((curriculum) => selected.has(curriculum.language));
  const mappedLessons = selectedPlans.reduce(
    (sum, curriculum) => sum + curriculum.path.reduce((count, segment) => count + segment.lessons.length, 0),
    0,
  );
  const extensions = selectedPlans.reduce((sum, curriculum) => sum + curriculum.extensions.length, 0);
  note.textContent =
    `Choose any mix. The selected local paths currently map ${mappedLessons} micro-lessons` +
    ` and ${extensions} script, grammar, register, etymology, or consolidation extensions.`;
  details.appendChild(note);

  const grid = el("div", "language-picker__grid");
  for (const definition of LANGUAGE_REGISTRY.filter((language) =>
    AVAILABLE_LANGUAGE_IDS.includes(language.id),
  )) {
    const label = el("label", "language-picker__item");
    const input = document.createElement("input");
    input.type = "checkbox";
    input.value = definition.id;
    input.checked = selected.has(definition.id);
    input.onchange = () => {
      const next = new Set(selectedLanguages);
      if (input.checked) next.add(definition.id);
      else next.delete(definition.id);
      selectedLanguages = saveLanguages(REVIEW_STORAGE, next, AVAILABLE_LANGUAGE_IDS);
      focusedAttempt = null;
      learnNotice = null;
      reviewCell = null;
      lessonIndex = null;
      void refreshCorpus(loadLearnCorpus);

// The book-hash manifest is 136 kB and loads lazily (see bookhashes.ts), so
// the "book synced / stale" note in a lesson's metadata line is absent on
// first paint. Re-render once it lands rather than leaving it permanently
// blank. A failed load resolves too, and simply leaves the note off.
void whenBookHashesReady().then(() => {
  render();
});
    };
    const text = el("span", "");
    text.textContent = `${definition.name} · ${definition.script}`;
    label.append(input, text);
    grid.appendChild(label);
  }
  details.appendChild(grid);
  return details;
}

/** Render the authored Markdown through safe DOM nodes; no unsafe HTML path. */
function renderLessonBody(lesson: (typeof LESSONS)[number], initiallyOpen = false): HTMLElement {
  const details = el("details", "lesson-body") as HTMLDetailsElement;
  details.open = initiallyOpen;
  const summary = el("summary", "lesson-body__summary");
  summary.textContent = `Open ${lesson.estMinutes || 5}-minute lesson`;
  details.appendChild(summary);
  for (const sectionData of lessonSections(lesson.body)) {
    const sectionEl = el("section", "lesson-body__section");
    const heading = el("h4", "lesson-body__heading");
    heading.textContent = sectionData.title;
    sectionEl.appendChild(heading);
    for (const block of sectionData.blocks) {
      if (block.kind === "image") {
        const figure = el("figure", "lesson-body__figure");
        const img = document.createElement("img");
        img.src = generatedFigureUrl(lesson.language, block.source);
        img.alt = block.alt;
        img.loading = "lazy";
        img.decoding = "async";
        const caption = el("figcaption", "lesson-body__figure-caption");
        caption.textContent = block.alt;
        figure.append(img, caption);
        sectionEl.appendChild(figure);
      } else {
        const p = el("p", block.text.startsWith("• ") ? "lesson-body__bullet" : "");
        p.textContent = block.text;
        sectionEl.appendChild(p);
      }
    }
    details.appendChild(sectionEl);
  }
  return details;
}

/** One stop of the sweep: a language, its word(s) for the concept, its threads back. */
function renderTeachingStep(
  step: SessionStep,
  ordinal: number,
  intro: ScriptIntro | null,
  badgeText: string | null = ordinal === 0 ? "introduced here" : null,
): HTMLElement {
  const card = el("div", "step");
  card.dataset.language = step.language;

  const head = el("div", "step__head");
  const num = el("span", "step__num");
  num.textContent = String(ordinal + 1);
  const lang = el("span", "step__lang");
  lang.textContent = languageName(step.language);
  head.append(num, lang);
  if (badgeText) {
    const badge = el("span", "step__badge");
    badge.textContent = badgeText;
    head.appendChild(badge);
  }
  card.appendChild(head);

  // A new writing system, the first time the walk reaches it — what it is and
  // how to recognise it, straight from the script data (never invented).
  if (intro) {
    const note = el("div", "step__script");
    const label = el("span", "step__script-label");
    label.textContent = `New script — ${intro.name}`;
    const sys = el("span", "step__script-system");
    sys.textContent = intro.system;
    note.append(label, sys);
    if (intro.signature) {
      const sig = el("p", "step__script-sig");
      sig.textContent = intro.signature;
      note.appendChild(sig);
    }
    card.appendChild(note);
  }

  for (const lesson of step.lessons) {
    const row = el("div", "step__word");
    const glyph = el("span", "step__glyph");
    glyph.textContent = lesson.headword; // in its own script
    glyph.dir = SCRIPTS_BY_ID.get(lesson.script)?.direction ?? "auto";
    row.appendChild(glyph);

    const meta = el("div", "step__meta");
    // Only show romanization when it adds something the headword doesn't.
    if (lesson.romanization && lesson.romanization !== lesson.headword) {
      const rom = el("span", "step__rom");
      rom.textContent = lesson.romanization;
      meta.appendChild(rom);
    }
    const gl = el("span", "step__gloss");
    gl.textContent = lesson.gloss;
    meta.appendChild(gl);
    row.appendChild(meta);
    card.appendChild(row);

    if (lesson.etymologyHook) {
      const hook = el("p", "step__hook");
      hook.textContent = lesson.etymologyHook;
      card.appendChild(hook);
    }
    if (lesson.body.trim() !== "") card.appendChild(renderLessonBody(lesson));
  card.appendChild(renderVoiceControls(lesson));
  }

  // The threads back to earlier languages — the spiral, made literal. Each is a
  // grounded link: the two words genuinely share the named root.
  for (const c of step.connections) {
    const conn = el("p", "step__conn");
    conn.textContent = `↩ connects to ${c.to} — shared root ${c.sharedRoots.join(", ")}`;
    card.appendChild(conn);
  }
  return card;
}

type FrontierStep = ReturnType<typeof mixedCurriculumFrontier>["steps"][number];

function frontierLesson(step: FrontierStep): (typeof LESSONS)[number] | undefined {
  return LESSON_BY_ID.get(step.lessonId);
}

function frontierScriptIntro(step: FrontierStep): ScriptIntro | null {
  if (!step.extensions.some(({ extension }) => extension.category === "script")) return null;
  const data = SCRIPTS_BY_ID.get(scriptOf(step.language));
  return data
    ? { name: data.name, system: data.system, signature: data.signature ?? "" }
    : null;
}

/** Grounded root links among languages simultaneously ready at one ability. */
function frontierConnections(
  step: FrontierStep,
  lesson: (typeof LESSONS)[number],
  earlier: Array<{ step: FrontierStep; lesson: (typeof LESSONS)[number] }>,
): SessionStep["connections"] {
  const roots = new Set(lesson.roots);
  return earlier
    .filter((item) => item.step.spineNode === step.spineNode)
    .map((item) => ({
      to: item.step.language,
      sharedRoots: [...new Set(item.lesson.roots.filter((root) => roots.has(root)))].sort(),
    }))
    .filter((connection) => connection.sharedRoots.length > 0);
}

function finishFocusedCheck(step: FrontierStep): void {
  const curriculum = curriculumForLanguage(step.language);
  if (!curriculum) return;
  const result = completeFrontierLesson(learnCompletion, curriculum, step.lessonId);
  if (!result.changed) return;
  learnCompletion = result.completion;
  saveLearnProgress(REVIEW_STORAGE, learnCompletion, LANGUAGE_CURRICULA);
  focusedAttempt = null;
  reviewCell = null;
  reviewOptions = [];
  reviewChosen = null;
  learnNotice = `${languageName(step.language)} passed focused retrieval; this lesson is now eligible for mixed review.`;
  void refreshCorpus(loadLearnCorpus);
}

function renderFocusedCheck(
  step: FrontierStep,
  lesson: (typeof LESSONS)[number],
  ordinal: number,
): HTMLElement {
  const card = el("div", "step focused-check");
  card.dataset.language = step.language;
  const head = el("div", "step__head");
  const num = el("span", "step__num");
  num.textContent = String(ordinal + 1);
  const lang = el("span", "step__lang");
  lang.textContent = languageName(step.language);
  const badge = el("span", "step__badge");
  badge.textContent = "focused check";
  head.append(num, lang, badge);
  card.appendChild(head);

  const attempt = focusedAttempt?.lessonId === lesson.id ? focusedAttempt : null;
  const kind = focusedCheckKind(lesson);
  const activity = focusedActivity(lesson);
  if (attempt?.state === "correct") {
    const verdict = el("p", "focused-check__feedback yes");
    verdict.textContent = activity?.feedback.correct
      ?? "Correct. This lesson is ready to join independently unlocked review.";
    const advance = el("button", "next") as HTMLButtonElement;
    advance.textContent = `Continue ${languageName(step.language)}`;
    advance.onclick = () => finishFocusedCheck(step);
    card.append(verdict, advance);
    return card;
  }
  if (attempt?.state === "wrong") {
    const verdict = el("p", "focused-check__feedback no");
    verdict.textContent = activity
      ? `${activity.feedback.incorrect} Accepted answer: “${activity.answer}”.`
      : `Not yet. One accepted meaning is “${lesson.gloss}”. Review the lesson, then try again.`;
    card.appendChild(verdict);
    const again = el("button", "next") as HTMLButtonElement;
    again.textContent = "Review lesson again";
    again.onclick = () => {
      focusedAttempt = null;
      render();
    };
    card.appendChild(again);
    return card;
  }

  // An authored activity may ask the learner to produce the headword itself,
  // so its prompt is shown without the lesson's answer-bearing summary card.
  if (kind !== "activity") {
    const glyph = el("div", "focused-check__glyph");
    glyph.textContent = lesson.headword;
    glyph.dir = SCRIPTS_BY_ID.get(lesson.script)?.direction ?? "auto";
    card.appendChild(glyph);
    if (lesson.romanization && lesson.romanization !== lesson.headword) {
      const romanization = el("p", "step__rom");
      romanization.textContent = lesson.romanization;
      card.appendChild(romanization);
    }
  }

  if (kind === "activity" && activity) {
    const form = el("form", "focused-check__form") as HTMLFormElement;
    const label = el("label", "focused-check__prompt");
    label.textContent = activity.prompt;
    const budget = el("span", "focused-check__budget");
    budget.textContent = `Authored response budget: ${activity.responseSeconds}s`;
    const input = document.createElement("input");
    input.className = "focused-check__input";
    input.type = "text";
    input.autocomplete = "off";
    input.setAttribute("aria-label", activity.prompt);
    const submit = el("button", "next") as HTMLButtonElement;
    submit.type = "submit";
    submit.textContent = "Check answer";
    form.append(label, budget, input, submit);
    form.onsubmit = (event) => {
      event.preventDefault();
      const correct = activityAnswerIsCorrect(input.value, activity);
      recordAtomAnswer(activity.assesses, correct);
      focusedAttempt = {
        lessonId: lesson.id,
        state: correct ? "correct" : "wrong",
      };
      learnNotice = null;
      render();
    };
    card.appendChild(form);
  } else if (kind === "meaning") {
    const form = el("form", "focused-check__form") as HTMLFormElement;
    const label = el("label", "focused-check__prompt");
    label.textContent = `Without reopening the lesson, type one English meaning for “${lesson.headword}”.`;
    const input = document.createElement("input");
    input.className = "focused-check__input";
    input.type = "text";
    input.autocomplete = "off";
    input.setAttribute("aria-label", `English meaning for ${languageName(step.language)} ${lesson.headword}`);
    const submit = el("button", "next") as HTMLButtonElement;
    submit.type = "submit";
    submit.textContent = "Check answer";
    form.append(label, input, submit);
    form.onsubmit = (event) => {
      event.preventDefault();
      const correct = meaningAnswerIsCorrect(input.value, lesson.gloss);
      // A meaning check has no authored `assesses` list, so it credits what the
      // lesson exists to teach: its own introduced atoms.
      recordAtomAnswer(lesson.introducesAtoms, correct);
      focusedAttempt = {
        lessonId: lesson.id,
        state: correct ? "correct" : "wrong",
      };
      learnNotice = null;
      render();
    };
    card.appendChild(form);
  } else {
    const prompt = el("p", "focused-check__prompt");
    prompt.textContent = "Complete the lesson's final recall from memory, without reopening its explanation.";
    const actions = el("div", "focused-check__actions");
    const pass = el("button", "next") as HTMLButtonElement;
    pass.textContent = "I completed it from memory";
    pass.onclick = () => finishFocusedCheck(step);
    const again = el("button", "focused-check__secondary") as HTMLButtonElement;
    again.textContent = "Review lesson again";
    again.onclick = () => {
      focusedAttempt = null;
      render();
    };
    actions.append(pass, again);
    card.append(prompt, actions);
  }
  return card;
}

function renderFrontierEncounter(
  step: FrontierStep,
  lesson: (typeof LESSONS)[number],
  ordinal: number,
  connections: SessionStep["connections"],
  readyTogether: number,
): HTMLElement {
  const sessionStep: SessionStep = {
    language: step.language,
    lessons: [lesson],
    connections,
  };
  const card = renderTeachingStep(
    sessionStep,
    ordinal,
    frontierScriptIntro(step),
    readyTogether > 1 ? `${readyTogether} ready together` : "focused first",
  );
  const curriculum = curriculumForLanguage(step.language)!;
  const progress = localPathProgress(curriculum, learnCompletion);
  const context = el("p", "frontier__context");
  const node = spineNodeById(step.spineNode);
  context.textContent =
    `Local lesson ${progress.completed + 1} of ${progress.total}` +
    (node ? ` · ${node.stage} · ${node.canDo}` : "");
  card.appendChild(context);

  if (step.extensions.length > 0) {
    const extensionList = el("p", "frontier__extensions");
    extensionList.textContent = step.extensions
      .map(({ relation, extension }) => `${relation} ${extension.category}: ${extension.canDo}`)
      .join(" · ");
    card.appendChild(extensionList);
  }

  const start = el("button", "next focused-check__start") as HTMLButtonElement;
  start.textContent = "Start focused check";
  start.onclick = () => {
    focusedAttempt = { lessonId: lesson.id, state: "check" };
    learnNotice = null;
    render();
  };
  card.appendChild(start);
  return card;
}

// Which drill the learner is on. A counter rather than a clock, so the drill
// does not change under them mid-answer and "another one" is a deliberate act.
/**
 * How many tracked atoms before a drill is worth loading the full corpus for.
 *
 * Low enough that a learner who has done a handful of lessons gets drills, high
 * enough that a brand-new visitor never pays for a corpus they cannot use. Six
 * is roughly three completed lessons.
 */
const DRILL_CORPUS_THRESHOLD = 6;

// Voice mode (HL10 §10.2). One lesson plays at a time; starting another stops
// the first, and leaving the view stops it too — audio that outlives the thing
// that started it is the worst bug this feature can have.
let voice: { lessonId: string; handle: VoiceHandle; step: string } | null = null;

function stopVoice(): void {
  voice?.handle.stop();
  voice = null;
}

/**
 * Speak one lesson, from the narration the corpus already generates.
 *
 * The learner's half of the loop — recognition — is deliberately absent: a
 * `respond` step waits its authored budget and moves on. That is what a
 * cassette course did, it needs no microphone permission, and it is genuinely
 * useful to somebody driving. Scoring speech is a later slice.
 */
async function speakLesson(lesson: (typeof LESSONS)[number]): Promise<void> {
  stopVoice();
  const speech = browserSpeech(lesson.language);
  if (!speech) {
    learnNotice = "This browser has no speech synthesis, so lessons cannot be read aloud here.";
    render();
    return;
  }
  const chapter = (await loadNarration(lesson.language, lesson.chapter)) as
    | { lessons?: NarrationLesson[] }
    | null;
  const source = chapter?.lessons?.find((candidate) => candidate.id === lesson.id);
  if (!source) {
    learnNotice = "No narration has been generated for this lesson yet.";
    render();
    return;
  }
  const steps = buildVoiceScript(source);
  const handle = playVoiceScript(steps, speech, {
    onStep: (_index, step) => {
      if (!voice) return;
      voice.step =
        step.kind === "speak"
          ? step.text
          : step.kind === "respond"
            ? `Your turn: ${step.instruction}`
            : "…";
      render();
    },
    onDone: () => {
      voice = null;
      render();
    },
  });
  voice = { lessonId: lesson.id, handle, step: "…" };
  render();
}

let drillSeed = 0;
let drillAnswer: { seed: number; used: string[]; total: number } | null = null;

/**
 * A synthesis drill: pieces held, combination unseen.
 *
 * The check is honest about its own limits. It can tell you whether each piece
 * appeared, which is exactly what the drill claims to test; it cannot tell you
 * whether the sentence around them is good Spanish, and it does not pretend to.
 */
function renderSynthesisDrill(drill: SynthesisDrill): HTMLElement {
  const section = el("section", "drill");
  const heading = el("h2", "learn__concept");
  heading.textContent = "Put it together";
  section.appendChild(heading);
  const prompt = el("p", "drill__prompt");
  prompt.textContent = drill.prompt;
  section.appendChild(prompt);

  const list = el("ul", "drill__pieces");
  for (const piece of drill.pieces) {
    const item = el("li", "drill__piece");
    item.textContent = `${piece.headword} — ${piece.gloss} (${piece.domain})`;
    list.appendChild(item);
  }
  section.appendChild(list);

  const shown = drillAnswer?.seed === drillSeed ? drillAnswer : null;
  if (shown) {
    const verdict = el("p", shown.used.length === shown.total ? "drill__verdict yes" : "drill__verdict");
    verdict.textContent =
      shown.used.length === shown.total
        ? `All ${shown.total} pieces used. Whether the sentence around them is good Spanish is not something this check can judge — say it aloud and see if it sounds like something a person would say.`
        : `Used ${shown.used.length} of ${shown.total}. Missing: ${drill.pieces
            .filter((piece) => !shown.used.includes(piece.headword))
            .map((piece) => piece.headword)
            .join(", ")}.`;
    section.appendChild(verdict);
  }

  const form = el("form", "drill__form") as HTMLFormElement;
  const input = document.createElement("input");
  input.className = "drill__input";
  input.type = "text";
  input.autocomplete = "off";
  input.setAttribute("aria-label", "Your sentence");
  const submit = el("button", "next") as HTMLButtonElement;
  submit.type = "submit";
  submit.textContent = "Check my sentence";
  form.append(input, submit);
  form.onsubmit = (event) => {
    event.preventDefault();
    drillAnswer = {
      seed: drillSeed,
      used: piecesUsed(input.value, drill.pieces).map((piece) => piece.headword),
      total: drill.pieces.length,
    };
    render();
  };
  section.appendChild(form);

  const another = el("button", "opt") as HTMLButtonElement;
  another.textContent = "Another combination";
  another.onclick = () => {
    drillSeed += 1;
    drillAnswer = null;
    render();
  };
  section.appendChild(another);
  return section;
}

/**
 * Play/stop for one lesson, plus what is being said right now.
 *
 * The line of current text is not decoration. Voice mode is for somebody whose
 * eyes are elsewhere, but the same page is used by somebody sitting down, and a
 * button that produces sound with no visible sign of what it is doing is
 * indistinguishable from a broken one.
 */
function renderVoiceControls(lesson: (typeof LESSONS)[number]): HTMLElement {
  const bar = el("div", "voice");
  const playing = voice?.lessonId === lesson.id;
  const button = el("button", "opt voice__button") as HTMLButtonElement;
  button.textContent = playing ? "Stop" : "Play this lesson aloud";
  button.onclick = () => {
    if (playing) {
      stopVoice();
      render();
      return;
    }
    void speakLesson(lesson);
  };
  bar.appendChild(button);
  if (playing) {
    const now = el("p", "muted voice__now");
    now.textContent = voice!.step;
    bar.appendChild(now);
  }
  return bar;
}

/** Every lesson id the learner has passed, across all selected paths. */
function completedLessonIds(): Set<string> {
  const done = new Set<string>();
  for (const completed of learnCompletion.values()) {
    for (const id of completed) done.add(id);
  }
  return done;
}

/**
 * The review the learner owes, chosen by atom rather than by lesson.
 *
 * Each row says what it would refresh, because "review this" with no reason is
 * the thing that makes review feel arbitrary. The learner can see that these
 * three lessons are between them carrying nine atoms they have started to lose.
 */
function renderDueReview(picks: ReviewPick[]): HTMLElement {
  const section = el("section", "due-review");
  const heading = el("h2", "learn__concept");
  const owed = new Set(picks.flatMap((pick) => pick.covers)).size;
  heading.textContent = `Due for review — ${owed} atom${owed === 1 ? "" : "s"}`;
  section.appendChild(heading);
  const gloss = el("p", "muted learn__gloss");
  gloss.textContent =
    "Chosen from your own record rather than from lesson order: these are the lessons that" +
    " refresh the most of what you have started to forget.";
  section.appendChild(gloss);
  for (const pick of picks) {
    const lesson = LESSONS.find((candidate) => candidate.id === pick.lessonId);
    if (!lesson) continue;
    const card = el("article", "due-review__card");
    const title = el("p", "due-review__head");
    title.textContent = `${languageName(pick.language)} · ${lesson.headword}`;
    const covers = el("p", "muted due-review__covers");
    covers.textContent =
      `Refreshes ${pick.covers.length} due atom${pick.covers.length === 1 ? "" : "s"}: ` +
      pick.covers.slice(0, 4).join(", ") +
      (pick.covers.length > 4 ? `, and ${pick.covers.length - 4} more` : "");
    card.append(title, covers, renderLessonBody(lesson));
    section.appendChild(card);
  }
  return section;
}

function renderLearn(): HTMLElement {
  const wrap = el("div", "learn");
  wrap.appendChild(renderLanguagePicker());
  const selectedCurricula = selectedLanguages
    .map((language) => curriculumForLanguage(language))
    .filter((curriculum) => curriculum !== undefined);
  const counts = selectedCurricula.map((curriculum) => localPathProgress(curriculum, learnCompletion));
  const completed = counts.reduce((sum, item) => sum + item.completed, 0);
  const total = counts.reduce((sum, item) => sum + item.total, 0);
  const progress = el("p", "score");
  progress.textContent =
    `${completed} of ${total} local lessons passed focused retrieval` +
    ` · ${selectedLanguages.length} independent path${selectedLanguages.length === 1 ? "" : "s"}`;
  wrap.appendChild(progress);
  const track = el("div", "progress");
  const fill = el("div", "progress__fill");
  const pct = total === 0 ? 0 : (completed / total) * 100;
  fill.style.width = `${pct}%`;
  track.appendChild(fill);
  wrap.appendChild(track);

  if (learnNotice) {
    const notice = el("p", "learn__notice");
    notice.textContent = learnNotice;
    wrap.appendChild(notice);
  }

  // Atom-driven review (HL10 §10.1). Everything below this point schedules by
  // lesson; this one section schedules by what the learner has actually
  // forgotten, and it goes first because a debt is more urgent than a frontier.
  const duePicks = reviewPicks(
    MASTERY,
    LESSONS.filter((lesson) => selectedLanguages.includes(lesson.language)).map((lesson) => ({
      id: lesson.id,
      language: lesson.language,
      refreshes: refreshesOf(lesson),
    })),
    completedLessonIds(),
    Date.now(),
  );
  if (duePicks.length > 0) wrap.appendChild(renderDueReview(duePicks));

  // Synthesis drills (HL10 §10.3). Practice the course could not have authored:
  // pieces the learner holds, in a combination no lesson ever showed. Offered
  // AFTER review, because refreshing something you are losing beats stretching
  // something you have.
  //
  // A drill needs the WHOLE corpus, not the learn frontier. Learn mode keeps
  // only the frontier and the completed lessons in memory -- two Spanish
  // lessons for a beginner -- and a drill built from those can never find two
  // different domains to combine. So the first time a learner holds enough to
  // be drilled, pull the rest of the corpus in the background; until it lands,
  // no drill is offered rather than a wrong one.
  if (MASTERY.size >= DRILL_CORPUS_THRESHOLD && !fullCorpusLoaded && !corpusLoading) {
    void refreshCorpus(loadFullCorpus);
  }
  const drill = fullCorpusLoaded
    ? synthesisDrill(
      MASTERY,
      LESSONS.filter((lesson) => selectedLanguages.includes(lesson.language)),
      Date.now(),
      drillSeed,
    )
    : null;
  if (drill) wrap.appendChild(renderSynthesisDrill(drill));

  const frontier = mixedCurriculumFrontier(selectedLanguages, learnCompletion);
  const activeAttempt = focusedAttempt && frontier.steps.some((step) => step.lessonId === focusedAttempt!.lessonId);
  if (!activeAttempt) focusedAttempt = null;
  const heading = el("h2", "learn__concept");
  heading.textContent = frontier.steps.length > 0 ? "Your next local lessons" : "Selected paths complete";
  wrap.appendChild(heading);
  const explanation = el("p", "muted learn__gloss");
  explanation.textContent = frontier.steps.length > 0
    ? "Study one short lesson, then pass its focused check. Only that language advances; only passed lessons enter mixed review."
    : "Every mapped lesson in the selected languages has passed its focused check.";
  wrap.appendChild(explanation);

  if (frontier.steps.length > 0) {
    const sweep = el("div", "sweep");
    const earlier: Array<{ step: FrontierStep; lesson: (typeof LESSONS)[number] }> = [];
    const visibleSteps = focusedAttempt
      ? frontier.steps.filter((step) => step.lessonId === focusedAttempt!.lessonId)
      : frontier.steps;
    visibleSteps.forEach((step, index) => {
      const lesson = frontierLesson(step);
      if (!lesson) return;
      const attempt = focusedAttempt?.lessonId === lesson.id;
      const readyTogether = frontier.bySpineNode.get(step.spineNode)?.length ?? 1;
      const card = attempt
        ? renderFocusedCheck(step, lesson, index)
        : renderFrontierEncounter(
          step,
          lesson,
          index,
          frontierConnections(step, lesson, earlier),
          readyTogether,
        );
      sweep.appendChild(card);
      earlier.push({ step, lesson });
    });
    wrap.appendChild(sweep);
  } else {
    focusedAttempt = null;
  }

  const reviewGrid = eligibleReviewGrid(
    MAPPED_SPINE_LESSONS,
    LANGUAGE_CURRICULA,
    selectedLanguages,
    learnCompletion,
  );
  wrap.appendChild(renderReview(reviewGrid));

  // A quiet way to start over — clears every persisted key, two-click confirmed.
  wrap.appendChild(renderReset());
  return wrap;
}

/** Clear all persisted progress and reset the in-memory session to the start. */
function executeReset(): void {
  clearProgress(removableStorage());
  // Review + independent local-path progress.
  reviewProgress = { states: new Map(), log: [] };
  reviewSession = 0;
  reviewCell = null;
  reviewOptions = [];
  reviewChosen = null;
  learnCompletion = new Map();
  focusedAttempt = null;
  learnNotice = null;
  selectedLanguages = [...AVAILABLE_LANGUAGE_IDS];
  // The Lessons-mode schedule is one of the cleared keys, so its in-memory state
  // must be zeroed too — otherwise Lessons still shows the old stats and the next
  // grade would `persistLessons()` the stale schedule straight back into the key
  // we just wiped, defeating the reset until a reload.
  savedProgress = emptyProgress();
  lessonSchedule = fromSaved(LESSON_IDS, savedProgress);
  lessonSession = savedProgress.session;
  lessonIndex = null;
  lessonRevealed = false;
  lessonCursor = -1;
  resetArmed = false;
  render();
}

/**
 * The "Reset progress" footer — a two-click confirm so a stray tap can't wipe
 * everything. First click arms it (swaps in a warning + Yes/Cancel); the second
 * (Yes) executes; Cancel disarms.
 */
function renderReset(): HTMLElement {
  const wrap = el("div", "learn__reset");
  if (!resetArmed) {
    const btn = el("button", "reset-link") as HTMLButtonElement;
    btn.textContent = "Reset progress";
    btn.onclick = () => {
      resetArmed = true;
      render();
    };
    wrap.appendChild(btn);
    return wrap;
  }
  const warn = el("span", "reset-warn");
  warn.textContent = "Clear all progress — local paths, review, mistakes, and language mix?";
  const yes = el("button", "reset-yes") as HTMLButtonElement;
  yes.textContent = "Yes, reset";
  yes.onclick = () => executeReset();
  const cancel = el("button", "reset-cancel") as HTMLButtonElement;
  cancel.textContent = "Cancel";
  cancel.onclick = () => {
    resetArmed = false;
    render();
  };
  wrap.append(warn, yes, cancel);
  return wrap;
}

// --- learn mode — the review quiz -------------------------------------------

/** A deterministic Fisher–Yates shuffle driven by a seeded rng (pure of Math.random). */
function shuffleWith<T>(items: T[], rng: () => number): T[] {
  const a = items.slice();
  for (let i = a.length - 1; i > 0; i--) {
    const j = Math.floor(rng() * (i + 1));
    [a[i], a[j]] = [a[j]!, a[i]!];
  }
  return a;
}

/**
 * Draw the next review question from the independently eligible grid.
 *
 * The cell is chosen by the engine's SRS-weighted `pickNext` (missed/overdue
 * cells rise, mastered ones sink). The options are the SAME concept in other
 * languages — the cross-language look-alikes worth confusing — plus the answer;
 * if a concept lives in only one language, the remaining slots are filled from
 * elsewhere in the grid so there is always a real choice.
 */
function nextReviewQuestion(grid: GridCell[]): void {
  reviewChosen = null;
  // A fresh rng seeded by the SRS clock: the draw varies as the learner
  // progresses yet stays reproducible for a given state.
  const rng = makeRng(reviewSession * 2654435761 + 1);
  const cell = pickReviewCell(grid, reviewProgress.states, reviewSession, rng);
  reviewCell = cell;
  if (!cell) {
    reviewOptions = [];
    return;
  }

  const byLang = new Map<string, GridCell>();
  for (const c of grid) {
    if (c.concept === cell.concept && !byLang.has(c.language)) byLang.set(c.language, c);
  }
  byLang.set(cell.language, cell); // the exact drawn lesson stands for its language

  // Distractors must be distinct from the answer AND from each other by their
  // SURFACE WORD, not just by cell — sibling languages sometimes share a
  // byte-identical form for a concept (the Latin-script chain especially), and
  // two identical-looking buttons where only one counts is an unfair question.
  const seenWords = new Set<string>([cell.lesson.headword]);
  const distractors: GridCell[] = [];
  const take = (c: GridCell): void => {
    if (distractors.length >= OPTION_COUNT - 1) return;
    if (seenWords.has(c.lesson.headword)) return;
    distractors.push(c);
    seenWords.add(c.lesson.headword);
  };

  // First choice: the same concept in other languages — the cross-language
  // look-alikes the interleaving targets.
  for (const c of shuffleWith([...byLang.values()].filter((c) => c !== cell), rng)) take(c);
  // Fallback: fill any remaining slots from the rest of the grid, so a concept
  // taught in only one language still yields a real choice.
  if (distractors.length < OPTION_COUNT - 1) {
    for (const c of shuffleWith(grid, rng)) take(c);
  }
  reviewOptions = shuffleWith([...distractors, cell], rng);
}

/** Capitalize a chain-language name for display ("hindi" → "Hindi"). */
function capitalize(s: string): string {
  return s.length === 0 ? s : s[0]!.toUpperCase() + s.slice(1);
}

/** Resolve a logged cellKey back to its actual word, for the confusions panel. */
function wordForKey(key: string): string {
  try {
    const [, language, id] = JSON.parse(key) as [string, string, string];
    const lesson = LESSON_BY_ID.get(id);
    return lesson ? `${lesson.headword} (${language})` : key;
  } catch {
    return key;
  }
}

/** The "what I keep confusing" panel — grounded in answers actually recorded. */
function renderConfusions(): HTMLElement | null {
  const conf = confusions(reviewProgress.log);
  if (conf.length === 0) return null;
  const box = el("div", "confusions");
  const h = el("h4", "confusions__title");
  h.textContent = "What you keep confusing";
  box.appendChild(h);
  const list = el("ul", "confusions__list");
  for (const c of conf.slice(0, 6)) {
    const li = el("li", "");
    li.textContent =
      `Picked ${wordForKey(c.chosen)} for ${wordForKey(c.correct)}` +
      (c.count > 1 ? ` · ×${c.count}` : "");
    list.appendChild(li);
  }
  box.appendChild(list);
  return box;
}

function renderReview(grid: GridCell[]): HTMLElement {
  const wrap = el("div", "review");
  const title = el("h3", "review__title");
  title.textContent = "Review — independently unlocked";
  wrap.appendChild(title);

  if (grid.length === 0) {
    const empty = el("p", "muted");
    empty.textContent = "Nothing is eligible yet — pass a focused check above first.";
    wrap.appendChild(empty);
    return wrap;
  }

  if (!mixedReviewReady(grid)) {
    const waiting = el("p", "muted");
    waiting.textContent =
      `${grid.length} lesson${grid.length === 1 ? " is" : "s are"} eligible` +
      "; pass another focused check with a different answer to start mixed review.";
    wrap.appendChild(waiting);
    return wrap;
  }

  // Draw lazily: a null cell means "need a fresh question" (first entry, after
  // Next, or after a focused success changed the eligible local-prefix set).
  if (!reviewCell) nextReviewQuestion(grid);
  const cell = reviewCell;
  if (!cell) return wrap; // grid non-empty, so this is unreachable, but keeps TS happy

  const stat = el("p", "score");
  const conceptCount = new Set(grid.map((c) => c.concept)).size;
  stat.textContent =
    `${grid.length} item${grid.length === 1 ? "" : "s"} · ${conceptCount} concept${conceptCount === 1 ? "" : "s"}` +
    ` · ${reviewProgress.log.length} answered`;
  wrap.appendChild(stat);

  const prompt = el("div", "prompt");
  const label = el("div", "prompt__label");
  label.textContent = `“${cell.lesson.gloss}” — in ${capitalize(cell.language)}?`;
  prompt.appendChild(label);
  wrap.appendChild(prompt);

  const answerKey = cellKey(cell);
  const opts = el("div", "options");
  for (const opt of reviewOptions) {
    const k = cellKey(opt);
    const b = el("button", "option") as HTMLButtonElement;
    b.textContent = opt.lesson.headword;
    b.title = opt.language;
    b.dataset.language = opt.language;
    if (reviewChosen !== null) {
      b.disabled = true;
      if (k === answerKey) b.classList.add("option--correct");
      else if (k === reviewChosen) b.classList.add("option--wrong");
    }
    b.onclick = () => {
      if (reviewChosen !== null) return; // already answered
      reviewChosen = k;
      const correct = k === answerKey;
      // Thread the answer through the engine: promote on a hit, demote + log the
      // confusion (which wrong word was picked) on a miss; advance the SRS clock.
      reviewProgress = applyAnswer(reviewProgress, cell, correct, reviewSession, correct ? undefined : k);
      reviewSession += 1;
      // Persist immediately so a reload resumes exactly here. Silent on failure.
      saveReview(REVIEW_STORAGE, reviewProgress, reviewSession);
      render();
    };
    opts.appendChild(b);
  }
  wrap.appendChild(opts);

  if (reviewChosen !== null) {
    const correct = reviewChosen === answerKey;
    const reveal = el("div", "reveal");
    const verdict = el("div", "reveal__verdict " + (correct ? "ok" : "no"));
    verdict.textContent = correct
      ? "✓ Correct"
      : `✗ ${capitalize(cell.language)} for “${cell.lesson.gloss}” is ${cell.lesson.headword}`;
    reveal.appendChild(verdict);
    const next = el("button", "next") as HTMLButtonElement;
    next.textContent = "Next →";
    next.onclick = () => {
      reviewCell = null; // force a fresh draw from the current eligible grid
      render();
    };
    reveal.appendChild(next);
    wrap.appendChild(reveal);
  }

  const conf = renderConfusions();
  if (conf) wrap.appendChild(conf);
  return wrap;
}

/**
 * Concepts mode — the same idea, side by side, in every language that has it.
 *
 * This is the cross-learning the curriculum's shared `concept_tag`s were always
 * for: *hola / bonjour / नमस्ते* are one concept realized four ways, and seeing
 * them together is a different act from meeting them four chapters apart.
 *
 * Rendered as a collapsed list because there are hundreds of concepts and only
 * one is ever being studied. Everything goes in via `textContent` — the corpus
 * is repo-authored, but it is still data, and it is never worth building an
 * innerHTML habit.
 */
function renderConcepts(): HTMLElement {
  const wrap = el("div", "practice");
  wrap.appendChild(renderLanguagePicker());
  const selected = new Set(selectedLanguages);
  const cards = CONCEPT_CARDS.map((card) => ({
    ...card,
    realizations: card.realizations.filter((realization) => selected.has(realization.language)),
  })).filter((card) => new Set(card.realizations.map((realization) => realization.language)).size >= 2);
  const selectedLessonCount = LESSONS.filter((lesson) => selected.has(lesson.language)).length;

  const stats = el("p", "score");
  stats.textContent =
    `${cards.length} concepts shared by two or more selected languages` +
    ` · from ${selectedLessonCount} lessons`;
  wrap.appendChild(stats);

  if (cards.length === 0) {
    const empty = el("p", "muted");
    empty.textContent = "No concept is taught in more than one language yet.";
    wrap.appendChild(empty);
    return wrap;
  }

  const list = el("div", "concept-list");
  for (const card of cards) {
    const item = el("div", "concept");

    const langs = new Set(card.realizations.map((r) => r.language));
    const head = el("button", "concept__head");
    head.setAttribute("aria-expanded", String(openConcept === card.id));
    head.textContent = `${card.id} — ${langs.size} languages`;
    head.onclick = () => {
      openConcept = openConcept === card.id ? null : card.id;
      render();
    };
    item.appendChild(head);

    if (card.gloss) {
      const gloss = el("p", "muted concept__gloss");
      gloss.textContent = card.gloss;
      item.appendChild(gloss);
    }

    if (openConcept === card.id) {
      const rows = el("div", "concept__rows");
      // One row per language, in track order so the list is stable between
      // openings rather than reordering under the reader.
      for (const r of [...card.realizations].sort((a, b) =>
        a.language.localeCompare(b.language),
      )) {
        const row = el("div", "concept__row");
        row.dataset.language = r.language;

        const lang = el("span", "concept__lang");
        lang.textContent = languageName(r.language);
        row.appendChild(lang);

        const word = el("span", "concept__word");
        word.textContent = r.headword;
        row.appendChild(word);

        // Only useful when it differs from the headword — for Latin-script
        // tracks the package sets them equal, and repeating it is noise.
        if (r.romanization && r.romanization !== r.headword) {
          const rom = el("span", "concept__rom");
          rom.textContent = r.romanization;
          row.appendChild(rom);
        }

        const gloss = el("span", "concept__gloss-inline");
        gloss.textContent = r.gloss;
        row.appendChild(gloss);

        rows.appendChild(row);
      }
      item.appendChild(rows);

      // The etymology hooks are the reason this curriculum exists; surface them
      // where the comparison is happening, not three clicks away.
      const hooks = card.realizations.filter((r) => r.etymologyHook);
      if (hooks.length > 0) {
        const why = el("div", "concept__hooks");
        for (const r of hooks) {
          const p = el("p", "muted");
          p.textContent = `${r.language}: ${r.etymologyHook}`;
          why.appendChild(p);
        }
        item.appendChild(why);
      }
    }

    list.appendChild(item);
  }
  wrap.appendChild(list);
  return wrap;
}

function renderLessons(): HTMLElement {
  const wrap = el("div", "practice");
  wrap.appendChild(renderLanguagePicker());
  const selected = new Set(selectedLanguages);
  const selectedIndices = LESSONS.map((lesson, index) => ({ lesson, index }))
    .filter(({ lesson }) => selected.has(lesson.language))
    .map(({ index }) => index);
  const selectedStates = selectedIndices.map((index) => lessonSchedule[index]!);

  const due = selectedStates.filter((s) => s.dueAtSession <= lessonSession).length;
  const seen = selectedStates.filter((s) => s.reps > 0 || s.lapses > 0 || s.box > 0).length;
  const stats = el("p", "score");
  stats.textContent =
    `${selectedIndices.length} lessons · ${due} due · ` +
    `${seen} started · mastered ${masteredCount(selectedStates)}`;
  wrap.appendChild(stats);

  if (lessonIndex === null) {
    const empty = el("p", "muted");
    empty.textContent = "No lessons found.";
    wrap.appendChild(empty);
    return wrap;
  }

  const lesson = LESSONS[lessonIndex]!;
  const meta = el("p", "muted");
  const hashStatus = bookHashStatus(LESSONS, lesson.language, lesson.chapter);
  const bookStatus = hashStatus === "not-generated" ? "" : ` · book ${hashStatus}`;
  meta.textContent = `${languageName(lesson.language)} · chapter ${lesson.chapter} · ${lesson.id}${bookStatus}`;
  wrap.appendChild(meta);

  // Prompt: the headword, in its own script. Answer hidden until asked for —
  // recall, not recognition.
  const prompt = el("p", "prompt-glyph");
  prompt.textContent = lesson.headword;
  prompt.dir = SCRIPTS_BY_ID.get(lesson.script)?.direction ?? "auto";
  wrap.appendChild(prompt);

  if (!lessonRevealed) {
    const show = el("button", "opt");
    show.textContent = "Show meaning";
    show.onclick = () => {
      lessonRevealed = true;
      render();
    };
    wrap.appendChild(show);
    return wrap;
  }

  const gloss = el("p", "");
  gloss.textContent = lesson.gloss;
  wrap.appendChild(gloss);
  if (lesson.body.trim() !== "") wrap.appendChild(renderLessonBody(lesson, true));

  const buttons = el("div", "opts");
  ([["Again", false], ["Got it", true]] as [string, boolean][]).forEach(
    ([label, correct]) => {
      const b = el("button", "opt");
      b.textContent = label;
      b.onclick = () => gradeLesson(correct);
      buttons.appendChild(b);
    },
  );
  wrap.appendChild(buttons);

  // The curriculum's own review graph, surfaced: every lesson declares what it
  // revisits. Nothing schedules off this yet — that is the next app item — but
  // showing it makes the connective tissue visible.
  if (lesson.reviewsOf.length > 0) {
    wrap.appendChild(section("Revisits", listOf(lesson.reviewsOf, "links")));
  }
  return wrap;
}

function render(): void {
  const data = SCRIPTS[currentScript]!;
  app!.replaceChildren();
  app!.append(renderHeader());
  if (corpusLoading) {
    const loading = el("p", "muted corpus-status");
    loading.textContent = fullCorpusLoaded
      ? "Refreshing lessons…"
      : "Loading the lessons needed for this view…";
    app!.appendChild(loading);
    return;
  }
  if (corpusError) {
    const failure = el("p", "muted corpus-status");
    failure.textContent = `Lessons could not be loaded: ${corpusError}`;
    app!.appendChild(failure);
    return;
  }
  // The script tabs steer per-script work; hide them during a mixed session,
  // and in Lessons/Concepts modes, which span every language rather than one
  // script.
  const spansAllLanguages =
    mode === "learn" || mode === "lessons" || mode === "concepts";
  if (!spansAllLanguages && !(mode === "practice" && scope === "mixed")) {
    app!.appendChild(renderTabs());
  }

  if (mode !== "learn") stopVoice();
  if (mode === "learn") {
    app!.appendChild(renderLearn());
  } else if (mode === "concepts") {
    app!.appendChild(renderConcepts());
  } else if (mode === "lessons") {
    app!.appendChild(renderLessons());
  } else if (mode === "browse") {
    const views = buildScriptView(data);
    const active = views[currentLetter] ?? views[0]!;
    app!.appendChild(renderSummary(scriptSummary(data)));
    // The syllabaries also offer a consonant × vowel matrix; alphabets stay a
    // plain list. A ragged syllabary yields no matrix, so we fall back to the grid.
    const syllabary = isSyllabary(data.letters);
    if (data.independentVowels && data.independentVowels.length > 0) {
      app!.appendChild(renderIndependentVowels(data.independentVowels));
    }
    if (data.finalConsonants && data.finalConsonants.length > 0) {
      app!.appendChild(renderFinalConsonants(data.finalConsonants));
    }
    if (data.digits && data.digits.length > 0) {
      app!.appendChild(renderNumerals(data.digits));
    }
    if (syllabary) app!.appendChild(renderBrowseLayoutToggle());
    const matrix = syllabary && browseLayout === "matrix" ? renderMatrix(data.letters) : null;
    // For a syllabary, offer the same syllable in its sister scripts; alphabets
    // (where the match would be meaningless) get none.
    const siblings = syllabary ? crossScriptSiblings(active.sound, data.script, SCRIPTS) : [];
    const body = el("div", "body");
    body.dataset.script = data.script;
    body.append(matrix ?? renderGrid(views, data.direction), renderDetail(active, data.script, siblings));
    app!.appendChild(body);
  } else {
    if (!question) startPractice();
    app!.appendChild(renderPractice());
  }
}

// --- helpers ----------------------------------------------------------------

/** The bare romanization (drop any "(as in …)" gloss). */
function bareSound(sound: string): string {
  return sound.split(/[ (]/)[0] ?? sound;
}

/** UI-only randomness: an int in [0, n). Never used inside the pure modules. */
function randInt(n: number): number {
  return Math.floor(Math.random() * Math.max(1, n));
}

/**
 * Distractor chooser for the UI: draw from the top of the confusability ranking
 * (roughly twice the needed count) and shuffle, so wrong answers stay hard but
 * vary between questions. Deterministic core stays untouched — this is the
 * seeded/random layer.
 */
function chooseConfusableShuffled(ranked: number[], count: number): number[] {
  const poolSize = Math.min(ranked.length, Math.max(count, count * 2));
  const pool = ranked.slice(0, poolSize);
  for (let i = pool.length - 1; i > 0; i--) {
    const j = Math.floor(Math.random() * (i + 1));
    [pool[i], pool[j]] = [pool[j]!, pool[i]!];
  }
  return pool.slice(0, count);
}

function el<Tag extends keyof HTMLElementTagNameMap>(
  tag: Tag,
  className: string,
): HTMLElementTagNameMap[Tag] {
  const node = document.createElement(tag);
  if (className) node.className = className;
  return node;
}

function kv(key: string, value: string): HTMLElement {
  const wrap = el("span", "kv");
  const k = el("span", "kv__k");
  k.textContent = key;
  const val = el("span", "kv__v");
  val.textContent = value;
  wrap.append(k, val);
  return wrap;
}

function section(title: string, content: Node): HTMLElement {
  const s = el("section", "sec");
  const h = el("h3", "");
  h.textContent = title;
  s.append(h, content);
  return s;
}

function listOf(items: string[], emptyWord: string): HTMLElement {
  if (items.length === 0) {
    const p = el("p", "muted");
    p.textContent = `No ${emptyWord} recorded yet.`;
    return p;
  }
  const ul = el("ul", "pieces");
  for (const it of items) {
    const li = el("li", "");
    li.textContent = it;
    ul.appendChild(li);
  }
  return ul;
}

function orderedListOf(items: string[]): HTMLElement {
  const ol = el("ol", "strokes");
  for (const it of items) {
    const li = el("li", "");
    li.textContent = it;
    ol.appendChild(li);
  }
  return ol;
}

// --- the stroke-order filmstrip (HL-C08) ------------------------------------
//
// `ductusview.ts` describes the picture; this turns that description into real
// nodes. Note what it does NOT do: there is no `innerHTML` here, exactly as
// there is none anywhere else in this file. Every attribute goes in through
// `setAttribute` and every caption through `textContent`, so a label can only
// ever become text — never markup — no matter where the label came from.
const SVG_NS = "http://www.w3.org/2000/svg";

function svgElement(node: SvgNode): SVGElement {
  const element = document.createElementNS(SVG_NS, isSafeName(node.tag) ? node.tag : "g");
  for (const [name, value] of Object.entries(node.attrs)) {
    // `setAttribute` cannot be escaped out of, but it CAN set an event handler
    // if the name says so — same refusal as the string serialiser applies.
    if (isSafeName(name)) element.setAttribute(name, String(value));
  }
  if (node.text !== undefined) element.textContent = node.text;
  for (const child of node.children ?? []) element.appendChild(svgElement(child));
  return element;
}

// The glyph outline must come from the owning script's FONT — never a
// hand-drawn shape, because a subtly wrong letter looks perfect to precisely
// the audience that cannot yet read the script (see truetype.ts). Each font is
// fetched once, lazily, and only when one of its authored letters is opened.
// Any unavailable or unknown font keeps the prose fallback intact.
const DUCTUS_FONT_URLS = new Map<string, string>([
  ["_fonts/NotoSansTamil-Static.ttf", tamilFontUrl],
  ["_fonts/NotoNaskhArabic-Static.ttf", naskhFontUrl],
]);
const ductusFontPromises = new Map<string, Promise<Font | null>>();

function ductusFont(glyph: string, script: string): Promise<Font | null> {
  const letter = ductusFor(glyph, script);
  const fontPath = letter && verifiedLetterFont(glyph, letter.source.url);
  const url = fontPath && DUCTUS_FONT_URLS.get(fontPath);
  if (!url) return Promise.resolve(null);

  let promise = ductusFontPromises.get(url);
  if (!promise) {
    promise = fetch(url)
      .then((r) => (r.ok ? r.arrayBuffer() : Promise.reject(new Error(`font ${r.status}`))))
      .then((bytes) => parseFont(bytes))
      .catch(() => null);
    ductusFontPromises.set(url, promise);
  }
  return promise;
}

/**
 * The "Write it" section for a letter that HAS an authored, cited pen path.
 *
 * It starts as the prose stroke order — which is what every letter shows — and
 * upgrades itself to the filmstrip once the font has arrived and the glyph has
 * been found in it. If either never happens, the prose simply stays. That is
 * the whole fallback story: the richer view is additive, never load-bearing.
 */
function renderDuctusSection(v: LetterView, script: string): HTMLElement {
  const letter = ductusFor(v.glyph, script)!;
  const holder = el("div", "ductus");
  holder.appendChild(orderedListOf(v.strokeOrder));

  void ductusFont(letter.glyph, script).then((font) => {
    const glyph = font?.glyphFor(letter.glyph);
    if (!glyph || glyph.contours.length === 0) return; // keep the prose
    const strip = ductusFilmstrip(letter, { path: glyph.path, bounds: boundsOf(glyph.contours) });

    const film = el("div", "ductus__film");
    for (const frame of strip.frames) film.appendChild(svgElement(frame));

    const summary = el("p", "ductus__summary");
    summary.textContent = strip.summary;

    // Provenance, visible. The SHAPE of this path is checked against the font
    // by strokes.test; its ORDER can only be vouched for by a citation, so the
    // citation is shown rather than hidden in a comment.
    const cite = el("p", "ductus__source");
    const src = letter.source;
    if (/^https?:\/\//.test(src.url)) {
      const link = document.createElement("a");
      link.href = src.url;
      link.textContent = src.citation;
      link.rel = "noopener noreferrer";
      link.target = "_blank";
      cite.append("Stroke order after ", link);
    } else {
      cite.textContent = `Stroke order after ${src.citation}`;
    }
    if (src.variation) {
      const caveat = el("span", "ductus__variation");
      caveat.textContent = ` — ${src.variation}`;
      cite.appendChild(caveat);
    }

    holder.replaceChildren(film, summary, cite, orderedListOf(v.strokeOrder));
  });

  return holder;
}

void refreshCorpus(loadLearnCorpus);
