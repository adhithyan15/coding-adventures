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
import "./styles.css";

const app = document.getElementById("app");
if (!app) throw new Error("missing #app root");

type Mode = "browse" | "practice";
let mode: Mode = "browse";
let currentScript = 0;
let currentLetter = 0;

// Practice state
let score: Score = emptyScore();
let question: DrillQuestion | null = null;
let chosen: number | null = null; // which option the learner picked (null = unanswered)

const OPTION_COUNT = 4;

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
  (["browse", "practice"] as Mode[]).forEach((m) => {
    const b = el("button", "mode" + (m === mode ? " mode--active" : ""));
    b.textContent = m === "browse" ? "Browse" : "Practice";
    b.setAttribute("aria-pressed", String(m === mode));
    b.onclick = () => {
      if (mode === m) return;
      mode = m;
      if (mode === "practice") startPractice();
      render();
    };
    wrap.appendChild(b);
  });
  return wrap;
}

/** A tab per script (both modes). Switching script resets practice. */
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

/** Start (or restart) a practice session for the current script. */
function startPractice(): void {
  score = emptyScore();
  nextQuestion();
}

/** Pick a fresh random target + options for the current script. */
function nextQuestion(): void {
  const views = buildScriptView(SCRIPTS[currentScript]!);
  const target = randInt(views.length);
  const placeAt = randInt(Math.min(OPTION_COUNT, views.length));
  // Draw distractors from the most-confusable pool, shuffled for variety.
  question = buildDrillQuestion(views, target, OPTION_COUNT, chooseConfusableShuffled, placeAt);
  chosen = null;
}

function renderPractice(): HTMLElement {
  const views = buildScriptView(SCRIPTS[currentScript]!);
  const q = question!;
  const wrap = el("div", "practice");

  // Score line
  const acc = accuracy(score);
  const scoreLine = el("div", "score");
  scoreLine.textContent =
    acc === null ? "Score: 0 / 0" : `Score: ${score.correct} / ${score.total}  ·  ${acc}%`;
  wrap.appendChild(scoreLine);

  // Prompt
  const prompt = el("div", "prompt");
  const label = el("div", "prompt__label");
  label.textContent = "Which letter makes this sound?";
  const sound = el("div", "prompt__sound");
  sound.textContent = q.promptSound;
  prompt.append(label, sound);
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
      score = record(score, checkAnswer(q, i));
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

function render(): void {
  const data = SCRIPTS[currentScript]!;
  app!.replaceChildren();
  app!.append(renderHeader(), renderTabs());

  if (mode === "browse") {
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
