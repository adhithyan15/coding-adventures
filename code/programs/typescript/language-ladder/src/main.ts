// main.ts — the thin DOM shell. It wires the pure view-models from core.ts and
// drill.ts to the page. Two modes:
//   • Browse   — a grid of letters + a "break it apart / write it" detail panel.
//   • Practice — a recall drill: see a sound, pick the glyph, get scored.
//
// Deliberately framework-free vanilla DOM. All the interesting logic lives in
// core.ts / drill.ts (and is unit-tested there); the ONLY randomness lives here,
// in the UI, so the pure modules stay deterministic and testable.

import { SCRIPTS } from "./data.ts";
import {
  buildScriptView,
  scriptSummary,
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
import type { Letter } from "./types.ts";
import { loadLessons, indicesByLanguage, nextDue } from "./lessons.ts";
import {
  planSession,
  applyAnswer,
  type Progress,
} from "./sessionplan.ts";
import { pickNext as pickReviewCell, makeRng, cellKey, type GridCell } from "./quiz.ts";
import { confusions } from "./mistakes.ts";
import { loadReview, saveReview } from "./reviewstore.ts";
import { loadCursor, saveCursor } from "./cursorstore.ts";
import { clearProgress, removableStorage } from "./reset.ts";
import {
  scriptsById,
  firstIntroductionByScript,
  scriptIntroFor,
  type ScriptIntro,
} from "./scriptintro.ts";
import {
  sweepableConcepts,
  activeChain,
  LANGUAGE_CHAIN,
  spineProgress,
} from "./sequence.ts";
import type { SessionStep } from "./session.ts";
import {
  crossLanguageConcepts,
  datasetFromLessons,
  unlockedOrAll,
  type ConceptCard,
} from "./concepts.ts";
import taxonomyJson from "../../../../learning/human-languages/concepts/taxonomy.json";
import type { Taxonomy } from "@coding-adventures/human-language-data/src/types.ts";
import {
  browserStorage,
  emptyProgress,
  fromSaved,
  loadProgress,
  saveProgress,
  seenCount,
  toSaved,
} from "./progress.ts";
import "./styles.css";

const app = document.getElementById("app");
if (!app) throw new Error("missing #app root");

type Mode = "learn" | "browse" | "practice" | "lessons" | "concepts";
let mode: Mode = "learn";

// --- the curriculum session (HL03 phase 6) ----------------------------------
//
// The whole app is really ONE thing: walk the curriculum the way the book does —
// concept by concept, forward along the language chain — and let every new
// language connect back to the ones already learned. `sequence.ts` gives the
// book-ordered spine of concepts; `sessionplan.ts` assembles one concept into a
// teaching sweep across the active chain, each stop carrying its connections
// back to earlier languages that share a root.
//
// The whole chain is active: a concept simply appears in whichever of the ten
// languages actually teach it (a sweep is "the languages that CAN show it").
const ACTIVE_COUNT = LANGUAGE_CHAIN.length;
// How far along the spine the learner has walked. Everything up to and including
// this concept is "covered" (what the review quiz will later draw from); this
// concept is the one currently being taught. The spine itself (CONCEPT_SPINE) is
// built just below, once LESSONS is loaded.
let conceptCursor = 0;

// --- lesson review state ----------------------------------------------------
//
// The letter drills above schedule GLYPHS. This schedules LESSONS — the ~670
// written chapters — using the very same Leitner machinery, because
// scheduler.ts is generic over a numeric index and never cared what an item is.
// The one new thing is that this state SURVIVES: it is keyed by lesson id and
// written to localStorage (see progress.ts), so the app finally remembers you.
const LESSONS = loadLessons();
const LESSON_IDS = LESSONS.map((l) => l.id);
// Consolidation lessons — chapter practice, mixed drills, dialogues, reviews —
// are not atomic concepts: their headword is a placeholder ("(practice)"), they
// carry no roots, and they exist to REVISIT earlier lessons (`reviews_of`).
// That kind of consolidation is exactly what the review quiz is for, so keep
// these out of the teaching spine — the learner should walk real words and
// grammar, one concept at a time, not land on "(practice)".
const CONSOLIDATION_TYPES = new Set(["practice", "practice-mix", "review"]);
const CONCEPT_LESSONS = LESSONS.filter((l) => !CONSOLIDATION_TYPES.has(l.type));
// The book-ordered concept spine the Learn session walks — the concepts taught
// by any active language, in the order the book first introduces them. Constant
// for the page's lifetime (the curriculum does not change while it is open).
const CONCEPT_SPINE = sweepableConcepts(CONCEPT_LESSONS, activeChain(ACTIVE_COUNT));

// Script introductions (phase 7). Index the script data by id, then precompute
// which concept is the FIRST (in book order) to teach each non-Latin script we
// have data for — the single place its "new script" note should appear. Both
// are constant for the page's lifetime and grounded in the real scripts JSON.
const SCRIPTS_BY_ID = scriptsById(SCRIPTS);
const SCRIPT_INTRO_AT = firstIntroductionByScript(
  CONCEPT_SPINE,
  CONCEPT_LESSONS,
  new Set(SCRIPTS_BY_ID.keys()),
);

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
const LESSON_BY_ID = new Map(LESSONS.map((l) => [l.id, l]));
// Restore the review's SRS state + answer log from localStorage so the quiz
// remembers you between visits (reusing the same storage port progress.ts owns).
// A missing, corrupt, or wrong-version blob restores as empty — never throws.
const REVIEW_STORAGE = browserStorage();
const restoredReview = loadReview(REVIEW_STORAGE);
let reviewProgress: Progress = restoredReview.progress;
let reviewSession = restoredReview.session; // advances once per answered question — the SRS clock
let reviewCell: GridCell | null = null; // the question currently on screen
let reviewOptions: GridCell[] = []; // its answer options (one is `reviewCell`)
let reviewChosen: string | null = null; // cellKey of the picked option; null = unanswered

// Resume the teaching walk where it was left off: restore the concept cursor
// from storage, clamped to the current spine (the curriculum may have grown or
// shrunk since the save). A missing/corrupt value starts at 0.
conceptCursor = loadCursor(REVIEW_STORAGE, CONCEPT_SPINE.length);

// "Reset progress" is a two-click confirm: the first click ARMS it (so a stray
// tap can't wipe everything), the second executes. This flag is that arming.
let resetArmed = false;

// Constant for the page's lifetime: lesson indices grouped by language, and the
// round-robin pool over those groups. Computing them once is why consecutive
// reviews can walk across languages cheaply.
const LESSON_GROUPS = indicesByLanguage(LESSONS);
const LESSON_POOL = buildPool(LESSON_GROUPS.map((g) => g.length));

// Cross-language cards: one concept, several languages. Built once — the join
// walks every lesson, and neither the curriculum nor the taxonomy changes while
// the page is open.
const CONCEPT_CARDS: ConceptCard[] = crossLanguageConcepts(
  datasetFromLessons(taxonomyJson as unknown as Taxonomy, LESSONS),
);
/** Which concept card is expanded; null = none. */
let openConcept: string | null = null;
let savedProgress = loadProgress(browserStorage());
let lessonSchedule: ItemState[] = fromSaved(LESSON_IDS, savedProgress);
let lessonSession = savedProgress.session;
let lessonIndex: number | null = null;
let lessonRevealed = false;
/** Rotating position in the interleaved order — see pickLesson(). */
let lessonCursor = -1;

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
    "Walk the curriculum the way the book does — one concept at a time, across every language that teaches it, with the threads back to what you already know.",
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
      mode = m;
      if (mode === "practice") startPractice();
      if (mode === "lessons") pickLesson();
      render();
    };
    wrap.appendChild(b);
  });
  return wrap;
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
    const rh = el("th", "matrix__consonant");
    rh.textContent = row.label;
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

function renderDetail(v: LetterView): HTMLElement {
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
  // Only offer stroke order when we actually have it. The Dravidian syllabaries
  // are recognition-only (their ductus is a separate, paused effort), so showing
  // an empty "Write it" section would imply data we don't have.
  if (v.strokeOrder.length > 0) {
    d.appendChild(section(`Write it — stroke order (${v.strokeOrderNote})`, orderedListOf(v.strokeOrder)));
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
    reveal.appendChild(renderDetail(views[q.targetIndex]!));
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
  const open = new Set(unlockedOrAll(LESSONS, seenLessonIds()));

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
// This is the app's spine made visible: a single concept, taught across every
// active language that has it, in chain order, each stop showing the word and —
// the whole point — the threads back to the languages already learned. The
// learner walks the concept spine forward one step at a time; "covered" grows
// with the cursor, which is what the review quiz (a later slice) will draw from.
//
// All the sequencing is in the engine (`planSession` → teaching sweep with
// connections); this function only lays it out. Everything goes in via
// textContent — the corpus is repo-authored, but it is still data.

/** Humanize a concept tag ("COURTESY-THANKS") for a heading ("courtesy · thanks"). */
function conceptTitle(tag: string): string {
  return tag.toLowerCase().replace(/_/g, " ").replace(/-/g, " · ");
}

/** The gloss of the first lesson taught in the sweep, for the concept subhead. */
function firstGloss(teaching: SessionStep[]): string {
  return teaching[0]?.lessons[0]?.gloss ?? "";
}

/** One stop of the sweep: a language, its word(s) for the concept, its threads back. */
function renderTeachingStep(
  step: SessionStep,
  ordinal: number,
  intro: ScriptIntro | null,
): HTMLElement {
  const card = el("div", "step");

  const head = el("div", "step__head");
  const num = el("span", "step__num");
  num.textContent = String(ordinal + 1);
  const lang = el("span", "step__lang");
  lang.textContent = step.language;
  head.append(num, lang);
  if (ordinal === 0) {
    // The first stop has no connections — it is where the concept enters.
    const badge = el("span", "step__badge");
    badge.textContent = "introduced here";
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

/** Prev / Next along the concept spine — walking the curriculum forward. */
/**
 * Move the teaching cursor to `index` (clamped), the one place all navigation
 * funnels through — Prev, Next, and the jump picker. Resets the review draw
 * (the covered set changed), persists so the app resumes here, and re-renders.
 * A no-op if the target is where we already are.
 */
function jumpToConcept(index: number): void {
  const target = Math.max(0, Math.min(index, CONCEPT_SPINE.length - 1));
  if (target === conceptCursor) return;
  conceptCursor = target;
  reviewCell = null;
  saveCursor(REVIEW_STORAGE, conceptCursor);
  render();
}

function renderLearnNav(): HTMLElement {
  const nav = el("div", "learn__nav");

  const prev = el("button", "opt") as HTMLButtonElement;
  prev.textContent = "← Previous";
  prev.disabled = conceptCursor === 0;
  prev.onclick = () => jumpToConcept(conceptCursor - 1);

  // Jump anywhere in the spine — 186 concepts is a long walk to Next through.
  // A native <select> gives free keyboard type-ahead over the book-ordered list.
  const jump = el("select", "learn__jump") as HTMLSelectElement;
  jump.title = "Jump to concept";
  CONCEPT_SPINE.forEach((concept, i) => {
    const opt = el("option", "") as HTMLOptionElement;
    opt.value = String(i);
    opt.textContent = `${i + 1}. ${conceptTitle(concept)}`;
    if (i === conceptCursor) opt.selected = true;
    jump.appendChild(opt);
  });
  jump.onchange = () => jumpToConcept(Number(jump.value));

  const next = el("button", "opt") as HTMLButtonElement;
  next.textContent = "Next →";
  next.disabled = conceptCursor >= CONCEPT_SPINE.length - 1;
  next.onclick = () => jumpToConcept(conceptCursor + 1);

  nav.append(prev, jump, next);
  return nav;
}

function renderLearn(): HTMLElement {
  const wrap = el("div", "learn");

  if (CONCEPT_SPINE.length === 0) {
    const empty = el("p", "muted");
    empty.textContent = "No concepts to walk yet.";
    wrap.appendChild(empty);
    return wrap;
  }

  // The cursor only moves via the nav buttons, but clamp defensively so a stray
  // value can never index off the end of the spine.
  conceptCursor = Math.max(0, Math.min(conceptCursor, CONCEPT_SPINE.length - 1));

  const concept = CONCEPT_SPINE[conceptCursor]!;
  // Everything up to and including the current concept is "covered" — the review
  // quiz (a later slice) draws from exactly this. Here we render the teaching
  // pass: the current concept alone, swept across the chain.
  const covered = CONCEPT_SPINE.slice(0, conceptCursor + 1);
  const plan = planSession(concept, covered, CONCEPT_LESSONS, ACTIVE_COUNT);

  const progress = el("p", "score");
  progress.textContent =
    `Concept ${conceptCursor + 1} of ${CONCEPT_SPINE.length}` +
    ` · taught in ${plan.teaching.length} language${plan.teaching.length === 1 ? "" : "s"}`;
  wrap.appendChild(progress);

  // A slim bar for how far along the spine you are — a sense of the whole
  // journey that the bare "N of M" count doesn't convey at a glance.
  const track = el("div", "progress");
  const fill = el("div", "progress__fill");
  const pct = spineProgress(conceptCursor, CONCEPT_SPINE.length) * 100;
  fill.style.width = `${pct}%`;
  track.appendChild(fill);
  wrap.appendChild(track);

  const heading = el("h2", "learn__concept");
  heading.textContent = conceptTitle(concept);
  wrap.appendChild(heading);
  const gloss = firstGloss(plan.teaching);
  if (gloss) {
    const g = el("p", "muted learn__gloss");
    g.textContent = gloss;
    wrap.appendChild(g);
  }

  if (plan.teaching.length === 0) {
    const none = el("p", "muted");
    none.textContent = "No active language teaches this concept yet.";
    wrap.appendChild(none);
  } else {
    const sweep = el("div", "sweep");
    plan.teaching.forEach((step, i) => {
      const intro = scriptIntroFor(concept, step.language, SCRIPT_INTRO_AT, SCRIPTS_BY_ID);
      sweep.appendChild(renderTeachingStep(step, i, intro));
    });
    wrap.appendChild(sweep);
  }

  wrap.appendChild(renderLearnNav());

  // The review pass: a cumulative, SRS-weighted quiz over everything covered so
  // far (this concept and all before it). `plan.reviewGrid` is exactly that grid.
  wrap.appendChild(renderReview(plan.reviewGrid));

  // A quiet way to start over — clears every persisted key, two-click confirmed.
  wrap.appendChild(renderReset());
  return wrap;
}

/** Clear all persisted progress and reset the in-memory session to the start. */
function executeReset(): void {
  clearProgress(removableStorage());
  // Review + teaching cursor.
  reviewProgress = { states: new Map(), log: [] };
  reviewSession = 0;
  reviewCell = null;
  reviewOptions = [];
  reviewChosen = null;
  conceptCursor = 0;
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
  warn.textContent = "Clear all progress — review, mistakes, and your place in the walk?";
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
 * Draw the next review question from the covered grid.
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
  title.textContent = "Review — everything so far";
  wrap.appendChild(title);

  if (grid.length === 0) {
    const empty = el("p", "muted");
    empty.textContent = "Nothing to review yet — keep walking the concepts.";
    wrap.appendChild(empty);
    return wrap;
  }

  // Draw lazily: a null cell means "need a fresh question" (first entry, after
  // Next, or after the covered set changed when the concept cursor moved).
  if (!reviewCell) nextReviewQuestion(grid);
  const cell = reviewCell;
  if (!cell) return wrap; // grid non-empty, so this is unreachable, but keeps TS happy

  const stat = el("p", "score");
  const conceptCount = new Set(grid.map((c) => c.concept)).size;
  stat.textContent =
    `${grid.length} items · ${conceptCount} concept${conceptCount === 1 ? "" : "s"}` +
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
      reviewCell = null; // force a fresh draw from the current covered grid
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

  const stats = el("p", "score");
  stats.textContent =
    `${CONCEPT_CARDS.length} concepts shared by two or more languages` +
    ` · from ${LESSONS.length} lessons`;
  wrap.appendChild(stats);

  if (CONCEPT_CARDS.length === 0) {
    const empty = el("p", "muted");
    empty.textContent = "No concept is taught in more than one language yet.";
    wrap.appendChild(empty);
    return wrap;
  }

  const list = el("div", "concept-list");
  for (const card of CONCEPT_CARDS) {
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

        const lang = el("span", "concept__lang");
        lang.textContent = r.language;
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

  const due = lessonSchedule.filter((s) => s.dueAtSession <= lessonSession).length;
  const seen = seenCount(LESSON_IDS, savedProgress);
  const stats = el("p", "score");
  stats.textContent =
    `${LESSONS.length} lessons · ${due} due · ` +
    `${seen} started · mastered ${masteredCount(lessonSchedule)}`;
  wrap.appendChild(stats);

  if (lessonIndex === null) {
    const empty = el("p", "muted");
    empty.textContent = "No lessons found.";
    wrap.appendChild(empty);
    return wrap;
  }

  const lesson = LESSONS[lessonIndex]!;
  const meta = el("p", "muted");
  meta.textContent = `${lesson.language} · chapter ${lesson.chapter} · ${lesson.id}`;
  wrap.appendChild(meta);

  // Prompt: the headword, in its own script. Answer hidden until asked for —
  // recall, not recognition.
  const prompt = el("p", "prompt-glyph");
  prompt.textContent = lesson.headword;
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
  // The script tabs steer per-script work; hide them during a mixed session,
  // and in Lessons/Concepts modes, which span every language rather than one
  // script.
  const spansAllLanguages =
    mode === "learn" || mode === "lessons" || mode === "concepts";
  if (!spansAllLanguages && !(mode === "practice" && scope === "mixed")) {
    app!.appendChild(renderTabs());
  }

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
    if (syllabary) app!.appendChild(renderBrowseLayoutToggle());
    const matrix = syllabary && browseLayout === "matrix" ? renderMatrix(data.letters) : null;
    const body = el("div", "body");
    body.append(matrix ?? renderGrid(views, data.direction), renderDetail(active));
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

function el(tag: string, className: string): HTMLElement {
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

render();
