/**
 * types.ts — All TypeScript interfaces for the checklist app.
 *
 * The data model separates immutable *templates* (reusable procedure
 * definitions) from mutable *instances* (one execution run of a template).
 * Think of a Template as a class and an Instance as an object.
 *
 * Both layers use a discriminated union for items:
 *   type TemplateItem = CheckTemplateItem | DecisionTemplateItem
 *
 * TypeScript's exhaustive narrowing means a `switch (item.type)` with no
 * default is a compile-time proof that every variant is handled. This is
 * the key reason to prefer discriminated unions over a flat `kind: string`
 * field — the compiler catches missing cases.
 */

// ── Template types (immutable, authored once) ─────────────────────────────
//
// A CheckTemplateItem is the simplest node: a step that must be ticked off.
// A DecisionTemplateItem is a yes/no question whose answer determines which
// branch of the procedure to follow. Both branches are arrays of further
// TemplateItems, so the tree can be arbitrarily deep.

export interface CheckTemplateItem {
  id: string;
  type: "check";
  label: string;
}

export interface DecisionTemplateItem {
  id: string;
  type: "decision";
  /** The question to display — e.g. "Did smoke tests pass?" */
  label: string;
  yesBranch: TemplateItem[];
  noBranch: TemplateItem[];
}

export type TemplateItem = CheckTemplateItem | DecisionTemplateItem;

export interface Template {
  id: string;
  name: string;
  description: string;
  /** Unix timestamp from Date.now() */
  createdAt: number;
  items: TemplateItem[];
}

// ── Instance types (mutable, one per run) ─────────────────────────────────
//
// An Instance is created by deep-cloning a Template's item tree into a
// parallel tree of InstanceItems. Each node carries the same label as its
// template counterpart, plus mutable execution state (checked / answer).
//
// Two instances of the same template are fully independent — editing one
// does not affect the other.

export type DecisionAnswer = "yes" | "no" | null;
export type InstanceStatus = "in-progress" | "completed" | "abandoned";

export interface CheckInstanceItem {
  /** Reference back to the template item this was cloned from. */
  templateItemId: string;
  type: "check";
  label: string;
  checked: boolean;
}

export interface DecisionInstanceItem {
  templateItemId: string;
  type: "decision";
  label: string;
  /** null = unanswered; once set, the chosen branch becomes visible. */
  answer: DecisionAnswer;
  yesBranch: InstanceItem[];
  noBranch: InstanceItem[];
}

export type InstanceItem = CheckInstanceItem | DecisionInstanceItem;

export interface Instance {
  id: string;
  templateId: string;
  templateName: string;
  status: InstanceStatus;
  createdAt: number;
  completedAt: number | null;
  items: InstanceItem[];
}

// ── Stats (computed on demand, never stored) ──────────────────────────────
//
// Stats are a pure function of the final InstanceItem tree. They are
// recomputed on every render of the Stats view — no stale data possible.
//
// totalItems counts only *check* items that were visible (on active branches).
// Decision items are navigational; they are counted separately as decisionCount.
// completionRate = (checkedItems / totalItems) * 100, clamped 0–100.

export interface InstanceStats {
  totalItems: number;
  checkedItems: number;
  decisionCount: number;
  /** 0–100 */
  completionRate: number;
  /** null while in-progress */
  durationMs: number | null;
}

// ── Type guards ────────────────────────────────────────────────────────────
//
// These narrow the union types without relying on `as` casts. They are the
// single authoritative place where we check `item.type`. All tree-walking
// code should use these rather than repeating the string comparison.

export function isCheckItem(
  item: TemplateItem | InstanceItem,
): item is CheckTemplateItem | CheckInstanceItem {
  return item.type === "check";
}

export function isDecisionItem(
  item: TemplateItem | InstanceItem,
): item is DecisionTemplateItem | DecisionInstanceItem {
  return item.type === "decision";
}

export function isCheckInstanceItem(
  item: InstanceItem,
): item is CheckInstanceItem {
  return item.type === "check";
}

export function isDecisionInstanceItem(
  item: InstanceItem,
): item is DecisionInstanceItem {
  return item.type === "decision";
}
