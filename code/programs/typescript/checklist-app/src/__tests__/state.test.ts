/**
 * state.test.ts — Unit tests for all state mutations and tree algorithms.
 *
 * These tests run in Node (no DOM needed) because state.ts is pure logic.
 * The most critical functions are flattenVisibleItems (the decision-tree
 * walk) and createInstance (the deep-clone). Getting these right is what
 * makes the entire app correct.
 */

import { describe, it, expect, beforeEach } from "vitest";
import {
  createState,
  createTemplate,
  updateTemplate,
  deleteTemplate,
  getTemplate,
  createInstance,
  getInstance,
  checkItem,
  uncheckItem,
  answerDecision,
  completeInstance,
  abandonInstance,
  flattenVisibleItems,
  computeStats,
} from "../state.js";
import type {
  AppState,
  CheckInstanceItem,
  DecisionInstanceItem,
} from "../state.js";
import type { TemplateItem } from "../types.js";

// ── Helpers ────────────────────────────────────────────────────────────────

/** Builds a minimal flat template with N check items. */
function flatTemplate(n: number): TemplateItem[] {
  return Array.from({ length: n }, (_, i) => ({
    id: `item-${i}`,
    type: "check" as const,
    label: `Step ${i + 1}`,
  }));
}

/** Builds a template item tree: check → decision(yes: check, no: check) → check */
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
  let state: AppState;
  beforeEach(() => {
    state = createState();
  });

  it("returns a Template with the given name and description", () => {
    const t = createTemplate(state, "My Checklist", "A test", []);
    expect(t.name).toBe("My Checklist");
    expect(t.description).toBe("A test");
  });

  it("assigns a unique id", () => {
    const a = createTemplate(state, "A", "", []);
    const b = createTemplate(state, "B", "", []);
    expect(a.id).not.toBe(b.id);
  });

  it("sets createdAt to a recent timestamp", () => {
    const before = Date.now();
    const t = createTemplate(state, "T", "", []);
    expect(t.createdAt).toBeGreaterThanOrEqual(before);
    expect(t.createdAt).toBeLessThanOrEqual(Date.now());
  });

  it("stores the template in state.templates", () => {
    createTemplate(state, "T", "", []);
    expect(state.templates).toHaveLength(1);
  });

  it("stores the provided items", () => {
    const items = flatTemplate(3);
    const t = createTemplate(state, "T", "", items);
    expect(t.items).toHaveLength(3);
  });
});

// ── updateTemplate ─────────────────────────────────────────────────────────

describe("updateTemplate", () => {
  let state: AppState;
  beforeEach(() => {
    state = createState();
  });

  it("updates the name of an existing template", () => {
    const t = createTemplate(state, "Old", "", []);
    updateTemplate(state, t.id, { name: "New" });
    expect(getTemplate(state, t.id)?.name).toBe("New");
  });

  it("does not change unpatched fields", () => {
    const t = createTemplate(state, "Name", "Desc", []);
    updateTemplate(state, t.id, { name: "New" });
    expect(getTemplate(state, t.id)?.description).toBe("Desc");
  });

  it("is a no-op for unknown id", () => {
    createTemplate(state, "T", "", []);
    expect(() => updateTemplate(state, "unknown", { name: "X" })).not.toThrow();
    expect(state.templates[0]?.name).toBe("T");
  });
});

// ── deleteTemplate ─────────────────────────────────────────────────────────

describe("deleteTemplate", () => {
  let state: AppState;
  beforeEach(() => {
    state = createState();
  });

  it("removes the template from state", () => {
    const t = createTemplate(state, "T", "", []);
    deleteTemplate(state, t.id);
    expect(state.templates).toHaveLength(0);
  });

  it("is a no-op for unknown id", () => {
    createTemplate(state, "T", "", []);
    expect(() => deleteTemplate(state, "unknown")).not.toThrow();
    expect(state.templates).toHaveLength(1);
  });
});

// ── createInstance ─────────────────────────────────────────────────────────

describe("createInstance", () => {
  let state: AppState;
  beforeEach(() => {
    state = createState();
  });

  it("creates an instance with in-progress status", () => {
    const t = createTemplate(state, "T", "", flatTemplate(2));
    const inst = createInstance(state, t.id);
    expect(inst.status).toBe("in-progress");
    expect(inst.completedAt).toBeNull();
  });

  it("copies templateId and templateName", () => {
    const t = createTemplate(state, "My List", "", []);
    const inst = createInstance(state, t.id);
    expect(inst.templateId).toBe(t.id);
    expect(inst.templateName).toBe("My List");
  });

  it("clones check items preserving label", () => {
    const items = flatTemplate(3);
    const t = createTemplate(state, "T", "", items);
    const inst = createInstance(state, t.id);
    expect(inst.items).toHaveLength(3);
    expect((inst.items[0] as CheckInstanceItem).label).toBe("Step 1");
  });

  it("initializes check items with checked = false", () => {
    const t = createTemplate(state, "T", "", flatTemplate(2));
    const inst = createInstance(state, t.id);
    for (const item of inst.items) {
      expect((item as CheckInstanceItem).checked).toBe(false);
    }
  });

  it("deep-clones decision items with answer = null", () => {
    const t = createTemplate(state, "T", "", decisionTemplate());
    const inst = createInstance(state, t.id);
    const decision = inst.items[1] as DecisionInstanceItem;
    expect(decision.type).toBe("decision");
    expect(decision.answer).toBeNull();
  });

  it("clones decision branches recursively", () => {
    const t = createTemplate(state, "T", "", decisionTemplate());
    const inst = createInstance(state, t.id);
    const decision = inst.items[1] as DecisionInstanceItem;
    expect(decision.yesBranch).toHaveLength(1);
    expect(decision.noBranch).toHaveLength(1);
    expect(decision.yesBranch[0]?.label).toBe("Yes: Celebrate");
    expect(decision.noBranch[0]?.label).toBe("No: Rollback");
  });

  it("two instances are independent (mutations don't bleed)", () => {
    const t = createTemplate(state, "T", "", flatTemplate(2));
    const inst1 = createInstance(state, t.id);
    const inst2 = createInstance(state, t.id);
    checkItem(state, inst1.id, inst1.items[0]?.templateItemId ?? "");
    expect((inst1.items[0] as CheckInstanceItem).checked).toBe(true);
    expect((inst2.items[0] as CheckInstanceItem).checked).toBe(false);
  });

  it("throws for unknown templateId", () => {
    expect(() => createInstance(state, "no-such-id")).toThrow();
  });
});

// ── checkItem / uncheckItem ────────────────────────────────────────────────

describe("checkItem / uncheckItem", () => {
  let state: AppState;
  beforeEach(() => {
    state = createState();
  });

  it("marks a check item as checked", () => {
    const t = createTemplate(state, "T", "", flatTemplate(2));
    const inst = createInstance(state, t.id);
    const itemId = inst.items[0]?.templateItemId ?? "";
    checkItem(state, inst.id, itemId);
    expect((inst.items[0] as CheckInstanceItem).checked).toBe(true);
  });

  it("checkItem is idempotent", () => {
    const t = createTemplate(state, "T", "", flatTemplate(1));
    const inst = createInstance(state, t.id);
    const itemId = inst.items[0]?.templateItemId ?? "";
    checkItem(state, inst.id, itemId);
    checkItem(state, inst.id, itemId);
    expect((inst.items[0] as CheckInstanceItem).checked).toBe(true);
  });

  it("uncheckItem clears a checked item", () => {
    const t = createTemplate(state, "T", "", flatTemplate(1));
    const inst = createInstance(state, t.id);
    const itemId = inst.items[0]?.templateItemId ?? "";
    checkItem(state, inst.id, itemId);
    uncheckItem(state, inst.id, itemId);
    expect((inst.items[0] as CheckInstanceItem).checked).toBe(false);
  });

  it("can check an item inside a decision branch", () => {
    const t = createTemplate(state, "T", "", decisionTemplate());
    const inst = createInstance(state, t.id);
    // Answer the decision yes first
    answerDecision(state, inst.id, "d1", "yes");
    const decision = inst.items[1] as DecisionInstanceItem;
    const branchItemId = decision.yesBranch[0]?.templateItemId ?? "";
    checkItem(state, inst.id, branchItemId);
    expect((decision.yesBranch[0] as CheckInstanceItem).checked).toBe(true);
  });
});

// ── answerDecision ─────────────────────────────────────────────────────────

describe("answerDecision", () => {
  let state: AppState;
  beforeEach(() => {
    state = createState();
  });

  it("records yes answer", () => {
    const t = createTemplate(state, "T", "", decisionTemplate());
    const inst = createInstance(state, t.id);
    answerDecision(state, inst.id, "d1", "yes");
    const d = inst.items[1] as DecisionInstanceItem;
    expect(d.answer).toBe("yes");
  });

  it("records no answer", () => {
    const t = createTemplate(state, "T", "", decisionTemplate());
    const inst = createInstance(state, t.id);
    answerDecision(state, inst.id, "d1", "no");
    const d = inst.items[1] as DecisionInstanceItem;
    expect(d.answer).toBe("no");
  });

  it("can change answer from yes to no", () => {
    const t = createTemplate(state, "T", "", decisionTemplate());
    const inst = createInstance(state, t.id);
    answerDecision(state, inst.id, "d1", "yes");
    answerDecision(state, inst.id, "d1", "no");
    const d = inst.items[1] as DecisionInstanceItem;
    expect(d.answer).toBe("no");
  });
});

// ── flattenVisibleItems ────────────────────────────────────────────────────

describe("flattenVisibleItems", () => {
  let state: AppState;
  beforeEach(() => {
    state = createState();
  });

  it("flat list of check items — returns all", () => {
    const t = createTemplate(state, "T", "", flatTemplate(4));
    const inst = createInstance(state, t.id);
    const visible = flattenVisibleItems(inst.items);
    expect(visible).toHaveLength(4);
  });

  it("unanswered decision — includes decision item, stops before branches", () => {
    const t = createTemplate(state, "T", "", decisionTemplate());
    const inst = createInstance(state, t.id);
    // items: [check, decision, check]
    const visible = flattenVisibleItems(inst.items);
    // Should see: check(c1), decision(d1). NOT c4 (after decision) yet.
    // But also NOT c2/c3 (inside branches) yet.
    // Decision stops branch descent but c4 is a top-level item after the decision.
    // Per spec: flattenVisibleItems stops descending INTO the decision branches,
    // but continues with top-level siblings after the decision.
    // So visible = [c1, d1(unanswered), c4]
    expect(visible).toHaveLength(3);
    expect(visible[0]?.type).toBe("check");
    expect(visible[1]?.type).toBe("decision");
    expect(visible[2]?.type).toBe("check");
  });

  it("answered yes — yes-branch items visible, no-branch hidden", () => {
    const t = createTemplate(state, "T", "", decisionTemplate());
    const inst = createInstance(state, t.id);
    answerDecision(state, inst.id, "d1", "yes");
    const visible = flattenVisibleItems(inst.items);
    // [c1, d1(yes), c2(yes-branch), c4]
    expect(visible).toHaveLength(4);
    const labels = visible.map((i) => i.label);
    expect(labels).toContain("Yes: Celebrate");
    expect(labels).not.toContain("No: Rollback");
  });

  it("answered no — no-branch items visible, yes-branch hidden", () => {
    const t = createTemplate(state, "T", "", decisionTemplate());
    const inst = createInstance(state, t.id);
    answerDecision(state, inst.id, "d1", "no");
    const visible = flattenVisibleItems(inst.items);
    // [c1, d1(no), c3(no-branch), c4]
    expect(visible).toHaveLength(4);
    const labels = visible.map((i) => i.label);
    expect(labels).toContain("No: Rollback");
    expect(labels).not.toContain("Yes: Celebrate");
  });

  it("nested decision — inner branch only revealed after outer answered", () => {
    // Tree: decision1 → yes: [decision2 → yes: [check-inner]]
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
    const t = createTemplate(state, "T", "", items);
    const inst = createInstance(state, t.id);

    // Before any answer: only outer decision visible
    const v0 = flattenVisibleItems(inst.items);
    expect(v0).toHaveLength(1);
    expect(v0[0]?.label).toBe("Outer?");

    // After answering outer yes: outer + inner decision (inner unanswered)
    answerDecision(state, inst.id, "outer", "yes");
    const v1 = flattenVisibleItems(inst.items);
    expect(v1).toHaveLength(2);
    expect(v1[1]?.label).toBe("Inner?");

    // After answering inner yes: outer + inner + deep check item
    answerDecision(state, inst.id, "inner", "yes");
    const v2 = flattenVisibleItems(inst.items);
    expect(v2).toHaveLength(3);
    expect(v2[2]?.label).toBe("Deep item");
  });
});

// ── computeStats ───────────────────────────────────────────────────────────

describe("computeStats", () => {
  let state: AppState;
  beforeEach(() => {
    state = createState();
  });

  it("fresh instance — 0 checked, 0% completion", () => {
    const t = createTemplate(state, "T", "", flatTemplate(3));
    const inst = createInstance(state, t.id);
    const stats = computeStats(inst);
    expect(stats.totalItems).toBe(3);
    expect(stats.checkedItems).toBe(0);
    expect(stats.completionRate).toBe(0);
  });

  it("all checked — 100% completion", () => {
    const t = createTemplate(state, "T", "", flatTemplate(3));
    const inst = createInstance(state, t.id);
    for (const item of inst.items) {
      checkItem(state, inst.id, item.templateItemId);
    }
    const stats = computeStats(inst);
    expect(stats.checkedItems).toBe(3);
    expect(stats.completionRate).toBe(100);
  });

  it("counts only visible items (no-branch items excluded)", () => {
    const t = createTemplate(state, "T", "", decisionTemplate());
    const inst = createInstance(state, t.id);
    answerDecision(state, inst.id, "d1", "yes");
    // visible check items: c1, c2(yes-branch), c4 = 3 items
    const stats = computeStats(inst);
    expect(stats.totalItems).toBe(3);
    expect(stats.decisionCount).toBe(1);
  });

  it("durationMs is null while in-progress", () => {
    const t = createTemplate(state, "T", "", flatTemplate(1));
    const inst = createInstance(state, t.id);
    const stats = computeStats(inst);
    expect(stats.durationMs).toBeNull();
  });

  it("durationMs is set after completion", () => {
    const t = createTemplate(state, "T", "", flatTemplate(1));
    const inst = createInstance(state, t.id);
    completeInstance(state, inst.id);
    const stats = computeStats(inst);
    expect(stats.durationMs).not.toBeNull();
    expect(stats.durationMs).toBeGreaterThanOrEqual(0);
  });
});

// ── completeInstance / abandonInstance ────────────────────────────────────

describe("completeInstance / abandonInstance", () => {
  let state: AppState;
  beforeEach(() => {
    state = createState();
  });

  it("completeInstance sets status = completed and completedAt", () => {
    const t = createTemplate(state, "T", "", flatTemplate(1));
    const inst = createInstance(state, t.id);
    const before = Date.now();
    completeInstance(state, inst.id);
    expect(inst.status).toBe("completed");
    expect(inst.completedAt).not.toBeNull();
    expect(inst.completedAt!).toBeGreaterThanOrEqual(before);
  });

  it("abandonInstance sets status = abandoned and completedAt", () => {
    const t = createTemplate(state, "T", "", flatTemplate(1));
    const inst = createInstance(state, t.id);
    abandonInstance(state, inst.id);
    expect(inst.status).toBe("abandoned");
    expect(inst.completedAt).not.toBeNull();
  });

  it("getInstance returns undefined for unknown id", () => {
    expect(getInstance(state, "no-such")).toBeUndefined();
  });
});
