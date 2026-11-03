// One trim for every typed name on the web host (backlog 6e2b;
// code/specs/task-app-data-model.md, "Typed names are trimmed of Unicode
// White_Space"), against the real task-wasm engine. task-core stores names
// verbatim and task-mosaic-app trims each composer's draft with Rust's
// `str::trim`, which is Unicode White_Space. This host used to trim with
// `.trim()`, which keeps U+0085 and strips U+FEFF, so "Task\u0085" was a
// task named with a NEXT LINE here and "Task" natively, and a lone U+0085
// was a task here and refused there. Every case of
// fixtures/task-name-trimming-v1.json goes through every name composer
// below, and task-mosaic-app's native test runs the same cases.
import { readFile } from "node:fs/promises";
import path from "node:path";
import { beforeAll, describe, expect, it } from "vitest";

import { trimWhiteSpace } from "../src/controller";
import { makeController } from "../src/main";
import vectors from "../../../fixtures/task-name-trimming-v1.json";
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
const project = (engine: any) => engine.workspace().data.projects.project;
const tasks = (engine: any): any[] => Object.values(project(engine).tasks ?? {});
const taskNames = (engine: any): string[] => tasks(engine).map((t) => t.name as string);
const labelNames = (engine: any): string[] =>
  Object.values(project(engine).labels ?? {}).map((l: any) => l.name as string);

/// A fresh controller over a fresh engine, optionally with one task added.
function setup(firstTask?: string) {
  const engine = createTaskEngine(wasm);
  const controller = makeController(engine, { now: () => 1_790_000_000_000 });
  if (firstTask !== undefined) {
    send(controller, "newTaskNameChange", { value: firstTask });
    send(controller, "addTask");
  }
  return { engine, controller };
}

/// The Sheet column index of `label`, with the Sheet showing.
function sheetColumn(c: Controller, label: string): number {
  send(c, "showSheet");
  const col = (props(c).sheetColumnHeaders as string[]).indexOf(label);
  expect(col).toBeGreaterThanOrEqual(0);
  return col;
}

describe("typed names are trimmed of Unicode White_Space", () => {
  it("trims exactly the fixture's White_Space code points, and keeps every other one", () => {
    expect(vectors.schemaVersion).toBe(1);
    const expected = new Set(vectors.whiteSpace.map((hex) => parseInt(hex, 16)));
    expect(expected.size).toBe(25);
    // Every BMP code unit, one at a time (a lone surrogate half included),
    // plus astral characters: White_Space is all in the BMP, and an astral
    // character is two surrogate halves, neither of which is White_Space.
    // Mismatches are collected, not asserted one by one, so 65,000 checks
    // stay well inside vitest's per-test budget.
    const units = Array.from({ length: 0x10000 }, (_, cp) => cp);
    const astral = [0x10000, 0x1f600, 0xe0020, 0x10ffff];
    const trimmed: number[] = [];
    const changed: number[] = [];
    for (const cp of [...units, ...astral]) {
      const one = String.fromCodePoint(cp);
      const out = trimWhiteSpace(one);
      if (out === "") trimmed.push(cp);
      else if (out !== one) changed.push(cp);
    }
    expect(changed).toEqual([]);
    expect(trimmed).toEqual([...expected].sort((a, b) => a - b));
    // The two characters `.trim()` gets wrong, spelt out.
    expect(trimWhiteSpace("\u0085")).toBe("");
    expect(trimWhiteSpace("﻿")).toBe("﻿");
  });

  for (const c of vectors.cases) {
    describe(c.name, () => {
      it("Add Task", () => {
        const { engine, controller } = setup();
        send(controller, "newTaskNameChange", { value: c.typed });
        send(controller, "addTask");
        if (c.stored === null) {
          expect(taskNames(engine)).toEqual([]);
          expect(props(controller).newTaskNameError).toBe("Enter a task name.");
          // And typing the same draft again does not clear the error.
          send(controller, "newTaskNameChange", { value: c.typed });
          expect(props(controller).newTaskNameError).toBe("Enter a task name.");
        } else {
          expect(taskNames(engine)).toEqual([c.stored]);
          expect(props(controller).newTaskNameError).toBe("");
        }
      });

      it("the edit-task field's Save", () => {
        const { engine, controller } = setup("Seed");
        send(controller, "editTask", { index: 0 });
        send(controller, "editTaskNameChange", { value: c.typed });
        send(controller, "saveTaskEdit");
        if (c.stored === null) {
          expect(props(controller).editTaskNameError).toBe("Enter a task name.");
          expect(taskNames(engine)).toEqual(["Seed"]);
        } else {
          expect(props(controller).editTaskNameError).toBe("");
          expect(taskNames(engine)).toEqual([c.stored]);
        }
      });

      it("the Sheet Name cell", () => {
        const { engine, controller } = setup("Seed");
        const col = sheetColumn(controller, "Name");
        send(controller, "sheetNavigate", { row: 0, col });
        send(controller, "sheetEditCommit", { value: c.typed });
        expect(taskNames(engine)).toEqual([c.stored ?? "Seed"]);
      });

      it("the label composer", () => {
        const { engine, controller } = setup();
        send(controller, "newLabelNameChange", { value: c.typed });
        send(controller, "addLabel");
        expect(labelNames(engine)).toEqual(c.stored === null ? [] : [c.stored]);
      });

      it("the Sheet Labels cell", () => {
        const { engine, controller } = setup("Seed");
        const taskId = tasks(engine)[0].id;
        // A label whose stored name is the case's: typing the untrimmed name
        // into the cell must find it. For an empty case, a label already on
        // the task: an all-White_Space cell holds no names, so it clears them,
        // as task-mosaic-app's does.
        expect(engine.upsertLabel({ id: "lbl", name: c.stored ?? "Seed label", color: "" }).ok).not.toBe(false);
        if (c.stored === null) {
          expect(engine.setTaskLabels({ id: taskId, labels: ["lbl"] }).ok).not.toBe(false);
        }
        const col = sheetColumn(controller, "Labels");
        send(controller, "sheetNavigate", { row: 0, col });
        send(controller, "sheetEditCommit", { value: c.typed });
        expect(project(engine).tasks[taskId].labels ?? []).toEqual(c.stored === null ? [] : ["lbl"]);
      });

      it("the project composer", () => {
        const { engine, controller } = setup();
        const before = new Set(Object.keys(engine.workspace().data.projects));
        send(controller, "newProjectNameChange", { value: c.typed });
        send(controller, "addProject");
        const added = Object.entries(engine.workspace().data.projects as Record<string, any>)
          .filter(([id]) => !before.has(id))
          .map(([, p]) => p.name);
        expect(added).toEqual(c.stored === null ? [] : [c.stored]);
      });

      it("the checklist composer", () => {
        const { engine, controller } = setup();
        send(controller, "showChecklists");
        send(controller, "newChecklistNameChange", { value: c.typed });
        send(controller, "createChecklist");
        const names = Object.values(project(engine).checklists ?? {}).map((t: any) => t.name);
        expect(names).toEqual(c.stored === null ? [] : [c.stored]);
      });

      it("the checklist item composer", () => {
        const { engine, controller } = setup();
        send(controller, "showChecklists");
        send(controller, "newChecklistNameChange", { value: "Seed list" });
        send(controller, "createChecklist");
        const [template] = Object.values(project(engine).checklists) as any[];
        send(controller, "newChecklistItemChange", { value: c.typed });
        send(controller, "addChecklistItem");
        const items = tasks(engine)
          .filter((t) => t.parent === template.root)
          .map((t) => t.name);
        expect(items).toEqual(c.stored === null ? [] : [c.stored]);
      });
    });
  }
});
