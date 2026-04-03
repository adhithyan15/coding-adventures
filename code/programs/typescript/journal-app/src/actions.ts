/**
 * actions.ts — Action type constants and creator functions.
 *
 * Actions are plain objects that describe "what happened". They are the
 * only way to trigger a state change. The reducer receives each action
 * and computes the next state.
 *
 * === Pattern ===
 *
 * Each action has:
 *   1. A string constant (e.g. ENTRY_CREATE) — prevents typos when
 *      switching on action.type.
 *   2. A creator function (e.g. entryCreateAction) — a factory that
 *      builds the action object. Centralises the shape of each action.
 *
 * Components import creator functions, not the raw constants. The constants
 * are exported for use in the reducer and persistence middleware switch
 * statements.
 */

import type { Action } from "@coding-adventures/store";
import type { Entry } from "./types.js";

// ── Action type constants ──────────────────────────────────────────────────

export const ENTRY_CREATE = "ENTRY_CREATE";
export const ENTRY_UPDATE = "ENTRY_UPDATE";
export const ENTRY_DELETE = "ENTRY_DELETE";
export const ENTRIES_LOAD = "ENTRIES_LOAD";

// ── Action interfaces ──────────────────────────────────────────────────────

export interface EntryCreateAction extends Action {
  type: typeof ENTRY_CREATE;
  title: string;
  content: string;
  id: string;
  createdAt: string;
  updatedAt: number;
}

export interface EntryUpdateAction extends Action {
  type: typeof ENTRY_UPDATE;
  id: string;
  title: string;
  content: string;
  updatedAt: number;
}

export interface EntryDeleteAction extends Action {
  type: typeof ENTRY_DELETE;
  id: string;
}

export interface EntriesLoadAction extends Action {
  type: typeof ENTRIES_LOAD;
  entries: Entry[];
}

export type AppAction =
  | EntryCreateAction
  | EntryUpdateAction
  | EntryDeleteAction
  | EntriesLoadAction;

// ── Action creators ────────────────────────────────────────────────────────

/**
 * Generate a UUID. Uses crypto.randomUUID() in browsers and modern Node,
 * with a fallback for older environments.
 */
function generateId(): string {
  if (typeof crypto !== "undefined" && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

/**
 * Get today's date as an ISO 8601 string: "YYYY-MM-DD".
 */
function todayString(): string {
  return new Date().toISOString().slice(0, 10);
}

/**
 * entryCreateAction — create a new journal entry.
 *
 * The id, createdAt, and updatedAt are generated here so the reducer
 * stays a pure function with no side effects.
 */
export function entryCreateAction(
  title: string,
  content: string,
): EntryCreateAction {
  return {
    type: ENTRY_CREATE,
    title,
    content,
    id: generateId(),
    createdAt: todayString(),
    updatedAt: Date.now(),
  };
}

/**
 * entryUpdateAction — update an existing journal entry's title and content.
 */
export function entryUpdateAction(
  id: string,
  title: string,
  content: string,
): EntryUpdateAction {
  return {
    type: ENTRY_UPDATE,
    id,
    title,
    content,
    updatedAt: Date.now(),
  };
}

/**
 * entryDeleteAction — delete a journal entry by id.
 */
export function entryDeleteAction(id: string): EntryDeleteAction {
  return { type: ENTRY_DELETE, id };
}

/**
 * entriesLoadAction — bulk-load persisted entries from storage on startup.
 */
export function entriesLoadAction(entries: Entry[]): EntriesLoadAction {
  return { type: ENTRIES_LOAD, entries };
}
