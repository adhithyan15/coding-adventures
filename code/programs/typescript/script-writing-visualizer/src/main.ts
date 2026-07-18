// main.ts — the thin DOM shell. It wires the pure view-models from core.ts to
// the page: a row of script tabs, a grid of letter tiles, and a detail panel
// that "breaks apart" the selected letter into its pieces and stroke order.
//
// Deliberately framework-free vanilla DOM: an MVP a beginner can read top to
// bottom. All the interesting logic lives in core.ts (and is unit-tested there).

import { SCRIPTS } from "./data.ts";
import {
  buildScriptView,
  scriptSummary,
  type LetterView,
  type ScriptSummary,
} from "./core.ts";
import "./styles.css";

const app = document.getElementById("app");
if (!app) throw new Error("missing #app root");

let currentScript = 0;
let currentLetter = 0;

/** Build a tab button per script. */
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
      render();
    };
    tabs.appendChild(b);
  });
  return tabs;
}

/** The header line: what this script is, and how many letters / false friends. */
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

/** The clickable grid of letters. */
function renderGrid(views: LetterView[], dir: "ltr" | "rtl"): HTMLElement {
  const grid = el("div", "grid");
  grid.dir = dir;
  views.forEach((v, i) => {
    const tile = el("button", "tile" + (i === currentLetter ? " tile--active" : "") + (v.falseFriend ? " tile--ff" : ""));
    const glyph = el("span", "tile__glyph");
    glyph.textContent = v.glyph;
    const sound = el("span", "tile__sound");
    sound.textContent = v.sound.split(/[ (]/)[0]; // the bare romanization
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

/** The "break it apart and write it" detail for one letter. */
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

/** Full re-render (small enough that a rebuild-on-every-click is fine). */
function render(): void {
  const data = SCRIPTS[currentScript]!;
  const views = buildScriptView(data);
  const active = views[currentLetter] ?? views[0]!;

  app!.replaceChildren();
  const header = el("header", "header");
  const h1 = el("h1", "");
  h1.textContent = "Script writing — break it apart, then write it";
  const sub = el("p", "sub");
  sub.textContent = "Pick a script, pick a letter, and see its pieces and stroke order — for pen-and-paper practice.";
  header.append(h1, sub);

  app!.append(header, renderTabs(), renderSummary(scriptSummary(data)));

  const body = el("div", "body");
  body.append(renderGrid(views, data.direction), renderDetail(active));
  app!.appendChild(body);
}

// --- tiny DOM helpers (kept trivial and dependency-free) --------------------

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
