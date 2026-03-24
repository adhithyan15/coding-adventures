/**
 * state.test.ts — Unit tests for the reducer and tree algorithms.
 *
 * V0.3: Tests create a Store with the reducer and dispatch actions.
 * The same test cases from V0.1 are preserved, just adapted to the
 * store API: dispatch(action) + store.getState() instead of calling
 * mutation functions directly.
 */

import { describe, it, expect, beforeEach } from "vitest";
import { Store } from "@coding-adventures/store";
import { reducer, flattenVisibleItems, computeStats } from "../reducer.js";
import type { AppState } from "../reducer.js";
import {
  createTemplateAction,
  updateTemplateAction,
  deleteTemplateAction,
  createInstanceAction,
  checkItemAction,
  uncheckItemAction,
  answerDecisionAction,
  completeInstanceAction,
  abandonInstanceAction,
  createTodoAction,
  updateTodoAction,
  deleteTodoAction,
  toggleTodoAction,
} from "../actions.js";
import type { TemplateItem } from "../types.js";
import type { CheckInstanceItem, DecisionInstanceItem } from "../types.js";

// ── Helpers ────────────────────────────────────────────────────────────────

function createStore(): Store<AppState> {
  return new Store<AppState>({ templates: [], instances: [], todos: [] }, reducer);
}

/** Builds a minimal flat template with N check items. */
function flatTemplate(n: number): TemplateItem[] {
  return Array.from({ length: n }, (_, i) => ({
    id: `item-${i}`,
    type: "check" as const,
    label: `Step ${i + 1}`,
  }));
}

/** Builds a template item tree: check -> decision(yes: check, no: check) -> check */
function decisionTemplate(): TemplateItem[] {
  return [
    { id: "c1", type: "check", label: "Before decision" },
    {
      id: "d1",
      type: "decision",
      label: "Did it work?",
      yesBranch: [{ id: "c2", type: "check", label: "Yes: Celebrate" }],
      noBranch: [{ id: "c3", type: "check", label: "No: Rollback" }],
    },
    { id: "c4", type: "check", label: "After decision" },
  ];
}

// ── createTemplate ─────────────────────────────────────────────────────────

describe("createTemplate", () => {
  let s: Store<AppState>;
  beforeEach(() => {
    s = createStore();
  });

  it("returns a Template with the given name and description", () => {
    s.dispatch(createTemplateAction("My Checklist", "A test", []));
    const t = s.getState().templates[0]!;
    expect(t.name).toBe("My Checklist");
    expect(t.description).toBe("A test");
  });

  it("assigns a unique id", () => {
    s.dispatch(createTemplateAction("A", "", []));
    s.dispatch(createTemplateAction("B", "", []));
    const state = s.getState();
    expect(state.templates[0]!.id).not.toBe(state.templates[1]!.id);
  });

  it("sets createdAt to a recent timestamp", () => {
    const before = Date.now();
    s.dispatch(createTemplateAction("T", "", []));
    const t = s.getState().templates[0]!;
    expect(t.createdAt).toBeGreaterThanOrEqual(before);
    expect(t.createdAt).toBeLessThanOrEqual(Date.now());
  });

  it("stores the template in state.templates", () => {
    s.dispatch(createTemplateAction("T", "", []));
    expect(s.getState().templates).toHaveLength(1);
  });

  it("stores the provided items", () => {
    const items = flatTemplate(3);
    s.dispatch(createTemplateAction("T", "", items));
    expect(s.getState().templates[0]!.items).toHaveLength(3);
  });
});

// ── updateTemplate ─────────────────────────────────────────────────────────

describe("updateTemplate", () => {
  let s: Store<AppState>;
  beforeEach(() => {
    s = createStore();
  });

  it("updates the name of an existing template", () => {
    s.dispatch(createTemplateAction("Old", "", []));
    const id = s.getState().templates[0]!.id;
    s.dispatch(updateTemplateAction(id, { name: "New" }));
    expect(s.getState().templates[0]!.name).toBe("New");
  });

  it("does not change unpatched fields", () => {
    s.dispatch(createTemplateAction("Name", "Desc", []));
    const id = s.getState().templates[0]!.id;
    s.dispatch(updateTemplateAction(id, { name: "New" }));
    expect(s.getState().templates[0]!.description).toBe("Desc");
  });

  it("is a no-op for unknown id", () => {
    s.dispatch(createTemplateAction("T", "", []));
    s.dispatch(updateTemplateAction("unknown", { name: "X" }));
    expect(s.getState().templates[0]!.name).toBe("T");
  });
});

// ── deleteTemplate ─────────────────────────────────────────────────────────

describe("deleteTemplate", () => {
  let s: Store<AppState>;
  beforeEach(() => {
    s = createStore();
  });

  it("removes the template from state", () => {
    s.dispatch(createTemplateAction("T", "", []));
    const id = s.getState().templates[0]!.id;
    s.dispatch(deleteTemplateAction(id));
    expect(s.getState().templates).toHaveLength(0);
  });

  it("is a no-op for unknown id", () => {
    s.dispatch(createTemplateAction("T", "", []));
    s.dispatch(deleteTemplateAction("unknown"));
    expect(s.getState().templates).toHaveLength(1);
  });
});

// ── createInstance ─────────────────────────────────────────────────────────

describe("createInstance", () => {
  let s: Store<AppState>;
  beforeEach(() => {
    s = createStore();
  });

  it("creates an instance with in-progress status", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(2)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    expect(inst.status).toBe("in-progress");
    expect(inst.completedAt).toBeNull();
  });

  it("copies templateId and templateName", () => {
    s.dispatch(createTemplateAction("My List", "", []));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    expect(inst.templateId).toBe(templateId);
    expect(inst.templateName).toBe("My List");
  });

  it("clones check items preserving label", () => {
    const items = flatTemplate(3);
    s.dispatch(createTemplateAction("T", "", items));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    expect(inst.items).toHaveLength(3);
    expect((inst.items[0] as CheckInstanceItem).label).toBe("Step 1");
  });

  it("initializes check items with checked = false", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(2)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    for (const item of inst.items) {
      expect((item as CheckInstanceItem).checked).toBe(false);
    }
  });

  it("deep-clones decision items with answer = null", () => {
    s.dispatch(createTemplateAction("T", "", decisionTemplate()));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    const decision = inst.items[1] as DecisionInstanceItem;
    expect(decision.type).toBe("decision");
    expect(decision.answer).toBeNull();
  });

  it("clones decision branches recursively", () => {
    s.dispatch(createTemplateAction("T", "", decisionTemplate()));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    const decision = inst.items[1] as DecisionInstanceItem;
    expect(decision.yesBranch).toHaveLength(1);
    expect(decision.noBranch).toHaveLength(1);
    expect(decision.yesBranch[0]?.label).toBe("Yes: Celebrate");
    expect(decision.noBranch[0]?.label).toBe("No: Rollback");
  });

  it("two instances are independent (mutations don't bleed)", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(2)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    s.dispatch(createInstanceAction(templateId));
    const inst1Id = s.getState().instances[0]!.id;
    const inst1ItemId = s.getState().instances[0]!.items[0]!.templateItemId;
    s.dispatch(checkItemAction(inst1Id, inst1ItemId));
    const state = s.getState();
    expect((state.instances[0]!.items[0] as CheckInstanceItem).checked).toBe(true);
    expect((state.instances[1]!.items[0] as CheckInstanceItem).checked).toBe(false);
  });

  it("throws for unknown templateId", () => {
    expect(() => s.dispatch(createInstanceAction("no-such-id"))).toThrow();
  });
});

// ── checkItem / uncheckItem ────────────────────────────────────────────────

describe("checkItem / uncheckItem", () => {
  let s: Store<AppState>;
  beforeEach(() => {
    s = createStore();
  });

  it("marks a check item as checked", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(2)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    const itemId = inst.items[0]!.templateItemId;
    s.dispatch(checkItemAction(inst.id, itemId));
    expect((s.getState().instances[0]!.items[0] as CheckInstanceItem).checked).toBe(true);
  });

  it("checkItem is idempotent", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(1)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    const itemId = inst.items[0]!.templateItemId;
    s.dispatch(checkItemAction(inst.id, itemId));
    s.dispatch(checkItemAction(inst.id, itemId));
    expect((s.getState().instances[0]!.items[0] as CheckInstanceItem).checked).toBe(true);
  });

  it("uncheckItem clears a checked item", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(1)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    const itemId = inst.items[0]!.templateItemId;
    s.dispatch(checkItemAction(inst.id, itemId));
    s.dispatch(uncheckItemAction(inst.id, itemId));
    expect((s.getState().instances[0]!.items[0] as CheckInstanceItem).checked).toBe(false);
  });

  it("can check an item inside a decision branch", () => {
    s.dispatch(createTemplateAction("T", "", decisionTemplate()));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    // Answer the decision yes first
    s.dispatch(answerDecisionAction(inst.id, "d1", "yes"));
    const decision = s.getState().instances[0]!.items[1] as DecisionInstanceItem;
    const branchItemId = decision.yesBranch[0]!.templateItemId;
    s.dispatch(checkItemAction(inst.id, branchItemId));
    const updatedDecision = s.getState().instances[0]!.items[1] as DecisionInstanceItem;
    expect((updatedDecision.yesBranch[0] as CheckInstanceItem).checked).toBe(true);
  });
});

// ── answerDecision ─────────────────────────────────────────────────────────

describe("answerDecision", () => {
  let s: Store<AppState>;
  beforeEach(() => {
    s = createStore();
  });

  it("records yes answer", () => {
    s.dispatch(createTemplateAction("T", "", decisionTemplate()));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    s.dispatch(answerDecisionAction(inst.id, "d1", "yes"));
    const d = s.getState().instances[0]!.items[1] as DecisionInstanceItem;
    expect(d.answer).toBe("yes");
  });

  it("records no answer", () => {
    s.dispatch(createTemplateAction("T", "", decisionTemplate()));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    s.dispatch(answerDecisionAction(inst.id, "d1", "no"));
    const d = s.getState().instances[0]!.items[1] as DecisionInstanceItem;
    expect(d.answer).toBe("no");
  });

  it("can change answer from yes to no", () => {
    s.dispatch(createTemplateAction("T", "", decisionTemplate()));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    s.dispatch(answerDecisionAction(inst.id, "d1", "yes"));
    s.dispatch(answerDecisionAction(inst.id, "d1", "no"));
    const d = s.getState().instances[0]!.items[1] as DecisionInstanceItem;
    expect(d.answer).toBe("no");
  });
});

// ── flattenVisibleItems ────────────────────────────────────────────────────

describe("flattenVisibleItems", () => {
  let s: Store<AppState>;
  beforeEach(() => {
    s = createStore();
  });

  it("flat list of check items — returns all", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(4)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    const visible = flattenVisibleItems(inst.items);
    expect(visible).toHaveLength(4);
  });

  it("unanswered decision — includes decision item, stops before branches", () => {
    s.dispatch(createTemplateAction("T", "", decisionTemplate()));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    const visible = flattenVisibleItems(inst.items);
    expect(visible).toHaveLength(3);
    expect(visible[0]?.type).toBe("check");
    expect(visible[1]?.type).toBe("decision");
    expect(visible[2]?.type).toBe("check");
  });

  it("answered yes — yes-branch items visible, no-branch hidden", () => {
    s.dispatch(createTemplateAction("T", "", decisionTemplate()));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    s.dispatch(answerDecisionAction(inst.id, "d1", "yes"));
    const visible = flattenVisibleItems(s.getState().instances[0]!.items);
    expect(visible).toHaveLength(4);
    const labels = visible.map((i) => i.label);
    expect(labels).toContain("Yes: Celebrate");
    expect(labels).not.toContain("No: Rollback");
  });

  it("answered no — no-branch items visible, yes-branch hidden", () => {
    s.dispatch(createTemplateAction("T", "", decisionTemplate()));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    s.dispatch(answerDecisionAction(inst.id, "d1", "no"));
    const visible = flattenVisibleItems(s.getState().instances[0]!.items);
    expect(visible).toHaveLength(4);
    const labels = visible.map((i) => i.label);
    expect(labels).toContain("No: Rollback");
    expect(labels).not.toContain("Yes: Celebrate");
  });

  it("nested decision — inner branch only revealed after outer answered", () => {
    const items: TemplateItem[] = [
      {
        id: "outer",
        type: "decision",
        label: "Outer?",
        yesBranch: [
          {
            id: "inner",
            type: "decision",
            label: "Inner?",
            yesBranch: [{ id: "deep", type: "check", label: "Deep item" }],
            noBranch: [],
          },
        ],
        noBranch: [],
      },
    ];
    s.dispatch(createTemplateAction("T", "", items));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const instId = s.getState().instances[0]!.id;

    // Before any answer: only outer decision visible
    const v0 = flattenVisibleItems(s.getState().instances[0]!.items);
    expect(v0).toHaveLength(1);
    expect(v0[0]?.label).toBe("Outer?");

    // After answering outer yes: outer + inner decision (inner unanswered)
    s.dispatch(answerDecisionAction(instId, "outer", "yes"));
    const v1 = flattenVisibleItems(s.getState().instances[0]!.items);
    expect(v1).toHaveLength(2);
    expect(v1[1]?.label).toBe("Inner?");

    // After answering inner yes: outer + inner + deep check item
    s.dispatch(answerDecisionAction(instId, "inner", "yes"));
    const v2 = flattenVisibleItems(s.getState().instances[0]!.items);
    expect(v2).toHaveLength(3);
    expect(v2[2]?.label).toBe("Deep item");
  });
});

// ── computeStats ───────────────────────────────────────────────────────────

describe("computeStats", () => {
  let s: Store<AppState>;
  beforeEach(() => {
    s = createStore();
  });

  it("fresh instance — 0 checked, 0% completion", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(3)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    const stats = computeStats(inst);
    expect(stats.totalItems).toBe(3);
    expect(stats.checkedItems).toBe(0);
    expect(stats.completionRate).toBe(0);
  });

  it("all checked — 100% completion", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(3)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    for (const item of inst.items) {
      s.dispatch(checkItemAction(inst.id, item.templateItemId));
    }
    const stats = computeStats(s.getState().instances[0]!);
    expect(stats.checkedItems).toBe(3);
    expect(stats.completionRate).toBe(100);
  });

  it("counts only visible items (no-branch items excluded)", () => {
    s.dispatch(createTemplateAction("T", "", decisionTemplate()));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    s.dispatch(answerDecisionAction(inst.id, "d1", "yes"));
    // visible check items: c1, c2(yes-branch), c4 = 3 items
    const stats = computeStats(s.getState().instances[0]!);
    expect(stats.totalItems).toBe(3);
    expect(stats.decisionCount).toBe(1);
  });

  it("durationMs is null while in-progress", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(1)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    const stats = computeStats(inst);
    expect(stats.durationMs).toBeNull();
  });

  it("durationMs is set after completion", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(1)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    s.dispatch(completeInstanceAction(inst.id));
    const stats = computeStats(s.getState().instances[0]!);
    expect(stats.durationMs).not.toBeNull();
    expect(stats.durationMs).toBeGreaterThanOrEqual(0);
  });
});

// ── completeInstance / abandonInstance ────────────────────────────────────

describe("completeInstance / abandonInstance", () => {
  let s: Store<AppState>;
  beforeEach(() => {
    s = createStore();
  });

  it("completeInstance sets status = completed and completedAt", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(1)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    const before = Date.now();
    s.dispatch(completeInstanceAction(inst.id));
    const updated = s.getState().instances[0]!;
    expect(updated.status).toBe("completed");
    expect(updated.completedAt).not.toBeNull();
    expect(updated.completedAt!).toBeGreaterThanOrEqual(before);
  });

  it("abandonInstance sets status = abandoned and completedAt", () => {
    s.dispatch(createTemplateAction("T", "", flatTemplate(1)));
    const templateId = s.getState().templates[0]!.id;
    s.dispatch(createInstanceAction(templateId));
    const inst = s.getState().instances[0]!;
    s.dispatch(abandonInstanceAction(inst.id));
    const updated = s.getState().instances[0]!;
    expect(updated.status).toBe("abandoned");
    expect(updated.completedAt).not.toBeNull();
  });

  it("getInstance returns undefined for unknown id", () => {
    const state = s.getState();
    expect(state.instances.find((i) => i.id === "no-such")).toBeUndefined();
  });
});

// ── Todo actions ──────────────────────────────────────────────────────────

describe("Todo actions", () => {
  let s: Store<AppState>;
  beforeEach(() => {
    s = createStore();
  });

  it("TODO_CREATE creates a todo with status 'todo' and timestamps", () => {
    const before = Date.now();
    s.dispatch(createTodoAction("Buy milk", "From the store"));
    const todo = s.getState().todos[0]!;
    expect(todo.title).toBe("Buy milk");
    expect(todo.description).toBe("From the store");
    expect(todo.status).toBe("todo");
    expect(todo.createdAt).toBeGreaterThanOrEqual(before);
    expect(todo.updatedAt).toBeGreaterThanOrEqual(before);
    expect(todo.id).toBeTruthy();
  });

  it("TODO_UPDATE changes title and description", () => {
    s.dispatch(createTodoAction("Old title", "Old desc"));
    const todoId = s.getState().todos[0]!.id;
    s.dispatch(updateTodoAction(todoId, { title: "New title", description: "New desc" }));
    const todo = s.getState().todos[0]!;
    expect(todo.title).toBe("New title");
    expect(todo.description).toBe("New desc");
  });

  it("TODO_DELETE removes a todo", () => {
    s.dispatch(createTodoAction("Task", ""));
    const todoId = s.getState().todos[0]!.id;
    s.dispatch(deleteTodoAction(todoId));
    expect(s.getState().todos).toHaveLength(0);
  });

  it("TODO_TOGGLE cycles: todo -> in-progress -> done -> todo", () => {
    s.dispatch(createTodoAction("Task", ""));
    const todoId = s.getState().todos[0]!.id;

    s.dispatch(toggleTodoAction(todoId));
    expect(s.getState().todos[0]!.status).toBe("in-progress");

    s.dispatch(toggleTodoAction(todoId));
    expect(s.getState().todos[0]!.status).toBe("done");

    s.dispatch(toggleTodoAction(todoId));
    expect(s.getState().todos[0]!.status).toBe("todo");
  });

  it("TODO_TOGGLE updates updatedAt timestamp", () => {
    s.dispatch(createTodoAction("Task", ""));
    const todoId = s.getState().todos[0]!.id;
    const beforeToggle = s.getState().todos[0]!.updatedAt;
    s.dispatch(toggleTodoAction(todoId));
    const afterToggle = s.getState().todos[0]!.updatedAt;
    expect(afterToggle).toBeGreaterThanOrEqual(beforeToggle);
  });
});
