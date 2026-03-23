/**
 * reducer.ts — Pure reducer function and utility helpers.
 *
 * The reducer is the heart of the Flux architecture. It takes the current
 * state and an action, and returns a NEW state object. It must be pure:
 *
 *   - No side effects (no I/O, no randomness, no DOM access)
 *   - Same inputs always produce the same outputs
 *   - Never mutate the input state — always return a new object
 *
 * === Why immutability? ===
 *
 * React's useSyncExternalStore (used by our useStore hook) compares
 * state snapshots by reference (Object.is). If the reducer mutates the
 * existing state object instead of returning a new one, React won't
 * detect the change and won't re-render. Returning a new object on every
 * change ensures React sees a different reference and triggers re-render.
 *
 * === Structure ===
 *
 * The reducer uses a switch statement on action.type. Each case handles
 * one kind of state transition. The default case returns the current
 * state unchanged (unknown actions are no-ops).
 *
 * Utility functions (cloneItems, findInstanceItem, flattenVisibleItems,
 * computeStats, countBranchItems) are pure helpers exported for use by
 * components and tests. They perform tree-walking and computation but
 * never mutate state.
 */

import type { Action } from "@coding-adventures/store";
import type {
  Template,
  TemplateItem,
  Instance,
  InstanceItem,
  CheckInstanceItem,
  DecisionInstanceItem,
  DecisionAnswer,
  InstanceStats,
  TodoItem,
  TodoStatus,
} from "./types.js";
import {
  TEMPLATE_CREATE,
  TEMPLATE_UPDATE,
  TEMPLATE_DELETE,
  INSTANCE_CREATE,
  INSTANCE_CHECK,
  INSTANCE_UNCHECK,
  INSTANCE_ANSWER,
  INSTANCE_COMPLETE,
  INSTANCE_ABANDON,
  STATE_LOAD,
  TODO_CREATE,
  TODO_UPDATE,
  TODO_DELETE,
  TODO_TOGGLE,
} from "./actions.js";

// ── AppState ───────────────────────────────────────────────────────────────

export interface AppState {
  templates: Template[];
  instances: Instance[];
  todos: TodoItem[];
}

// ── ID generation ──────────────────────────────────────────────────────────

/**
 * generateId produces a UUID-like string.
 * Uses crypto.randomUUID() when available (browsers, Node >= 19), otherwise
 * falls back to a timestamp + random suffix — sufficient for in-memory IDs
 * that never leave the browser session.
 */
function generateId(): string {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

// ── Tree utilities ─────────────────────────────────────────────────────────

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
export function cloneItems(templateItems: TemplateItem[]): InstanceItem[] {
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

/**
 * updateItemInTree — immutably updates a single item in the instance item tree.
 *
 * Because the reducer must not mutate state, we can't just find the item
 * and flip a field. Instead, we rebuild the tree path from root to the
 * target node, creating new objects along the way. Items not on the path
 * are reused by reference (structural sharing).
 *
 * This is the same approach Redux Toolkit's immer uses under the hood,
 * but done explicitly here for educational clarity.
 */
function updateItemInTree(
  items: InstanceItem[],
  templateItemId: string,
  updater: (item: InstanceItem) => InstanceItem,
): InstanceItem[] {
  return items.map((item): InstanceItem => {
    if (item.templateItemId === templateItemId) {
      return updater(item);
    }
    if (item.type === "decision") {
      const newYes = updateItemInTree(item.yesBranch, templateItemId, updater);
      const newNo = updateItemInTree(item.noBranch, templateItemId, updater);
      // Only create a new object if a branch actually changed
      if (newYes !== item.yesBranch || newNo !== item.noBranch) {
        return { ...item, yesBranch: newYes, noBranch: newNo };
      }
    }
    return item;
  });
}

/**
 * updateInstance — immutably updates a single instance in the instances array.
 *
 * Returns a new array with the target instance replaced by the result of
 * the updater function. Other instances are reused by reference.
 */
function updateInstance(
  instances: Instance[],
  instanceId: string,
  updater: (inst: Instance) => Instance,
): Instance[] {
  return instances.map((inst) =>
    inst.id === instanceId ? updater(inst) : inst,
  );
}

// ── Reducer ─────────────────────────────────────────────────────────────────

/**
 * reducer — the pure function that computes the next state.
 *
 * Each case in the switch handles one action type and returns a new
 * AppState object. The default case returns the current state unchanged.
 */
export function reducer(state: AppState, action: Action): AppState {
  switch (action.type) {
    // ── Template actions ────────────────────────────────────────────────

    case TEMPLATE_CREATE: {
      const template: Template = {
        id: generateId(),
        name: action.name as string,
        description: action.description as string,
        createdAt: Date.now(),
        items: action.items as TemplateItem[],
      };
      return {
        ...state,
        templates: [...state.templates, template],
      };
    }

    case TEMPLATE_UPDATE: {
      const id = action.id as string;
      const patch = action.patch as Partial<Omit<Template, "id" | "createdAt">>;
      return {
        ...state,
        templates: state.templates.map((t) =>
          t.id === id
            ? {
                ...t,
                ...(patch.name !== undefined ? { name: patch.name } : {}),
                ...(patch.description !== undefined
                  ? { description: patch.description }
                  : {}),
                ...(patch.items !== undefined ? { items: patch.items } : {}),
              }
            : t,
        ),
      };
    }

    case TEMPLATE_DELETE: {
      const id = action.id as string;
      return {
        ...state,
        templates: state.templates.filter((t) => t.id !== id),
      };
    }

    // ── Instance actions ────────────────────────────────────────────────

    case INSTANCE_CREATE: {
      const templateId = action.templateId as string;
      const template = state.templates.find((t) => t.id === templateId);
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
      return {
        ...state,
        instances: [...state.instances, instance],
      };
    }

    case INSTANCE_CHECK: {
      const { instanceId, templateItemId } = action as {
        type: string;
        instanceId: string;
        templateItemId: string;
      };
      const inst = state.instances.find((i) => i.id === instanceId);
      if (!inst) return state;
      const item = findInstanceItem(inst.items, templateItemId);
      if (!item || item.type !== "check") return state;
      return {
        ...state,
        instances: updateInstance(state.instances, instanceId, (i) => ({
          ...i,
          items: updateItemInTree(i.items, templateItemId, (it) =>
            it.type === "check" ? { ...it, checked: true } : it,
          ),
        })),
      };
    }

    case INSTANCE_UNCHECK: {
      const { instanceId, templateItemId } = action as {
        type: string;
        instanceId: string;
        templateItemId: string;
      };
      const inst = state.instances.find((i) => i.id === instanceId);
      if (!inst) return state;
      const item = findInstanceItem(inst.items, templateItemId);
      if (!item || item.type !== "check") return state;
      return {
        ...state,
        instances: updateInstance(state.instances, instanceId, (i) => ({
          ...i,
          items: updateItemInTree(i.items, templateItemId, (it) =>
            it.type === "check" ? { ...it, checked: false } : it,
          ),
        })),
      };
    }

    case INSTANCE_ANSWER: {
      const { instanceId, templateItemId, answer } = action as {
        type: string;
        instanceId: string;
        templateItemId: string;
        answer: DecisionAnswer;
      };
      const inst = state.instances.find((i) => i.id === instanceId);
      if (!inst) return state;
      const item = findInstanceItem(inst.items, templateItemId);
      if (!item || item.type !== "decision") return state;
      return {
        ...state,
        instances: updateInstance(state.instances, instanceId, (i) => ({
          ...i,
          items: updateItemInTree(i.items, templateItemId, (it) =>
            it.type === "decision" ? { ...it, answer } : it,
          ),
        })),
      };
    }

    case INSTANCE_COMPLETE: {
      const instanceId = action.instanceId as string;
      return {
        ...state,
        instances: updateInstance(state.instances, instanceId, (i) => ({
          ...i,
          status: "completed" as const,
          completedAt: Date.now(),
        })),
      };
    }

    case INSTANCE_ABANDON: {
      const instanceId = action.instanceId as string;
      return {
        ...state,
        instances: updateInstance(state.instances, instanceId, (i) => ({
          ...i,
          status: "abandoned" as const,
          completedAt: Date.now(),
        })),
      };
    }

    // ── Bulk load (startup) ─────────────────────────────────────────────

    case STATE_LOAD: {
      return {
        templates: (action.templates as Template[]) ?? [],
        instances: (action.instances as Instance[]) ?? [],
        todos: (action.todos as TodoItem[]) ?? [],
      };
    }

    // ── Todo actions ─────────────────────────────────────────────────

    case TODO_CREATE: {
      const now = Date.now();
      const todo: TodoItem = {
        id: generateId(),
        title: action.title as string,
        description: (action.description as string) ?? "",
        status: "todo",
        dueDate: (action.dueDate as string | null) ?? null,
        createdAt: now,
        updatedAt: now,
      };
      return { ...state, todos: [...state.todos, todo] };
    }

    case TODO_UPDATE: {
      const todoId = action.todoId as string;
      const patch = action.patch as Partial<TodoItem>;
      return {
        ...state,
        todos: state.todos.map((t) =>
          t.id === todoId ? { ...t, ...patch, updatedAt: Date.now() } : t
        ),
      };
    }

    case TODO_DELETE: {
      const todoId = action.todoId as string;
      return {
        ...state,
        todos: state.todos.filter((t) => t.id !== todoId),
      };
    }

    case TODO_TOGGLE: {
      const todoId = action.todoId as string;
      const statusCycle: Record<TodoStatus, TodoStatus> = {
        "todo": "in-progress",
        "in-progress": "done",
        "done": "todo",
      };
      return {
        ...state,
        todos: state.todos.map((t) =>
          t.id === todoId ? { ...t, status: statusCycle[t.status], updatedAt: Date.now() } : t
        ),
      };
    }

    default:
      return state;
  }
}

// ── Tree algorithms (pure utility functions) ──────────────────────────────
//
// These are not actions or reducers — they are pure functions that compute
// derived data from an instance's item tree. Components use them for
// rendering (flattenVisibleItems) and statistics (computeStats).

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
 * completionRate = (checkedItems / totalItems) * 100, clamped to 0-100.
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
