import { readFile } from "node:fs/promises";
import path from "node:path";
import { describe, expect, it } from "vitest";

import fixture from "../../../fixtures/presentation-contract-v1.json";
import { makeController } from "../src/main";
// The dependency-free accessor is the production web boundary. The adjacent
// declaration belongs to the generated host copy, so this direct source import
// deliberately stays untyped in the contract test.
// @ts-expect-error JavaScript ABI accessor has no source declaration file.
import { createTaskEngine } from "../../../../../../packages/rust/task-wasm/js/task-engine.mjs";

interface PersistedControllerState {
  snapshot: string;
  order: string[];
  counter: number;
  activeProject?: string;
}

function canonicalEngine(engine: any) {
  const workspace = engine.workspace().data;
  const activeId = engine.activeProject().data as string;
  const projects = Object.values(workspace.projects ?? {})
    .map((project: any) => ({
      name: project.name,
      complexity: project.settings?.complexity ?? "full",
      tasks: Object.values(project.tasks ?? {})
        .map((task: any) => ({
          name: task.name,
          completed: task.completed,
          deadline: task.schedule?.deadline ?? null,
        }))
        .sort((left: any, right: any) => left.name.localeCompare(right.name)),
    }))
    .sort((left: any, right: any) => left.name.localeCompare(right.name));
  return {
    activeProject: workspace.projects[activeId].name,
    projects,
  };
}

function canonicalSlots(props: any) {
  const view = props.boardMode
    ? "board"
    : props.timelineMode
      ? "timeline"
      : props.sheetMode
        ? "sheet"
        : props.calendarMode
          ? "calendar"
          : props.notesMode
            ? "notes"
            : "list";
  return {
    view,
    summary: props.summary,
    ringPercent: props.ringPercent,
    complexityLabel: props.complexityLabel,
    newTaskName: props.newTaskName,
    newTaskDue: props.newTaskDue,
    emptyList: props.emptyList,
    newProjectName: props.newProjectName,
    projectRows: props.projectRows,
    taskRows: props.taskRows.map((row: string[]) => row.slice(0, 4)),
  };
}

describe("shared TaskApp web/native presentation contract", () => {
  it("runs every shared behavior checkpoint through the real WASM controller", async () => {
    const wasmPath = path.resolve(
      process.cwd(),
      "../../../../../packages/rust/target/wasm32-unknown-unknown/release/task_wasm.wasm",
    );
    const wasm = await readFile(wasmPath);
    let engine = createTaskEngine(wasm);
    let persisted: PersistedControllerState | undefined;
    const createController = (seed?: PersistedControllerState) =>
      makeController(engine, {
        initialOrder: seed?.order,
        initialCounter: seed?.counter,
        today: fixture.today,
        onMutate: (snapshot, order, counter, activeProject) => {
          persisted = { snapshot, order: [...order], counter, activeProject };
        },
      });
    let controller = createController();

    for (const step of fixture.steps) {
      if (step.restore) {
        expect(persisted, `${step.id}: no persisted structural checkpoint`).toBeDefined();
        engine = createTaskEngine(wasm);
        engine.load(persisted!.snapshot);
        if (persisted!.activeProject) {
          engine.setActiveProject({ id: persisted!.activeProject });
        }
        controller = createController(persisted);
      } else if (step.event) {
        controller.apply({ type: step.event.type, ...step.event.payload } as never);
      }
      expect(
        {
          engine: canonicalEngine(engine),
          slots: canonicalSlots(controller.getProps()),
        },
        step.id,
      ).toEqual(step.expected);
    }
  });

  it("rejects blank names and impossible due dates without mutating the workspace", async () => {
    const wasmPath = path.resolve(
      process.cwd(),
      "../../../../../packages/rust/target/wasm32-unknown-unknown/release/task_wasm.wasm",
    );
    const engine = createTaskEngine(await readFile(wasmPath));
    let mutations = 0;
    const controller = makeController(engine, { onMutate: () => mutations++ });

    controller.apply({ type: "addTask" });
    expect(controller.getProps().newTaskNameError).toBe("Enter a task name.");
    expect(Object.keys(engine.workspace().data.projects.project.tasks)).toHaveLength(0);
    expect(mutations).toBe(0);

    controller.apply({ type: "newTaskNameChange", value: "Plan the launch" });
    expect(controller.getProps().newTaskNameError).toBe("");
    controller.apply({ type: "newTaskDueChange", value: "2026-02-31" });
    controller.apply({ type: "addTask" });
    expect(controller.getProps().newTaskDueError).toBe(
      "Use a real date in YYYY-MM-DD format.",
    );
    expect(Object.keys(engine.workspace().data.projects.project.tasks)).toHaveLength(0);
    expect(mutations).toBe(0);

    controller.apply({ type: "newTaskDueChange", value: "2026-02-28" });
    expect(controller.getProps().newTaskDueError).toBe("");
    controller.apply({ type: "addTask" });
    expect(Object.keys(engine.workspace().data.projects.project.tasks)).toEqual(["t1"]);
    expect(controller.getProps().newTaskNameFocus).toBe("focus");
    expect(controller.getProps().newTaskDueFocus).toBe("");
    expect(controller.getProps().taskRows[0][16]).toBe("Complete task: Plan the launch");
    expect(mutations).toBe(1);
  });

  it("gives completion controls action-oriented names that track task state", async () => {
    const wasmPath = path.resolve(
      process.cwd(),
      "../../../../../packages/rust/target/wasm32-unknown-unknown/release/task_wasm.wasm",
    );
    const engine = createTaskEngine(await readFile(wasmPath));
    const controller = makeController(engine);

    controller.apply({ type: "newTaskNameChange", value: "Draft release" });
    controller.apply({ type: "addTask" });
    expect(controller.getProps().taskRows[0][16]).toBe("Complete task: Draft release");

    controller.apply({ type: "toggleTask", index: 0 } as never);
    expect(controller.getProps().taskRows[0][16]).toBe("Reopen task: Draft release");
  });

  it("edits a List task atomically and persists through the Rust engine", async () => {
    const wasmPath = path.resolve(
      process.cwd(),
      "../../../../../packages/rust/target/wasm32-unknown-unknown/release/task_wasm.wasm",
    );
    const engine = createTaskEngine(await readFile(wasmPath));
    let mutations = 0;
    const controller = makeController(engine, { onMutate: () => mutations++ });

    controller.apply({ type: "newTaskNameChange", value: "Draft plan" });
    controller.apply({ type: "addTask" });
    controller.apply({ type: "editTask", index: 0 } as never);
    expect(controller.getProps().taskRows[0][15]).toBe("editing");
    expect(controller.getProps().editTaskName).toBe("Draft plan");

    controller.apply({ type: "editTaskNameChange", value: "" } as never);
    controller.apply({ type: "editTaskDueChange", value: "2026-02-31" } as never);
    controller.apply({ type: "saveTaskEdit" } as never);
    expect(controller.getProps().editTaskNameError).toBe("Enter a task name.");
    expect(engine.workspace().data.projects.project.tasks.t1.name).toBe("Draft plan");
    expect(mutations).toBe(1);

    controller.apply({ type: "editTaskNameChange", value: "Launch plan" } as never);
    controller.apply({ type: "saveTaskEdit" } as never);
    expect(controller.getProps().editTaskDueError).toBe(
      "Use a real date in YYYY-MM-DD format.",
    );
    expect(engine.workspace().data.projects.project.tasks.t1.name).toBe("Draft plan");
    expect(mutations).toBe(1);

    controller.apply({ type: "editTaskDueChange", value: "2026-02-28" } as never);
    controller.apply({ type: "saveTaskEdit" } as never);
    const task = engine.workspace().data.projects.project.tasks.t1;
    expect(task.name).toBe("Launch plan");
    expect(task.schedule.deadline).toBe(20512);
    expect(controller.getProps().taskRows[0][15]).toBe("");
    expect(controller.getProps().newTaskNameFocus).toBe("focus");
    expect(mutations).toBe(2);
  });
});
