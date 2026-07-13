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

console.log("task-wasm smoke OK — bars:", JSON.stringify(gantt.data.bars.map((b) => [b.name, b.start, b.finish])));

function assert(cond, msg) {
  if (!cond) {
    console.error("SMOKE FAIL:", msg);
    process.exit(1);
  }
}
