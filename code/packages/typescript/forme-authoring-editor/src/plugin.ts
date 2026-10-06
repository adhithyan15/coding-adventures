/**
 * Declarative editor extension boundary.
 *
 * A browser component cannot sandbox another browser component: JSX running in
 * the same realm can reach `document`, globals, and any ambient authority the
 * page exposes. FM09 therefore treats plugin contributions as hostile data.
 * The only executable extension point is an injected bridge implemented by the
 * already-sandboxed FM02 host.
 */

import type {
  AuthoringCommand,
  AuthoringProject,
} from "@coding-adventures/forme-authoring-core";
import { validateAuthoringProject } from "@coding-adventures/forme-authoring-core";

export const MAX_EDITOR_CONTRIBUTIONS = 32;
const MAX_DESCRIPTOR_SCALARS = 80;
const PORTABLE_ID = /^[a-z0-9]+(?:[._-][a-z0-9]+)*$/;
const BIDI_FORMATTING = /[\u202a-\u202e\u2066-\u2069]/u;

export type EditorSlot = "document-toolbar" | "block-toolbar" | "site-toolbar";

export interface EditorContribution {
  readonly pluginId: string;
  readonly actionId: string;
  readonly slot: EditorSlot;
  readonly label: string;
}

export interface EditorThemeOption {
  readonly id: string;
  readonly label: string;
}

export interface EditorPluginTarget {
  readonly documentId: string | null;
  readonly blockIndex: number | null;
}

export interface EditorPluginRequest {
  readonly pluginId: string;
  readonly actionId: string;
  readonly slot: EditorSlot;
  readonly target: EditorPluginTarget;
  readonly project: AuthoringProject;
}

export interface EditorPluginBridge {
  execute(request: EditorPluginRequest, signal?: AbortSignal): Promise<AuthoringCommand>;
}

export class EditorBoundaryError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "EditorBoundaryError";
  }
}

function scalarLength(value: string): number {
  let count = 0;
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (!(next >= 0xdc00 && next <= 0xdfff)) return -1;
      index += 1;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      return -1;
    }
    count += 1;
  }
  return count;
}

function boundedText(value: unknown, field: string, portable = false): string {
  if (typeof value !== "string") {
    throw new EditorBoundaryError(`${field} must be a string.`);
  }
  const length = scalarLength(value);
  if (length < 1 || length > MAX_DESCRIPTOR_SCALARS) {
    throw new EditorBoundaryError(`${field} must contain 1 through ${MAX_DESCRIPTOR_SCALARS} Unicode scalars.`);
  }
  if (value.trim() !== value) {
    throw new EditorBoundaryError(`${field} must not have surrounding whitespace.`);
  }
  if (BIDI_FORMATTING.test(value)) {
    throw new EditorBoundaryError(`${field} must not contain bidirectional formatting controls.`);
  }
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit < 0x20 || unit === 0x7f) {
      throw new EditorBoundaryError(`${field} must not contain control characters.`);
    }
  }
  if (portable && !PORTABLE_ID.test(value)) {
    throw new EditorBoundaryError(`${field} must be a portable lowercase identifier.`);
  }
  return value;
}

function denseItems(value: unknown, label: string, maximum: number): readonly unknown[] {
  if (!Array.isArray(value)) {
    throw new EditorBoundaryError(`${label} must be an array.`);
  }
  let lengthDescriptor: PropertyDescriptor | undefined;
  try {
    lengthDescriptor = Object.getOwnPropertyDescriptor(value, "length");
  } catch {
    throw new EditorBoundaryError(`${label} cannot be inspected safely.`);
  }
  if (lengthDescriptor === undefined || !("value" in lengthDescriptor)) {
    throw new EditorBoundaryError(`${label} has an invalid length.`);
  }
  const lengthValue = lengthDescriptor.value as unknown;
  if (typeof lengthValue !== "number" || !Number.isSafeInteger(lengthValue) || lengthValue > maximum) {
    throw new EditorBoundaryError(`${label} must contain at most ${maximum} entries.`);
  }
  const length = lengthValue;
  let keys: readonly PropertyKey[];
  try {
    keys = Reflect.ownKeys(value);
  } catch {
    throw new EditorBoundaryError(`${label} cannot be inspected safely.`);
  }
  if (keys.some((key) => typeof key !== "string")) {
    throw new EditorBoundaryError(`${label} must use string fields only.`);
  }
  if (keys.length !== length + 1) {
    throw new EditorBoundaryError(`${label} must be dense.`);
  }
  const result: unknown[] = [];
  for (let index = 0; index < length; index += 1) {
    let descriptor: PropertyDescriptor | undefined;
    try {
      descriptor = Object.getOwnPropertyDescriptor(value, String(index));
    } catch {
      throw new EditorBoundaryError(`${label} cannot be inspected safely.`);
    }
    if (descriptor === undefined) {
      throw new EditorBoundaryError(`${label} must be dense.`);
    }
    if (!("value" in descriptor) || !descriptor.enumerable) {
      throw new EditorBoundaryError(`${label} must contain data entries.`);
    }
    result.push(descriptor.value);
  }
  return result;
}

function exactDataObject(value: unknown, label: string, fields: readonly string[]): Record<string, unknown> {
  if (value === null || typeof value !== "object") {
    throw new EditorBoundaryError(`${label} must be a plain object.`);
  }
  let prototype: object | null;
  let keys: readonly PropertyKey[];
  try {
    prototype = Object.getPrototypeOf(value);
    keys = Reflect.ownKeys(value);
  } catch {
    throw new EditorBoundaryError(`${label} cannot be inspected safely.`);
  }
  if (prototype !== Object.prototype) {
    throw new EditorBoundaryError(`${label} must be a plain object.`);
  }
  if (keys.some((key) => typeof key !== "string")) {
    throw new EditorBoundaryError(`${label} must use string fields only.`);
  }
  const actual = [...keys as string[]].sort();
  const expected = [...fields].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    throw new EditorBoundaryError(`${label} has missing or unknown fields.`);
  }
  const result: Record<string, unknown> = Object.create(null) as Record<string, unknown>;
  for (const key of actual) {
    let descriptor: PropertyDescriptor | undefined;
    try {
      descriptor = Object.getOwnPropertyDescriptor(value, key);
    } catch {
      throw new EditorBoundaryError(`${label} cannot be inspected safely.`);
    }
    if (descriptor === undefined || !("value" in descriptor) || !descriptor.enumerable) {
      throw new EditorBoundaryError(`${label} must contain enumerable data fields.`);
    }
    Object.defineProperty(result, key, {
      value: descriptor.value,
      enumerable: true,
      writable: true,
      configurable: true,
    });
  }
  return result;
}

function editorSlot(value: unknown): EditorSlot {
  if (value !== "document-toolbar" && value !== "block-toolbar" && value !== "site-toolbar") {
    throw new EditorBoundaryError("Contribution slot is not supported.");
  }
  return value;
}

export function validateEditorContributions(value: unknown): readonly EditorContribution[] {
  const items = denseItems(value, "Editor contributions", MAX_EDITOR_CONTRIBUTIONS);
  const identities = new Set<string>();
  const result = items.map((item, index): EditorContribution => {
    const entry = exactDataObject(item, `Editor contribution ${index + 1}`, [
      "pluginId", "actionId", "slot", "label",
    ]);
    const contribution = Object.freeze({
      pluginId: boundedText(entry.pluginId, "Contribution pluginId", true),
      actionId: boundedText(entry.actionId, "Contribution actionId", true),
      slot: editorSlot(entry.slot),
      label: boundedText(entry.label, "Contribution label"),
    });
    const identity = `${contribution.pluginId}\u0000${contribution.actionId}\u0000${contribution.slot}`;
    if (identities.has(identity)) {
      throw new EditorBoundaryError("Editor contributions contain a duplicate identity.");
    }
    identities.add(identity);
    return contribution;
  });
  return Object.freeze(result);
}

export function validateThemeOptions(value: unknown): readonly EditorThemeOption[] {
  const items = denseItems(value, "Theme options", 64);
  if (items.length === 0) {
    throw new EditorBoundaryError("Theme options must contain at least one reviewed theme.");
  }
  const identities = new Set<string>();
  const result = items.map((item, index): EditorThemeOption => {
    const entry = exactDataObject(item, `Theme option ${index + 1}`, ["id", "label"]);
    const option = Object.freeze({
      id: boundedText(entry.id, "Theme id", true),
      label: boundedText(entry.label, "Theme label"),
    });
    if (identities.has(option.id)) {
      throw new EditorBoundaryError("Theme options contain a duplicate identifier.");
    }
    identities.add(option.id);
    return option;
  });
  return Object.freeze(result);
}

function validateTarget(target: EditorPluginTarget): EditorPluginTarget {
  const entry = exactDataObject(target, "Plugin target", ["documentId", "blockIndex"]);
  if (entry.documentId !== null && typeof entry.documentId !== "string") {
    throw new EditorBoundaryError("Plugin target documentId must be a string or null.");
  }
  if (entry.blockIndex !== null && (!Number.isSafeInteger(entry.blockIndex) || (entry.blockIndex as number) < 0)) {
    throw new EditorBoundaryError("Plugin target blockIndex must be a non-negative safe integer or null.");
  }
  return Object.freeze({
    documentId: entry.documentId as string | null,
    blockIndex: entry.blockIndex as number | null,
  });
}

export function createEditorPluginRequest(
  contribution: EditorContribution,
  project: AuthoringProject,
  target: EditorPluginTarget,
): EditorPluginRequest {
  const checked = validateEditorContributions([contribution])[0]!;
  let topLevelFrozen = false;
  try {
    topLevelFrozen = Object.isFrozen(project);
  } catch {
    throw new EditorBoundaryError("Plugin request project cannot be inspected safely.");
  }
  if (!topLevelFrozen) {
    throw new EditorBoundaryError("Plugin requests require the core's frozen project snapshot.");
  }
  let projectSnapshot: AuthoringProject;
  try {
    projectSnapshot = validateAuthoringProject(project);
  } catch {
    throw new EditorBoundaryError("Plugin request project is not a valid authoring snapshot.");
  }
  return Object.freeze({
    pluginId: checked.pluginId,
    actionId: checked.actionId,
    slot: checked.slot,
    target: validateTarget(target),
    project: projectSnapshot,
  });
}
