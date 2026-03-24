/**
 * actions.ts — Action types and action creator functions.
 *
 * In the Flux architecture, ACTIONS are the only way to change state.
 * An action is a plain object with a `type` field (a string constant)
 * and zero or more payload fields carrying the data needed to perform
 * the state transition.
 *
 * === The Flux Action Pattern ===
 *
 * The pattern comes from Facebook's Flux (2014), later popularized by
 * Redux (2015). The idea is simple but powerful:
 *
 *   1. Something happens (user clicks, data loads, timer fires).
 *   2. An ACTION CREATOR builds a plain object describing what happened.
 *   3. The action is DISPATCHED to the store.
 *   4. The REDUCER reads the action type and payload to compute new state.
 *
 * Why not just mutate state directly? Because:
 *   - Every state change is traceable (you can log every action).
 *   - The reducer is a pure function — easy to test, easy to reason about.
 *   - Middleware can intercept actions for side effects (persistence, logging).
 *   - Time-travel debugging becomes possible (replay actions to reproduce bugs).
 *
 * === Action Creators ===
 *
 * Action creators are factory functions that return action objects.
 * They encapsulate the shape of each action, so the rest of the app
 * doesn't need to know the internal structure. If the payload changes,
 * only the creator and the reducer need updating — not every component
 * that dispatches the action.
 *
 * Example:
 *   // Instead of building the object inline everywhere:
 *   store.dispatch({ type: "INSTANCE_CHECK", instanceId, templateItemId });
 *
 *   // Use a creator:
 *   store.dispatch(checkItemAction(instanceId, templateItemId));
 *
 * The creator validates inputs and provides autocomplete in editors.
 */

import type { Action } from "@coding-adventures/store";
import type { Template, TemplateItem, Instance, DecisionAnswer, TodoStatus, TodoItem } from "./types.js";

// ── Action type constants ──────────────────────────────────────────────────
//
// Each constant is the unique identifier for one kind of state transition.
// Using constants (not raw strings) prevents typos — a misspelled constant
// is a compile error, but a misspelled string is a silent no-op.

export const TEMPLATE_CREATE = "TEMPLATE_CREATE";
export const TEMPLATE_UPDATE = "TEMPLATE_UPDATE";
export const TEMPLATE_DELETE = "TEMPLATE_DELETE";
export const INSTANCE_CREATE = "INSTANCE_CREATE";
export const INSTANCE_CHECK = "INSTANCE_CHECK";
export const INSTANCE_UNCHECK = "INSTANCE_UNCHECK";
export const INSTANCE_ANSWER = "INSTANCE_ANSWER";
export const INSTANCE_COMPLETE = "INSTANCE_COMPLETE";
export const INSTANCE_ABANDON = "INSTANCE_ABANDON";

/**
 * STATE_LOAD — bulk-loads state from IndexedDB on startup.
 *
 * This is dispatched once during initialization to hydrate the store
 * with previously persisted data. It replaces the entire templates
 * and instances arrays in one atomic action.
 */
export const STATE_LOAD = "STATE_LOAD";

export const TODO_CREATE = "TODO_CREATE";
export const TODO_UPDATE = "TODO_UPDATE";
export const TODO_DELETE = "TODO_DELETE";
export const TODO_TOGGLE = "TODO_TOGGLE";

// ── Action creator functions ──────────────────────────────────────────────

/** Create a new template with the given name, description, and item tree. */
export function createTemplateAction(
  name: string,
  description: string,
  items: TemplateItem[],
): Action {
  return { type: TEMPLATE_CREATE, name, description, items };
}

/** Update an existing template's mutable fields (name, description, items). */
export function updateTemplateAction(
  id: string,
  patch: Partial<Omit<Template, "id" | "createdAt">>,
): Action {
  return { type: TEMPLATE_UPDATE, id, patch };
}

/** Delete a template by ID. */
export function deleteTemplateAction(id: string): Action {
  return { type: TEMPLATE_DELETE, id };
}

/** Create a new instance by deep-cloning a template's item tree. */
export function createInstanceAction(templateId: string): Action {
  return { type: INSTANCE_CREATE, templateId };
}

/** Mark a check item as checked in an instance. */
export function checkItemAction(
  instanceId: string,
  templateItemId: string,
): Action {
  return { type: INSTANCE_CHECK, instanceId, templateItemId };
}

/** Mark a check item as unchecked in an instance. */
export function uncheckItemAction(
  instanceId: string,
  templateItemId: string,
): Action {
  return { type: INSTANCE_UNCHECK, instanceId, templateItemId };
}

/** Answer a decision item with "yes" or "no" (or null to clear). */
export function answerDecisionAction(
  instanceId: string,
  templateItemId: string,
  answer: DecisionAnswer,
): Action {
  return { type: INSTANCE_ANSWER, instanceId, templateItemId, answer };
}

/** Mark an instance as completed. */
export function completeInstanceAction(instanceId: string): Action {
  return { type: INSTANCE_COMPLETE, instanceId };
}

/** Mark an instance as abandoned. */
export function abandonInstanceAction(instanceId: string): Action {
  return { type: INSTANCE_ABANDON, instanceId };
}

/** Bulk-load state from persistence (IndexedDB or MemoryStorage). */
export function stateLoadAction(
  templates: Template[],
  instances: Instance[],
  todos: TodoItem[],
): Action {
  return { type: STATE_LOAD, templates, instances, todos };
}

export function createTodoAction(title: string, description: string, dueDate: string | null = null) {
  return { type: TODO_CREATE, title, description, dueDate };
}

export function updateTodoAction(todoId: string, patch: { title?: string; description?: string; status?: TodoStatus; dueDate?: string | null }) {
  return { type: TODO_UPDATE, todoId, patch };
}

export function deleteTodoAction(todoId: string) {
  return { type: TODO_DELETE, todoId };
}

export function toggleTodoAction(todoId: string) {
  return { type: TODO_TOGGLE, todoId };
}
