// Note drafts on the web host (E5; task-app-notes-ui-v1.md addendum v1.2),
// against the real task-wasm engine. The same behaviour task-mosaic-app
// serves natively: a Save that names no task is refused where the person can
// see it, with the draft kept, and the title and body are capped as typed
// with Journal's limits, refused rather than truncated.
import { readFile } from "node:fs/promises";
import path from "node:path";
import { beforeAll, describe, expect, it } from "vitest";

import { makeController } from "../src/main";
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
