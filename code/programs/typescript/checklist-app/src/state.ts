/**
 * state.ts — In-memory store and all pure state functions.
 *
 * Architecture: one module-level AppState singleton. All mutations are
 * plain functions that take `state` as their first argument — no classes,
 * no React context, no Redux. This makes every function trivially testable
 * in Node without a DOM.
 *
 * The two most important functions:
 *
 *   createInstance   — deep-clones a Template item tree into a parallel
 *                      InstanceItem tree with fresh mutable state on each node.
 *
 *   flattenVisibleItems — walks the InstanceItem tree and returns the linear
 *                         list of items the user currently sees. Decisions
 *                         only reveal their branch after being answered.
 *
 * Everything else (check, uncheck, answer, complete, abandon) is a simple
 * mutation on the already-cloned instance tree.
 */

import type {
  Template,
  TemplateItem,
  Instance,
  InstanceItem,
  CheckInstanceItem,
  DecisionInstanceItem,
  DecisionAnswer,
  InstanceStats,
} from "./types.js";

// Re-export types consumed by tests so they have a single import path.
export type {
  Template,
  TemplateItem,
  Instance,
  InstanceItem,
  CheckInstanceItem,
  DecisionInstanceItem,
  InstanceStats,
};

// ── AppState ───────────────────────────────────────────────────────────────

export interface AppState {
  templates: Template[];
  instances: Instance[];
}

/**
 * createState returns a fresh empty store.
 * Tests call this in beforeEach so they never share state between cases.
 * The app calls it once at startup.
 */
export function createState(): AppState {
  return { templates: [], instances: [] };
}

/** The singleton used by the running app. Components import this directly. */
export const appState: AppState = createState();

// ── ID generation ──────────────────────────────────────────────────────────

/**
 * generateId produces a UUID-like string.
 * Uses crypto.randomUUID() when available (browsers, Node ≥ 19), otherwise
 * falls back to a timestamp + random suffix — sufficient for in-memory IDs
 * that never leave the browser session.
 */
function generateId(): string {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

// ── Template operations ────────────────────────────────────────────────────

export function createTemplate(
  state: AppState,
  name: string,
  description: string,
  items: TemplateItem[],
): Template {
  const template: Template = {
    id: generateId(),
    name,
    description,
    createdAt: Date.now(),
    items,
  };
  state.templates.push(template);
  return template;
}

export function updateTemplate(
  state: AppState,
  id: string,
  patch: Partial<Omit<Template, "id" | "createdAt">>,
): void {
  const template = state.templates.find((t) => t.id === id);
  if (!template) return;
  if (patch.name !== undefined) template.name = patch.name;
  if (patch.description !== undefined) template.description = patch.description;
  if (patch.items !== undefined) template.items = patch.items;
}

export function deleteTemplate(state: AppState, id: string): void {
  const idx = state.templates.findIndex((t) => t.id === id);
  if (idx !== -1) state.templates.splice(idx, 1);
}

export function getTemplate(
  state: AppState,
  id: string,
): Template | undefined {
  return state.templates.find((t) => t.id === id);
}

// ── Instance creation (the deep-clone) ────────────────────────────────────

/**
 * cloneItems recursively converts a TemplateItem[] into an InstanceItem[].
 *
 * For each check item: copy label, set checked = false.
 * For each decision item: copy label, set answer = null, recursively clone
 * both branches.
 *
 * The `templateItemId` field on each node records which template item it
 * was cloned from — used by checkItem / answerDecision to find the node.
 */
function cloneItems(templateItems: TemplateItem[]): InstanceItem[] {
  return templateItems.map((item): InstanceItem => {
    if (item.type === "check") {
      return {
        templateItemId: item.id,
        type: "check",
        label: item.label,
        checked: false,
      };
    } else {
      return {
        templateItemId: item.id,
        type: "decision",
        label: item.label,
        answer: null,
        yesBranch: cloneItems(item.yesBranch),
        noBranch: cloneItems(item.noBranch),
      };
    }
  });
}

export function createInstance(state: AppState, templateId: string): Instance {
  const template = getTemplate(state, templateId);
  if (!template) {
    throw new Error(`No template with id "${templateId}"`);
  }
  const instance: Instance = {
    id: generateId(),
    templateId: template.id,
    templateName: template.name,
    status: "in-progress",
    createdAt: Date.now(),
    completedAt: null,
    items: cloneItems(template.items),
  };
  state.instances.push(instance);
  return instance;
}

export function getInstance(
  state: AppState,
  id: string,
): Instance | undefined {
  return state.instances.find((i) => i.id === id);
}

// ── Item mutations ─────────────────────────────────────────────────────────

/**
 * findInstanceItem searches the InstanceItem tree (including all decision
 * branches) for an item with the given templateItemId.
 *
 * This is a depth-first search that must look inside every branch, not just
 * the visible path. That way checkItem works even for items currently hidden
 * (e.g., pre-checking a branch item before the decision is answered).
 */
function findInstanceItem(
  items: InstanceItem[],
  templateItemId: string,
): InstanceItem | undefined {
  for (const item of items) {
    if (item.templateItemId === templateItemId) return item;
    if (item.type === "decision") {
      const inYes = findInstanceItem(item.yesBranch, templateItemId);
      if (inYes) return inYes;
      const inNo = findInstanceItem(item.noBranch, templateItemId);
      if (inNo) return inNo;
    }
  }
  return undefined;
}

export function checkItem(
  state: AppState,
  instanceId: string,
  templateItemId: string,
): void {
  const instance = getInstance(state, instanceId);
  if (!instance) return;
  const item = findInstanceItem(instance.items, templateItemId);
  if (item?.type === "check") item.checked = true;
}

export function uncheckItem(
  state: AppState,
  instanceId: string,
  templateItemId: string,
): void {
  const instance = getInstance(state, instanceId);
  if (!instance) return;
  const item = findInstanceItem(instance.items, templateItemId);
  if (item?.type === "check") item.checked = false;
}

export function answerDecision(
  state: AppState,
  instanceId: string,
  templateItemId: string,
  answer: DecisionAnswer,
): void {
  const instance = getInstance(state, instanceId);
  if (!instance) return;
  const item = findInstanceItem(instance.items, templateItemId);
  if (item?.type === "decision") item.answer = answer;
}

export function completeInstance(state: AppState, instanceId: string): void {
  const instance = getInstance(state, instanceId);
  if (!instance) return;
  instance.status = "completed";
  instance.completedAt = Date.now();
}

export function abandonInstance(state: AppState, instanceId: string): void {
  const instance = getInstance(state, instanceId);
  if (!instance) return;
  instance.status = "abandoned";
  instance.completedAt = Date.now();
}

// ── Tree algorithms ────────────────────────────────────────────────────────

/**
 * flattenVisibleItems — the core decision-tree walk.
 *
 * Returns the ordered list of items the user currently sees. The rule:
 *
 *   1. Include every top-level item in the list.
 *   2. For a check item: no recursion needed (no children).
 *   3. For a decision item with answer = null: include the decision itself,
 *      but do NOT recurse into either branch yet.
 *   4. For a decision item with an answer: include the decision, then
 *      recursively flatten the chosen branch and include those items.
 *
 * Note: items *after* a decision in the same list are always included.
 * The decision only gates its own branches — it does not gate siblings.
 * This matches the aviation checklist model: an unanswered question
 * blocks the branch below it, not the rest of the page.
 */
export function flattenVisibleItems(items: InstanceItem[]): InstanceItem[] {
  const result: InstanceItem[] = [];
  for (const item of items) {
    result.push(item);
    if (item.type === "decision" && item.answer !== null) {
      const branch = item.answer === "yes" ? item.yesBranch : item.noBranch;
      result.push(...flattenVisibleItems(branch));
    }
  }
  return result;
}

/**
 * computeStats — pure function over an instance's final state.
 *
 * Only counts items returned by flattenVisibleItems (i.e., items the user
 * actually encountered). Decision items are counted separately as they are
 * navigational, not checkable.
 *
 * completionRate = (checkedItems / totalItems) * 100, clamped to 0–100.
 * Returns 0% (not NaN) when there are no check items.
 */
export function computeStats(instance: Instance): InstanceStats {
  const visible = flattenVisibleItems(instance.items);
  let totalItems = 0;
  let checkedItems = 0;
  let decisionCount = 0;

  for (const item of visible) {
    if (item.type === "check") {
      totalItems++;
      if (item.checked) checkedItems++;
    } else {
      decisionCount++;
    }
  }

  const completionRate =
    totalItems === 0 ? 0 : Math.min(100, (checkedItems / totalItems) * 100);

  const durationMs =
    instance.completedAt !== null
      ? instance.completedAt - instance.createdAt
      : null;

  return { totalItems, checkedItems, decisionCount, completionRate, durationMs };
}

/**
 * countBranchItems — counts items in a branch for the collapsed summary.
 *
 * Recursively walks the InstanceItem tree and returns counts of check items
 * and decision items. Used by the tree view's inactive branch summary:
 * "3 steps" or "3 steps, 1 decision".
 */
export function countBranchItems(items: InstanceItem[]): {
  checks: number;
  decisions: number;
} {
  let checks = 0;
  let decisions = 0;
  for (const item of items) {
    if (item.type === "check") {
      checks++;
    } else {
      decisions++;
      checks += countBranchItems(item.yesBranch).checks;
      decisions += countBranchItems(item.yesBranch).decisions;
      checks += countBranchItems(item.noBranch).checks;
      decisions += countBranchItems(item.noBranch).decisions;
    }
  }
  return { checks, decisions };
}
