// The Checklists view on the web host (C3b of #14018), against the real
// task-wasm engine. The same behaviour task-mosaic-app serves natively (C3a;
// spec task-app-checklists-view-v1.md), event for event and slot for slot.
import { readFile } from "node:fs/promises";
import path from "node:path";
import { beforeAll, describe, expect, it } from "vitest";

import { makeController } from "../src/main";
// @ts-expect-error JavaScript ABI accessor has no source declaration file.
import { createTaskEngine } from "../../../../../../packages/rust/task-wasm/js/task-engine.mjs";

const NOW = 1_790_000_000_000;
let wasm: Buffer;
beforeAll(async () => {
  wasm = await readFile(
    path.resolve(process.cwd(), "../../../../../packages/rust/target/wasm32-unknown-unknown/release/task_wasm.wasm"),
  );
});

function setup() {
  const engine = createTaskEngine(wasm);
  let mutations = 0;
  const controller = makeController(engine, { now: () => NOW, onMutate: () => mutations++ });
  controller.apply({ type: "showChecklists" } as never);
  return { engine, controller, mutations: () => mutations };
}

type Controller = ReturnType<typeof setup>["controller"];
const send = (c: Controller, type: string, extra: Record<string, unknown> = {}) =>
  c.apply({ type, ...extra } as never);

function template(c: Controller, name: string, items: string[]) {
  send(c, "newChecklistNameChange", { value: name });
  send(c, "createChecklist");
  for (const item of items) {
    send(c, "newChecklistItemChange", { value: item });
    send(c, "addChecklistItem");
  }
  return c.getProps() as any;
}

const column = (rows: string[][], field: number) => rows.map((row) => row[field]);

describe("Checklists on the web host", () => {
  it("is offered before Timeline at both tiers", () => {
    const { controller } = setup();
    const props = controller.getProps() as any;
    expect(props.navOptions).toEqual(["List", "Board", "Sheet", "Calendar", "Notes", "Checklists"]);
    expect(props.navSelectedIndex).toBe(5);
    expect(props.checklistsMode).toBe("checklists");
    expect(props.checklistLibraryEmpty).toBe(true);
    send(controller, "showList");
    send(controller, "showView", { index: 5 });
    expect((controller.getProps() as any).checklistsMode).toBe("checklists");
  });

  it("builds a template, runs it and completes it", () => {
    const { engine, controller, mutations } = setup();
    // Twelve items: the eleventh and twelfth sort after the ninth and tenth
    // although their ids sort before them.
    const names = Array.from({ length: 12 }, (_, n) => `Step ${n + 1}`);
    let props = template(controller, "Release", names);
    expect(props.checklistTemplateMode).toBe(true);
    expect(column(props.checklistOutlineRows, 2)).toEqual(names);
    expect(props.newChecklistItem).toBe("");
    expect(props.checklistLibraryRows[0].slice(1, 4)).toEqual(["Templates", "Release", "12 items"]);
    // Template items are not tasks on the list.
    expect(props.taskRows).toEqual([]);

    send(controller, "startChecklistRun");
    props = controller.getProps();
    expect(props.checklistRunMode).toBe(true);
    expect(props.checklistRunTitle).toBe("Release");
    expect(props.checklistRunProgress).toBe("0 of 12 done");
    expect(props.checklistCompleteLabel).toBe("");
    expect(props.checklistAbandonLabel).toBe("Abandon");
    expect(column(props.checklistLibraryRows, 1)).toEqual(["Templates", "Runs"]);

    for (let i = 0; i < 12; i++) send(controller, "checklistToggle", { index: i });
    props = controller.getProps();
    expect(props.checklistRunProgress).toBe("12 of 12 done");
    expect(props.checklistRunRows[0][4]).toBe("1");
    expect(props.checklistCompleteLabel).toBe("Complete");

    send(controller, "completeChecklistRun");
    props = controller.getProps();
    expect(props.checklistRunProgress).toBe("Completed");
    expect(props.checklistCompleteLabel).toBe("");
    expect(props.checklistLibraryRows[1][5]).toBe("✓");
    expect(props.taskRows).toEqual([]);
    const run = engine.workspace().data.projects.project.checklists[props.selectedChecklistKey];
    expect(run.run.finishedAt).toBe(NOW);
    expect(mutations()).toBeGreaterThan(0);
  });

  it("reveals a question's branch, and a repeated answer clears it", () => {
    const { engine, controller } = setup();
    const props = template(controller, "Pre-flight", ["Raining?", "Take umbrella", "Wear hat"]);
    const [q, yes, no] = column(props.checklistOutlineRows, 0);
    // Decision authoring is C3c: build the branch through the engine.
    expect(
      engine.setDecision({ id: q, decision: { question: "Raining?", answer: null, yesChildren: [yes], noChildren: [no] } }).ok,
    ).toBe(true);

    send(controller, "startChecklistRun");
    let p = controller.getProps() as any;
    expect(column(p.checklistRunRows, 2)).toEqual(["Raining?"]);
    expect(p.checklistRunProgress).toBe("0 of 0 done · 0 of 1 answered");

    send(controller, "checklistAnswerYes", { index: 0 });
    p = controller.getProps();
    expect(column(p.checklistRunRows, 2)).toEqual(["Raining?", "Take umbrella"]);
    send(controller, "checklistAnswerNo", { index: 0 });
    expect(column((controller.getProps() as any).checklistRunRows, 2)).toEqual(["Raining?", "Wear hat"]);
    send(controller, "checklistAnswerNo", { index: 0 });
    expect(column((controller.getProps() as any).checklistRunRows, 2)).toEqual(["Raining?"]);

    // A question is answered, not ticked.
    send(controller, "checklistToggle", { index: 0 });
    expect((controller.getProps() as any).checklistRunRows[0][4]).toBe("");
  });

  it("an abandoned run is read-only, and delete clears the selection", () => {
    const { controller } = setup();
    template(controller, "Release", ["Step"]);
    send(controller, "startChecklistRun");
    send(controller, "abandonChecklistRun");
    let p = controller.getProps() as any;
    expect(p.checklistRunProgress).toBe("Abandoned");
    expect(p.checklistAbandonLabel).toBe("");
    expect(p.checklistLibraryRows[1][5]).toBe("✗");
    send(controller, "checklistToggle", { index: 0 });
    expect((controller.getProps() as any).checklistRunRows[0][4]).toBe("");

    send(controller, "deleteChecklist");
    p = controller.getProps();
    expect(p.selectedChecklistKey).toBe("");
    expect(column(p.checklistLibraryRows, 2)).toEqual(["Release"]);
  });

  it("orders templates by name and runs newest first, and bounds the composers", () => {
    const { controller } = setup();
    template(controller, "Zulu", ["Z"]);
    let p = template(controller, "alpha", ["A"]);
    expect(column(p.checklistLibraryRows, 2)).toEqual(["alpha", "Zulu"]);
    send(controller, "selectChecklist", { index: 1 });
    expect(column((controller.getProps() as any).checklistOutlineRows, 2)).toEqual(["Z"]);

    send(controller, "newChecklistNameChange", { value: "x".repeat(513) });
    expect((controller.getProps() as any).newChecklistName).toBe("");
    send(controller, "newChecklistNameChange", { value: "é".repeat(512) });
    expect(chars((controller.getProps() as any).newChecklistName)).toBe(512);
    send(controller, "newChecklistNameChange", { value: "   " });
    send(controller, "createChecklist");
    p = controller.getProps();
    expect(p.checklistLibraryRows).toHaveLength(2);
  });

  it("switching project clears the selection", () => {
    const { controller } = setup();
    template(controller, "Release", []);
    send(controller, "newProjectNameChange", { value: "Other" });
    send(controller, "addProject");
    const p = controller.getProps() as any;
    expect(p.selectedChecklistKey).toBe("");
    expect(p.checklistLibraryEmpty).toBe(true);
  });

  it("a calendar drop cannot reach a checklist item", () => {
    const { engine, controller } = setup();
    const p = template(controller, "Release", ["Step"]);
    const item = p.checklistOutlineRows[0][0];
    send(controller, "calendarEventDropped", { key: item, kind: "task", targetKey: "2026-02-02", position: "inside" });
    const task = engine.workspace().data.projects.project.tasks[item];
    expect(task.schedule?.constraint ?? "asap").toEqual("asap");
  });
});

describe("a corrupted stored id counter", () => {
  it("is recovered from the ids in use, never reset to reuse them", () => {
    const engine = createTaskEngine(wasm);
    const first = makeController(engine, { now: () => NOW });
    send(first, "newTaskNameChange", { value: "Existing" });
    send(first, "addTask");
    // A reload whose stored counter is garbage: minting must not hang, and
    // must not reuse t1.
    const controller = makeController(engine, { now: () => NOW, initialCounter: Number.NaN });
    send(controller, "newTaskNameChange", { value: "Next" });
    send(controller, "addTask");
    const tasks = Object.keys(engine.workspace().data.projects.project.tasks);
    expect(tasks.sort()).toEqual(["t1", "t2"]);
  });

  it("fails instead of looping once the counter cannot advance", () => {
    const engine = createTaskEngine(wasm);
    const controller = makeController(engine, { now: () => NOW, initialCounter: Number.MAX_SAFE_INTEGER });
    send(controller, "showChecklists");
    send(controller, "newChecklistNameChange", { value: "Release" });
    send(controller, "createChecklist");
    expect((controller.getProps() as any).checklistLibraryEmpty).toBe(true);
  });
});

function chars(text: string) {
  return [...text].length;
}
