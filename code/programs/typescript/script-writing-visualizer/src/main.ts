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
import { loadLessons, indicesByLanguage, nextDue } from "./lessons.ts";
import {
  browserStorage,
  fromSaved,
  loadProgress,
  saveProgress,
  seenCount,
  toSaved,
} from "./progress.ts";
import "./styles.css";

const app = document.getElementById("app");
if (!app) throw new Error("missing #app root");

type Mode = "browse" | "practice" | "lessons";
let mode: Mode = "browse";

// --- lesson review state ----------------------------------------------------
//
// The letter drills above schedule GLYPHS. This schedules LESSONS — the ~670
// written chapters — using the very same Leitner machinery, because
// scheduler.ts is generic over a numeric index and never cared what an item is.
// The one new thing is that this state SURVIVES: it is keyed by lesson id and
// written to localStorage (see progress.ts), so the app finally remembers you.
const LESSONS = loadLessons();
const LESSON_IDS = LESSONS.map((l) => l.id);
// Constant for the page's lifetime: lesson indices grouped by language, and the
// round-robin pool over those groups. Computing them once is why consecutive
// reviews can walk across languages cheaply.
const LESSON_GROUPS = indicesByLanguage(LESSONS);
const LESSON_POOL = buildPool(LESSON_GROUPS.map((g) => g.length));
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

// --- shared chrome ----------------------------------------------------------

function renderHeader(): HTMLElement {
  const header = el("header", "header");
  const h1 = el("h1", "");
  h1.textContent = "Script writing — learn it, then write it";
  const sub = el("p", "sub");
  sub.textContent =
    mode === "browse"
      ? "Pick a script and a letter to see its pieces and stroke order — for pen-and-paper practice."
      : "Recall drill: read the sound, pick the matching letter. Wrong answers are the confusable ones.";
  header.append(h1, sub, renderModeToggle());
  return header;
}

function renderModeToggle(): HTMLElement {
  const wrap = el("div", "modes");
  const LABELS: Record<Mode, string> = {
    browse: "Browse",
    practice: "Practice",
    lessons: "Lessons",
  };
  (["browse", "practice", "lessons"] as Mode[]).forEach((m) => {
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
    const tile = el("button", "tile" + (i === currentLetter ? " tile--active" : "") + (v.falseFriend ? " tile--ff" : ""));
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
  head.append(big, meta);
  d.appendChild(head);
  d.appendChild(section("Break it apart — the pieces", listOf(v.components, "pieces")));
  d.appendChild(section(`Write it — stroke order (${v.strokeOrderNote})`, orderedListOf(v.strokeOrder)));
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
  const idx = pickNext(schedule, sessionTick);
  scheduleIndex = idx < 0 ? 0 : idx;
  const { scriptIndex, letterIndex } = resolve(scheduleIndex);
  questionScript = scriptIndex;
  const views = buildScriptView(SCRIPTS[scriptIndex]!);
  const placeAt = randInt(Math.min(OPTION_COUNT, views.length));
  // Distractors come from the target's OWN script, so a Cyrillic prompt never
  // offers a Hebrew decoy.
  question = buildDrillQuestion(views, letterIndex, OPTION_COUNT, chooseConfusableShuffled, placeAt);
  chosen = null;
}

function renderPractice(): HTMLElement {
  // Options + reveal come from the QUESTION's script (may differ from the tab in
  // mixed scope).
  const views = buildScriptView(SCRIPTS[questionScript]!);
  const q = question!;
  const wrap = el("div", "practice");

  wrap.appendChild(renderScopeToggle());

  // Score line + a spaced-repetition mastery read-out (across the whole pool in
  // mixed scope).
  const acc = accuracy(score);
  const mastered = masteredCount(schedule);
  const scoreLine = el("div", "score");
  const scoreText = acc === null ? "Score: 0 / 0" : `Score: ${score.correct} / ${score.total}  ·  ${acc}%`;
  scoreLine.textContent = `${scoreText}   ·   mastered ${mastered} / ${schedule.length}`;
  wrap.appendChild(scoreLine);

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
  // The scan itself is a pure function in lessons.ts (and tested there); this
  // only threads the cursor. LESSON_GROUPS / LESSON_POOL are computed once —
  // they are constant for the page's lifetime.
  const { index, cursor } = nextDue(
    LESSON_GROUPS,
    LESSON_POOL,
    lessonSchedule,
    lessonSession,
    lessonCursor,
  );
  lessonCursor = cursor;
  // Nothing due: fall back to the scheduler's most-overdue pick so the mode is
  // never a dead end.
  lessonIndex = index ?? pickNext(lessonSchedule, lessonSession);
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
  // and in Lessons mode, which spans every language rather than one script.
  if (mode !== "lessons" && !(mode === "practice" && scope === "mixed")) {
    app!.appendChild(renderTabs());
  }

  if (mode === "lessons") {
    app!.appendChild(renderLessons());
  } else if (mode === "browse") {
    const views = buildScriptView(data);
    const active = views[currentLetter] ?? views[0]!;
    app!.appendChild(renderSummary(scriptSummary(data)));
    const body = el("div", "body");
    body.append(renderGrid(views, data.direction), renderDetail(active));
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
