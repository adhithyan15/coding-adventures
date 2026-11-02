// Note drafts on the web host (E5; task-app-notes-ui-v1.md addendum v1.2),
// against the real task-wasm engine. The same behaviour task-mosaic-app
// serves natively: a Save that names no task is refused where the person can
// see it, with the draft kept, and the title and body are capped as typed
// with Journal's limits, refused rather than truncated.
import { readFile } from "node:fs/promises";
import path from "node:path";
import { beforeAll, describe, expect, it } from "vitest";

import { taskNameKey, trimWhiteSpace } from "../src/controller";
import { makeController } from "../src/main";
import taskNameVectors from "../../../fixtures/task-name-matching-v1.json";
// @ts-expect-error JavaScript ABI accessor has no source declaration file.
import { createTaskEngine } from "../../../../../../packages/rust/task-wasm/js/task-engine.mjs";

const NOT_FOUND =
  "No task in this project has that name. Fix it, or clear the field to save the note without one.";
const TITLE_MAX = 512; // characters
const BODY_MAX = 1024 * 1024; // UTF-8 bytes

let wasm: Buffer;
beforeAll(async () => {
  wasm = await readFile(
    path.resolve(process.cwd(), "../../../../../packages/rust/target/wasm32-unknown-unknown/release/task_wasm.wasm"),
  );
});

/// A controller with one task, "Groceries", showing a new note's editor.
function setup() {
  const engine = createTaskEngine(wasm);
  let mutations = 0;
  const controller = makeController(engine, { onMutate: () => mutations++ });
  send(controller, "newTaskNameChange", { value: "Groceries" });
  send(controller, "addTask");
  send(controller, "showNotes");
  send(controller, "newNote");
  return { engine, controller, mutations: () => mutations };
}

type Controller = ReturnType<typeof makeController>;
const send = (c: Controller, type: string, extra: Record<string, unknown> = {}) =>
  c.apply({ type, ...extra } as never);
const props = (c: Controller) => c.getProps() as any;
const notesOf = (engine: any) => engine.workspace().data.projects.project.notes ?? {};

describe("Notes on the web host", () => {
  it("refuses a Save for an unknown task visibly and keeps the draft", () => {
    const { engine, controller, mutations } = setup();
    const before = mutations();
    send(controller, "noteTitleChange", { value: "Shopping" });
    send(controller, "noteBodyChange", { value: "milk, eggs" });
    send(controller, "noteTaskNameChange", { value: "Grocerys" });
    send(controller, "saveNote");

    expect(props(controller).noteError).toBe(NOT_FOUND);
    expect(Object.keys(notesOf(engine))).toHaveLength(0);
    expect(mutations()).toBe(before);
    expect(props(controller).selectedNoteId).not.toBe("");
    expect(props(controller).noteTitleValue).toBe("Shopping");
    expect(props(controller).noteBodyValue).toBe("milk, eggs");
    expect(props(controller).noteTaskValue).toBe("Grocerys");

    // Typing clears the message; the corrected save lands and stays clear.
    send(controller, "noteTaskNameChange", { value: "groceries" });
    expect(props(controller).noteError).toBe("");
    send(controller, "saveNote");
    expect(props(controller).noteError).toBe("");
    const notes = Object.values(notesOf(engine)) as any[];
    expect(notes).toHaveLength(1);
    expect(notes[0].body).toBe("milk, eggs");
    expect(notes[0].attachedTask).toBeTruthy();
  });

  it("clears the note error on any other event", () => {
    const { controller } = setup();
    send(controller, "noteTaskNameChange", { value: "nobody" });
    send(controller, "saveNote");
    expect(props(controller).noteError).toBe(NOT_FOUND);
    // (toggleTheme would not do: the Root intercepts it before the controller.)
    send(controller, "newProjectNameChange", { value: "Later" });
    expect(props(controller).noteError).toBe("");
    expect(props(controller).noteTaskValue).toBe("nobody");
  });

  it("caps the title in characters, refusing rather than truncating", () => {
    const { controller } = setup();
    // 512 two-byte characters: over 512 bytes, at the cap in characters.
    const atCap = "é".repeat(TITLE_MAX);
    send(controller, "noteTitleChange", { value: atCap });
    expect(props(controller).noteTitleValue).toBe(atCap);
    send(controller, "noteTitleChange", { value: `${atCap}x` });
    expect(props(controller).noteTitleValue).toBe(atCap);
  });

  it("caps the body in UTF-8 bytes, refusing rather than truncating", () => {
    const { controller } = setup();
    const atCap = "b".repeat(BODY_MAX);
    send(controller, "noteBodyChange", { value: atCap });
    expect(props(controller).noteBodyValue).toBe(atCap);
    send(controller, "noteBodyChange", { value: `${atCap}b` });
    expect(props(controller).noteBodyValue).toBe(atCap);

    // Bytes, not characters: half as many two-byte characters fill it.
    send(controller, "noteBodyChange", { value: "" });
    const wide = "é".repeat(BODY_MAX / 2);
    send(controller, "noteBodyChange", { value: wide });
    expect(props(controller).noteBodyValue).toBe(wide);
    send(controller, "noteBodyChange", { value: `${wide}é` });
    expect(props(controller).noteBodyValue).toBe(wide);
  });

  it("loads a note stored over the caps whole, and lets it shrink but not grow", () => {
    const { engine, controller } = setup();
    // Saved before the caps existed: the engine itself has no limit.
    const big = "b".repeat(BODY_MAX + 10);
    expect(engine.upsertNote({ id: "legacy", title: "Old", body: big, attachedTask: null }).ok).not.toBe(false);
    send(controller, "cancelNote");
    send(controller, "selectNote", { index: 0 });
    expect(props(controller).noteBodyValue).toBe(big);

    const shorter = big.slice(0, -1);
    send(controller, "noteBodyChange", { value: shorter });
    expect(props(controller).noteBodyValue).toBe(shorter);
    send(controller, "noteBodyChange", { value: big });
    expect(props(controller).noteBodyValue).toBe(shorter);

    send(controller, "saveNote");
    expect(notesOf(engine).legacy.body).toBe(shorter);
  });

  // Notes 0.3.0 on RecordList and DraftEditor (E3; addendum v1.3).
  it("builds note rows in RecordList's six-field shape", () => {
    const { controller } = setup();
    send(controller, "saveNote");
    send(controller, "newNote");
    send(controller, "noteTitleChange", { value: "Agenda" });
    send(controller, "saveNote");
    const rows = props(controller).noteRows as string[][];
    expect(rows).toHaveLength(2);
    for (const row of rows) {
      expect(row).toHaveLength(6);
      expect([row[1], row[3], row[4], row[5]]).toEqual(["", "", "", ""]);
    }
    expect(rows.map((row) => row[2]).sort()).toEqual(["Agenda", "Untitled"]);
    // The open note is the row whose key matches, as RecordList compares.
    const open = rows.find((row) => row[0] === props(controller).selectedNoteId);
    expect(open?.[2]).toBe("Agenda");
  });

  it("offers Delete only for a note the engine has saved", () => {
    const { controller } = setup();
    // A new note's id is minted, but the engine has never seen it.
    expect(props(controller).selectedNoteId).not.toBe("");
    expect(props(controller).noteDeleteLabel).toBe("");
    send(controller, "saveNote");
    expect(props(controller).noteDeleteLabel).toBe("Delete");
    send(controller, "cancelNote");
    expect(props(controller).noteDeleteLabel).toBe("");
    send(controller, "selectNote", { index: 0 });
    expect(props(controller).noteDeleteLabel).toBe("Delete");
    send(controller, "newNote");
    expect(props(controller).noteDeleteLabel).toBe("");
  });
});

// Matching the typed task name (backlog 6e2; addendum v1.4). The cases are
// shared with task-mosaic-app's `typed_task_names_match_by_the_shared_vectors`,
// which runs the same file natively; a case that passes on one host and fails
// on the other is the bug. Before 6e2 this host already lowercased with
// toLowerCase(), so the cases it changes here are the tie (the last task used
// to win) and the U+0085 / U+FEFF trims (`.trim()` disagrees with Rust's).
type TaskNameCase = { name: string; tasks: string[]; typed: string; attachesTo: number | null };
const tasksOf = (engine: any): Record<string, any> => engine.workspace().data.projects.project.tasks ?? {};

describe("Matching a note's typed task name (shared vectors)", () => {
  it("reads the vectors it shares with task-mosaic-app", () => {
    expect(taskNameVectors.schemaVersion).toBe(1);
    expect(taskNameVectors.cases.length).toBeGreaterThan(0);
  });

  for (const testCase of taskNameVectors.cases as TaskNameCase[]) {
    it(testCase.name, () => {
      const engine = createTaskEngine(wasm);
      const controller = makeController(engine);
      // The id each task got, in the order the case lists them.
      const ids: string[] = [];
      for (const task of testCase.tasks) {
        const before = new Set(Object.keys(tasksOf(engine)));
        send(controller, "newTaskNameChange", { value: task });
        send(controller, "addTask");
        const added = Object.keys(tasksOf(engine)).filter((id) => !before.has(id));
        expect(added).toHaveLength(1);
        ids.push(added[0]);
      }
      // The tie rule is "first by id"; the vectors say "first listed". They
      // agree only while ids ascend in creation order (as strings, the
      // engine's order), so pin that, as the native test does.
      for (let i = 1; i < ids.length; i++) expect(ids[i - 1] < ids[i]).toBe(true);

      send(controller, "showNotes");
      send(controller, "newNote");
      send(controller, "noteTaskNameChange", { value: testCase.typed });
      send(controller, "saveNote");

      const notes = Object.values(notesOf(engine)) as any[];
      if (testCase.attachesTo === null) {
        expect(props(controller).noteError).toBe(NOT_FOUND);
        expect(notes).toHaveLength(0);
      } else {
        expect(props(controller).noteError).toBe("");
        expect(notes).toHaveLength(1);
        expect(notes[0].attachedTask).toBe(ids[testCase.attachesTo]);
      }
    });
  }

  it("lowercases without a locale, exactly as Rust's to_lowercase does", () => {
    // The byte-level results the native side produces for the same input
    // (checked against `str::to_lowercase` when 6e2 was written).
    expect(taskNameKey("ΣΑΣ")).toBe("σας");
    expect(taskNameKey("ΑΣ'Β")).toBe("ασ'β"); // ' is case-ignorable: Σ is not final
    expect(taskNameKey("İ")).toBe("i\u0307");
    expect(taskNameKey("ẞ")).toBe("ß");
    expect(taskNameKey("ß")).toBe("ß");
    expect(taskNameKey("Ǆ")).toBe("ǆ");
    expect(taskNameKey("ǅ")).toBe("ǆ");
    expect(taskNameKey("\u212a")).toBe("k");
  });

  it("trims Unicode White_Space, as Rust's str::trim does", () => {
    expect(trimWhiteSpace(" \t\u00a0Groceries\u3000\n")).toBe("Groceries");
    expect(trimWhiteSpace("Groceries\u0085")).toBe("Groceries");
    expect(trimWhiteSpace("\ufeffGroceries")).toBe("\ufeffGroceries");
    expect(trimWhiteSpace("Gro ceries")).toBe("Gro ceries");
    expect(trimWhiteSpace("   ")).toBe("");
  });
});
