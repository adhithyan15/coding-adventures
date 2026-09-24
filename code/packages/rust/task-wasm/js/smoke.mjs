// Smoke test: load the built wasm and drive the engine from JS end to end.
//   node js/smoke.mjs   (after ./build-wasm.sh)
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, join } from "node:path";
import { createTaskEngine } from "./task-engine.mjs";

const here = dirname(fileURLToPath(import.meta.url));
const wasm = readFileSync(join(here, "..", "pkg", "task_engine.wasm"));
const engine = createTaskEngine(wasm);

engine.reset();
assert(engine.createTask({ id: "a", name: "Write spec" }).ok, "create a");
assert(engine.createTask({ id: "b", name: "Build" }).ok, "create b");
assert(engine.setDuration({ id: "a", duration: { workingMinutes: 480, elapsed: false } }).ok, "dur a");
assert(engine.setDuration({ id: "b", duration: { workingMinutes: 480, elapsed: false } }).ok, "dur b");
assert(
  engine.linkDependency({
    id: "l1",
    predecessor: "a",
    successor: "b",
    kind: "finishToStart",
    lag: { workingMinutes: 0, elapsed: false },
  }).ok,
  "link a→b",
);

const list = engine.checklist();
assert(list.ok && list.data.length === 2, "checklist has 2 rows");

// 2026-07-13 (Monday) days-since-epoch = 20647.
const gantt = engine.gantt(20647);
assert(gantt.ok && gantt.data.bars.every((b) => b.critical), "FS chain is all-critical");

// Rejections come back as typed errors, not throws.
const dup = engine.createTask({ id: "a", name: "dup" });
assert(dup.ok === false && dup.code === 2, "duplicate rejected with code 2");

// Notes: a standalone note, one attached to a task, and deleting that task
// orphans (not deletes) the note attached to it.
assert(
  engine.upsertNote({ id: "n1", title: "Kickoff", body: "Agenda TBD", attachedTask: null }).ok,
  "standalone note",
);
assert(
  engine.upsertNote({ id: "n2", title: "Detail", body: "...", attachedTask: "b" }).ok,
  "note attached to task b",
);
assert(engine.deleteTask({ id: "b" }).ok, "delete task b");
const snap = JSON.parse(engine.snapshot());
const notes = Object.values(snap.projects[snap.roots[0]].notes ?? {});
assert(notes.length === 2, "both notes survive the task's deletion");
assert(
  notes.find((n) => n.id === "n2").attachedTask === null,
  "n2 orphaned to standalone, not left dangling on a deleted task id",
);

// Complexity config: a freshly reset project starts Board (new-project default),
// and the toggle persists through a snapshot round-trip.
const beforeToggle = JSON.parse(engine.snapshot());
assert(
  beforeToggle.projects[beforeToggle.roots[0]].settings.complexity === "board",
  "fresh project starts Board",
);
assert(engine.setProjectComplexity({ complexity: "full" }).ok, "toggle to full");
const afterToggle = JSON.parse(engine.snapshot());
assert(
  afterToggle.projects[afterToggle.roots[0]].settings.complexity === "full",
  "toggle to Full persists",
);

// Checklists (C1): a template with a yes/no branch, one run, completion gated
// on the visible items only.
assert(engine.createChecklistTemplate({ id: "pre", root: "pre-root", name: "Pre-flight", now: 1 }).ok, "template");
assert(engine.createTask({ id: "fuel", name: "Fuel", parent: "pre-root" }).ok, "item");
assert(engine.createTask({ id: "pax", name: "Passengers?", parent: "pre-root" }).ok, "question");
assert(engine.createTask({ id: "brief", name: "Brief them", parent: "pax" }).ok, "branch item");
assert(
  engine.setDecision({ id: "pax", decision: { question: "Passengers?", answer: null, yesChildren: ["brief"], noChildren: [] } }).ok,
  "decision",
);
assert(engine.answerDecision({ id: "pax", answer: true }).ok === false, "templates are not answered");
assert(engine.instantiateChecklist({ template: "pre", run: "r1", now: 10 }).ok, "run");
let run = engine.checklistRun({ id: "r1" });
assert(run.ok && run.data.rows.length === 2 && !run.data.progress.complete, "unanswered hides the branch");
assert(engine.completeChecklistRun({ id: "r1", now: 20 }).ok === false, "incomplete run cannot complete");
assert(engine.answerDecision({ id: "r1/pax", answer: true }).ok, "answer in the run");
assert(engine.setCompleted({ id: "r1/fuel", completed: true }).ok, "tick");
assert(engine.setCompleted({ id: "r1/brief", completed: true }).ok, "tick revealed item");
assert(engine.completeChecklistRun({ id: "r1", now: 30 }).ok, "complete");
run = engine.checklistRun({ id: "r1" });
assert(run.data.status === "completed" && run.data.durationMs === 20, "completed with duration");
assert(engine.checklists().data.length === 2, "library lists template + run");
assert(!engine.todos().data.some((t) => t.task.startsWith("r1/") || t.task === "fuel"), "checklist items are not todos");

console.log("task-wasm smoke OK — bars:", JSON.stringify(gantt.data.bars.map((b) => [b.name, b.start, b.finish])));

function assert(cond, msg) {
  if (!cond) {
    console.error("SMOKE FAIL:", msg);
    process.exit(1);
  }
}
