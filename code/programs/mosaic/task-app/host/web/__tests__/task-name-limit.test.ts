// The task-name limit on the web host (6e4; code/specs/task-app-data-model.md,
// "Task names are bounded"), against the real task-wasm engine. The same
// behaviour task-mosaic-app serves natively: every field a task name is typed
// into refuses an edit past 512 Unicode scalar values as it is typed, the way
// a note title does, and a name stored before the cap loads whole and can only
// shrink. The boundary cases are shared with the native test through
// fixtures/task-name-limit-v1.json.
import { readFile } from "node:fs/promises";
import path from "node:path";
import { beforeAll, describe, expect, it } from "vitest";

import { scalarCount, TASK_NAME_MAX_CHARS, TASK_NAME_TOO_LONG, taskNameFits } from "../src/controller";
import { makeController } from "../src/main";
import limitVectors from "../../../fixtures/task-name-limit-v1.json";
// @ts-expect-error JavaScript ABI accessor has no source declaration file.
import { createTaskEngine } from "../../../../../../packages/rust/task-wasm/js/task-engine.mjs";

let wasm: Buffer;
beforeAll(async () => {
  wasm = await readFile(
    path.resolve(process.cwd(), "../../../../../packages/rust/target/wasm32-unknown-unknown/release/task_wasm.wasm"),
  );
});

type Controller = ReturnType<typeof makeController>;
const send = (c: Controller, type: string, extra: Record<string, unknown> = {}) =>
  c.apply({ type, ...extra } as never);
const props = (c: Controller) => c.getProps() as any;
const taskNames = (engine: any): string[] =>
  Object.values(engine.workspace().data.projects.project.tasks ?? {}).map((t: any) => t.name as string);

/// A fresh controller over a fresh engine, optionally with one task added.
function setup(firstTask?: string) {
  const engine = createTaskEngine(wasm);
  const controller = makeController(engine);
  if (firstTask !== undefined) {
    send(controller, "newTaskNameChange", { value: firstTask });
    send(controller, "addTask");
  }
  return { engine, controller };
}

const typedOf = (c: { prefix?: string; repeat: string; count: number; suffix: string }) =>
  `${c.prefix ?? ""}${c.repeat.repeat(c.count)}${c.suffix}`;

describe("the task-name limit", () => {
  it("is the shared number, in Unicode scalar values", () => {
    expect(limitVectors.schemaVersion).toBe(1);
    expect(TASK_NAME_MAX_CHARS).toBe(limitVectors.maxChars);
    expect(TASK_NAME_TOO_LONG).toContain(String(TASK_NAME_MAX_CHARS));
  });

  it("counts code points, not UTF-16 units", () => {
    expect("😀".length).toBe(2);
    expect(scalarCount("😀")).toBe(1);
    expect(scalarCount("é")).toBe(2); // scalars, not graphemes
    expect(scalarCount("\ud83d")).toBe(1); // a lone surrogate is one
    expect(scalarCount("x\ude00\ud83d")).toBe(3); // reversed halves are two
    expect(scalarCount("")).toBe(0);
    // It stops one past the limit it is given.
    expect(scalarCount("x".repeat(10_000), 512)).toBe(513);
    expect(taskNameFits("😀".repeat(512))).toBe(true);
    expect(taskNameFits("😀".repeat(513))).toBe(false);
  });

  // Every shared case, end to end through the Add Task composer, exactly as
  // task-mosaic-app's task_names_are_capped_by_the_shared_vectors runs it.
  for (const c of limitVectors.cases) {
    it(`Add Task: ${c.name}`, () => {
      const typed = typedOf(c);
      const { engine, controller } = setup();
      send(controller, "newTaskNameChange", { value: typed });
      expect(props(controller).newTaskName).toBe(c.accepted ? typed : "");
      send(controller, "addTask");
      if (c.accepted) {
        expect(taskNames(engine)).toEqual([typed.trim()]);
      } else {
        expect(taskNames(engine)).toEqual([]);
      }
    });
  }

  it("caps the edit field as typed, and renames at the cap", () => {
    const { engine, controller } = setup("Groceries");
    send(controller, "editTask", { index: 0 });
    const atCap = "😀".repeat(TASK_NAME_MAX_CHARS);
    send(controller, "editTaskNameChange", { value: atCap });
    send(controller, "editTaskNameChange", { value: `${atCap}x` });
    expect(props(controller).editTaskName).toBe(atCap);
    send(controller, "saveTaskEdit");
    expect(props(controller).editTaskNameError).toBe("");
    expect(taskNames(engine)).toEqual([atCap]);
  });

  it("caps the Sheet's Name cell, and only that cell", () => {
    const { engine, controller } = setup("Groceries");
    send(controller, "showSheet");
    const nameCol = (props(controller).sheetColumnHeaders as string[]).indexOf("Name");
    expect(nameCol).toBeGreaterThanOrEqual(0);
    send(controller, "sheetNavigate", { row: 0, col: nameCol });
    expect(props(controller).sheetEditContent).toBe("Groceries");
    const over = "x".repeat(TASK_NAME_MAX_CHARS + 1);
    send(controller, "sheetFormulaChange", { value: over });
    expect(props(controller).sheetEditContent).toBe("Groceries");
    // A commit past the cap is refused and the edit stays open.
    send(controller, "sheetEditCommit", { value: over });
    expect(props(controller).sheetEditCol).toBe(nameCol);
    expect(taskNames(engine)).toEqual(["Groceries"]);
    const atCap = "x".repeat(TASK_NAME_MAX_CHARS);
    send(controller, "sheetEditCommit", { value: atCap });
    expect(taskNames(engine)).toEqual([atCap]);

    // Another column is not a name, so its draft is not capped.
    send(controller, "sheetNavigate", { row: 0, col: nameCol + 1 });
    send(controller, "sheetFormulaChange", { value: over });
    expect(props(controller).sheetEditContent).toBe(over);
  });

  it("caps a note's Attach to task field as typed", () => {
    const { controller } = setup("Groceries");
    send(controller, "showNotes");
    send(controller, "newNote");
    const atCap = "é".repeat(TASK_NAME_MAX_CHARS);
    send(controller, "noteTaskNameChange", { value: atCap });
    send(controller, "noteTaskNameChange", { value: `${atCap}é` });
    expect(props(controller).noteTaskValue).toBe(atCap);
  });

  it("is enforced by the engine too", () => {
    const { engine } = setup();
    const over = "x".repeat(TASK_NAME_MAX_CHARS + 1);
    expect(engine.createTask({ id: "a", name: over }).ok).toBe(false);
    expect(engine.createTask({ id: "a", name: "😀".repeat(TASK_NAME_MAX_CHARS) }).ok).not.toBe(false);
    expect(engine.renameTask({ id: "a", name: `${"😀".repeat(TASK_NAME_MAX_CHARS)}x` }).ok).toBe(false);
  });

  it("loads a task stored over the cap whole, and lets it shrink but not grow", () => {
    // A task named before the cap: written into a snapshot, which loads
    // without measuring anything.
    const seeded = setup("Groceries");
    const legacy = "L".repeat(TASK_NAME_MAX_CHARS + 10);
    const snapshot = JSON.parse(seeded.engine.snapshot());
    const projects = snapshot.projects ?? snapshot.workspace?.projects;
    const task = Object.values(projects.project.tasks)[0] as any;
    task.name = legacy;
    const engine = createTaskEngine(wasm);
    expect(engine.load(JSON.stringify(snapshot)).ok).not.toBe(false);
    expect(taskNames(engine)).toEqual([legacy]);
    const controller = makeController(engine, { initialOrder: [task.id] });

    // A note attached to it opens with the whole name, and still saves.
    expect(engine.upsertNote({ id: "n-old", title: "Old", body: "", attachedTask: task.id }).ok).not.toBe(false);
    send(controller, "showNotes");
    send(controller, "selectNote", { index: 0 });
    expect(props(controller).noteTaskValue).toBe(legacy);
    send(controller, "saveNote");
    expect(props(controller).noteError).toBe("");

    // Edited down, still over the cap: kept. Grown back: refused as typed.
    send(controller, "showList");
    send(controller, "editTask", { index: 0 });
    expect(props(controller).editTaskName).toBe(legacy);
    const shorter = legacy.slice(0, -1);
    send(controller, "editTaskNameChange", { value: shorter });
    send(controller, "editTaskNameChange", { value: legacy });
    expect(props(controller).editTaskName).toBe(shorter);
    send(controller, "saveTaskEdit");
    expect(props(controller).editTaskNameError).toBe("");
    expect(taskNames(engine)).toEqual([shorter]);
  });
});
