/**
 * Hostile-input validation for the authoring project's Content IR subset.
 *
 * Validation constructs a fresh tree. Callers therefore never retain a path
 * to mutate a session through a value they supplied earlier. Exact-key checks
 * also prevent getters, prototypes, and quietly ignored fields from crossing
 * the persistence boundary.
 */

import type {
  BlockNode,
  DocumentNode,
  InlineNode,
  ListChildNode,
  TableAlignment,
} from "@coding-adventures/document-ast";

import { canonicalJson } from "./canonical.js";
import { AuthoringError, invalidProject } from "./error.js";
import {
  HARD_AUTHORING_LIMITS,
  type AuthoringDocument,
  type AuthoringDocumentStatus,
  type AuthoringLimitOverrides,
  type AuthoringLimits,
  type AuthoringProject,
  type CreateAuthoringProjectInput,
} from "./types.js";

const UUID_V7 = /^[0-9a-f]{8}-[0-9a-f]{4}-7[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;
const PORTABLE_NAME = /^[a-z0-9]+(?:[._-][a-z0-9]+)*$/;
const LIMIT_KEYS = Object.freeze(Object.keys(HARD_AUTHORING_LIMITS) as (keyof AuthoringLimits)[]);

interface WalkState {
  nodes: number;
  readonly seen: WeakSet<object>;
  readonly limits: AuthoringLimits;
}

interface ByteBudget {
  used: number;
  readonly maximum: number;
}

function spendBytes(budget: ByteBudget, count: number, path: string): void {
  budget.used += count;
  if (budget.used > budget.maximum) invalidProject(path, `exceeds ${budget.maximum} canonical JSON bytes`);
}

function jsonStringBytes(value: string): number {
  let bytes = 2;
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit === 0x22 || unit === 0x5c || unit === 0x08 || unit === 0x09 || unit === 0x0a || unit === 0x0c || unit === 0x0d) bytes += 2;
    else if (unit < 0x20) bytes += 6;
    else if (unit < 0x80) bytes += 1;
    else if (unit < 0x800) bytes += 2;
    else if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) invalidProject("$", "contains a lone surrogate");
      bytes += 4;
      index += 1;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) invalidProject("$", "contains a lone surrogate");
    else bytes += 3;
  }
  return bytes;
}

function safeOwnKeys(value: object, path: string): readonly PropertyKey[] {
  try { return Reflect.ownKeys(value); } catch { invalidProject(path, "cannot be inspected safely"); }
}

function ownDataSnapshot(value: object, path: string, keys = safeOwnKeys(value, path)): Record<PropertyKey, unknown> {
  let array = false;
  try { array = Array.isArray(value); } catch { invalidProject(path, "cannot be inspected safely"); }
  const result = Object.create(null) as Record<PropertyKey, unknown>;
  for (const key of keys) {
    let descriptor: PropertyDescriptor | undefined;
    try { descriptor = Object.getOwnPropertyDescriptor(value, key); } catch { invalidProject(path, "cannot be inspected safely"); }
    if (descriptor === undefined) invalidProject(path, "changed while being inspected");
    if (!('value' in descriptor)) invalidProject(path, "contains an accessor field");
    if (!descriptor.enumerable && !(array && key === "length")) invalidProject(path, "contains a hidden field");
    Object.defineProperty(result, key, { value: descriptor.value, enumerable: true, configurable: true, writable: true });
  }
  return result;
}

/** Exact, allocation-bounded canonical JSON measurement and descriptor snapshot. */
function preflightCanonicalSnapshot(value: unknown, limits: AuthoringLimits): unknown {
  const budget: ByteBudget = { used: 0, maximum: limits.maxJsonBytes };
  const seen = new WeakSet<object>();
  const walk = (item: unknown, path: string, depth: number): unknown => {
    if (depth > limits.maxDepth * 2 + 16) invalidProject(path, "exceeds the structural depth limit");
    if (item === null) { spendBytes(budget, 4, path); return null; }
    if (typeof item === "string") { spendBytes(budget, jsonStringBytes(item), path); return item; }
    if (typeof item === "boolean") { spendBytes(budget, item ? 4 : 5, path); return item; }
    if (typeof item === "number" && Number.isFinite(item)) { spendBytes(budget, JSON.stringify(item).length, path); return item; }
    if (typeof item !== "object") invalidProject(path, "contains a non-JSON value");
    if (seen.has(item)) invalidProject(path, "contains a cycle or shared object");
    seen.add(item);
    let isArray: boolean;
    try { isArray = Array.isArray(item); } catch { invalidProject(path, "cannot be inspected safely"); }
    if (isArray!) {
      let lengthDescriptor: PropertyDescriptor | undefined;
      try { lengthDescriptor = Object.getOwnPropertyDescriptor(item, "length"); } catch { invalidProject(path, "cannot be inspected safely"); }
      if (lengthDescriptor === undefined || !("value" in lengthDescriptor)) invalidProject(path, "has an invalid array length");
      const rawLength = lengthDescriptor.value;
      if (!Number.isSafeInteger(rawLength) || rawLength < 0) invalidProject(path, "has an invalid array length");
      if (rawLength > limits.maxNodesPerDocument) invalidProject(path, "exceeds the node or array-entry limit");
      const structuralBytes = 2 + Math.max(0, rawLength - 1);
      if (budget.used + structuralBytes + rawLength > budget.maximum) invalidProject(path, `exceeds ${budget.maximum} canonical JSON bytes`);
      const inputKeys = safeOwnKeys(item, path);
      if (inputKeys.length !== rawLength + 1) invalidProject(path, "has invalid array fields");
      const fields = ownDataSnapshot(item, path, inputKeys);
      const length = fields.length;
      if (!Number.isSafeInteger(length) || (length as number) < 0) invalidProject(path, "has an invalid array length");
      const keys = Reflect.ownKeys(fields);
      if (keys.some((key) => typeof key !== "string") || keys.length !== (length as number) + 1) invalidProject(path, "has invalid array fields");
      spendBytes(budget, structuralBytes, path);
      const result: unknown[] = [];
      for (let index = 0; index < (length as number); index += 1) {
        if (!Object.hasOwn(fields, String(index))) invalidProject(`${path}[${index}]`, "array is sparse");
        result.push(walk(fields[String(index)], `${path}[${index}]`, depth + 1));
      }
      return result;
    }
    let prototype: object | null;
    try { prototype = Object.getPrototypeOf(item); } catch { invalidProject(path, "cannot be inspected safely"); }
    if (prototype !== Object.prototype) invalidProject(path, "expected a plain object");
    const keys = safeOwnKeys(item, path);
    if (keys.length > 6) invalidProject(path, "has too many fields");
    const fields = ownDataSnapshot(item, path, keys);
    if (keys.some((key) => typeof key !== "string")) invalidProject(path, "contains a symbol field");
    const names = (keys as string[]).sort();
    spendBytes(budget, 2 + Math.max(0, names.length - 1), path);
    const result: Record<string, unknown> = {};
    for (const key of names) {
      spendBytes(budget, jsonStringBytes(key) + 1, path);
      Object.defineProperty(result, key, {
        value: walk(fields[key], `${path}.[field]`, depth + 1),
        enumerable: true,
        configurable: true,
        writable: true,
      });
    }
    return result;
  };
  return walk(value, "$", 0);
}

function scalarLength(value: string): number {
  let length = 0;
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) return -1;
      index += 1;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      return -1;
    }
    length += 1;
  }
  return length;
}

function stringValue(
  value: unknown,
  path: string,
  maximum: number,
  options: { readonly nonEmpty?: boolean; readonly trimmed?: boolean; readonly allowNewline?: boolean } = {},
): string {
  if (typeof value !== "string") invalidProject(path, "expected a string");
  const length = scalarLength(value);
  if (length < 0) invalidProject(path, "contains a lone surrogate");
  if (length > maximum) invalidProject(path, `exceeds ${maximum} Unicode scalars`);
  if (options.nonEmpty && length === 0) invalidProject(path, "must not be empty");
  if (options.trimmed && value.trim() !== value) invalidProject(path, "must not have surrounding whitespace");
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    const allowedWhitespace = options.allowNewline && (unit === 0x09 || unit === 0x0a || unit === 0x0d);
    if (!allowedWhitespace && (unit < 0x20 || unit === 0x7f)) {
      invalidProject(path, "contains a control character");
    }
  }
  return value;
}

function plainObjectSnapshot(value: unknown, path: string, state?: WalkState): Record<string, unknown> {
  if (value === null || typeof value !== "object") {
    invalidProject(path, "expected a plain object");
  }
  let prototype: object | null;
  try { prototype = Object.getPrototypeOf(value); } catch { invalidProject(path, "cannot be inspected safely"); }
  if (prototype !== Object.prototype) invalidProject(path, "expected a plain object");
  if (state !== undefined) {
    if (state.seen.has(value)) invalidProject(path, "contains a cycle or shared object");
    state.seen.add(value);
  }
  const keys = safeOwnKeys(value, path);
  if (keys.length > 6) invalidProject(path, "has too many fields");
  const snapshot = ownDataSnapshot(value, path, keys);
  const ownKeys = Reflect.ownKeys(snapshot);
  if (ownKeys.some((key) => typeof key !== "string")) invalidProject(path, "contains a symbol field");
  return snapshot as Record<string, unknown>;
}

function exactFields(snapshot: Record<string, unknown>, path: string, keys: readonly string[]): Record<string, unknown> {
  const ownKeys = Reflect.ownKeys(snapshot);
  const actual = (ownKeys as string[]).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    invalidProject(path, "has missing or unknown fields");
  }
  return snapshot as Record<string, unknown>;
}

function plainObject(value: unknown, path: string, keys: readonly string[], state?: WalkState): Record<string, unknown> {
  return exactFields(plainObjectSnapshot(value, path, state), path, keys);
}

function denseArray(value: unknown, path: string, state: WalkState): readonly unknown[] {
  let isArray: boolean;
  try { isArray = Array.isArray(value); } catch { invalidProject(path, "cannot be inspected safely"); }
  if (!isArray) invalidProject(path, "expected an array");
  const array = value as object;
  if (state.seen.has(array)) invalidProject(path, "contains a cycle or shared array");
  state.seen.add(array);
  const snapshot = ownDataSnapshot(array, path);
  const length = snapshot.length;
  if (!Number.isSafeInteger(length) || (length as number) < 0) invalidProject(path, "has an invalid array length");
  const ownKeys = Reflect.ownKeys(snapshot);
  if (ownKeys.some((key) => typeof key !== "string") || ownKeys.length !== (length as number) + 1) invalidProject(path, "has invalid array fields");
  const result: unknown[] = [];
  for (let index = 0; index < (length as number); index += 1) {
    if (!Object.hasOwn(snapshot, String(index))) invalidProject(`${path}[${index}]`, "array is sparse");
    result.push(snapshot[String(index)]);
  }
  return result;
}

function uuid(value: unknown, path: string, limits: AuthoringLimits): string {
  const result = stringValue(value, path, limits.maxSlugScalars, { nonEmpty: true, trimmed: true });
  if (!UUID_V7.test(result)) invalidProject(path, "expected canonical UUIDv7 text");
  return result;
}

function portableName(value: unknown, path: string, maximum: number): string {
  const result = stringValue(value, path, maximum, { nonEmpty: true, trimmed: true });
  if (!PORTABLE_NAME.test(result)) invalidProject(path, "expected a portable lowercase name");
  return result;
}

function safeDestination(value: unknown, path: string, limits: AuthoringLimits, email = false): string {
  const result = stringValue(value, path, limits.maxUrlScalars, { nonEmpty: true, trimmed: true });
  if (email) {
    if (!/^[^\s@]+@[^\s@]+$/.test(result)) invalidProject(path, "expected an email address");
    return result;
  }
  if (result.startsWith("//") || result.includes("\\")) invalidProject(path, "uses an unsafe URL form");
  const scheme = /^([a-zA-Z][a-zA-Z0-9+.-]*):/.exec(result)?.[1]?.toLowerCase();
  if (scheme !== undefined && scheme !== "https" && scheme !== "http" && scheme !== "mailto") {
    invalidProject(path, "uses an unsafe URL scheme");
  }
  return result;
}

function countNode(state: WalkState, path: string, depth: number): void {
  if (depth > state.limits.maxDepth) invalidProject(path, "exceeds the document depth limit");
  state.nodes += 1;
  if (state.nodes > state.limits.maxNodesPerDocument) invalidProject(path, "exceeds the document node limit");
}

function inlineNodes(value: unknown, path: string, depth: number, state: WalkState, inLink = false): readonly InlineNode[] {
  return denseArray(value, path, state).map((node, index) => inlineNode(node, `${path}[${index}]`, depth, state, inLink));
}

function inlineNode(value: unknown, path: string, depth: number, state: WalkState, inLink: boolean): InlineNode {
  countNode(state, path, depth);
  const probe = plainObjectSnapshot(value, path, state);
  const type = probe.type;
  switch (type) {
    case "text": {
      const node = exactFields(probe, path, ["type", "value"]);
      return { type, value: stringValue(node.value, `${path}.value`, state.limits.maxStringScalars, { allowNewline: true }) };
    }
    case "emphasis":
    case "strong":
    case "strikethrough": {
      const node = exactFields(probe, path, ["type", "children"]);
      return { type, children: inlineNodes(node.children, `${path}.children`, depth + 1, state, inLink) };
    }
    case "code_span": {
      const node = exactFields(probe, path, ["type", "value"]);
      return { type, value: stringValue(node.value, `${path}.value`, state.limits.maxStringScalars, { allowNewline: true }) };
    }
    case "link": {
      if (inLink) invalidProject(path, "links must not be nested");
      const node = exactFields(probe, path, ["type", "destination", "title", "children"]);
      return {
        type,
        destination: safeDestination(node.destination, `${path}.destination`, state.limits),
        title: node.title === null ? null : stringValue(node.title, `${path}.title`, state.limits.maxTitleScalars),
        children: inlineNodes(node.children, `${path}.children`, depth + 1, state, true),
      };
    }
    case "image": {
      const node = exactFields(probe, path, ["type", "destination", "title", "alt"]);
      return {
        type,
        destination: safeDestination(node.destination, `${path}.destination`, state.limits),
        title: node.title === null ? null : stringValue(node.title, `${path}.title`, state.limits.maxTitleScalars),
        alt: stringValue(node.alt, `${path}.alt`, state.limits.maxStringScalars, { allowNewline: true }),
      };
    }
    case "autolink": {
      const node = exactFields(probe, path, ["type", "destination", "isEmail"]);
      if (typeof node.isEmail !== "boolean") invalidProject(`${path}.isEmail`, "expected a boolean");
      return {
        type,
        destination: safeDestination(node.destination, `${path}.destination`, state.limits, node.isEmail),
        isEmail: node.isEmail,
      };
    }
    case "hard_break":
    case "soft_break":
      exactFields(probe, path, ["type"]);
      return { type };
    case "raw_inline":
      invalidProject(path, "raw inline nodes are not authorable");
    default:
      invalidProject(path, "has an unknown inline node type");
  }
}

function blockNodes(value: unknown, path: string, depth: number, state: WalkState): readonly BlockNode[] {
  return denseArray(value, path, state).map((node, index) => blockNode(node, `${path}[${index}]`, depth, state));
}

function listChildren(value: unknown, path: string, depth: number, state: WalkState): readonly ListChildNode[] {
  return denseArray(value, path, state).map((node, index) => {
    const result = blockNode(node, `${path}[${index}]`, depth, state);
    if (result.type !== "list_item" && result.type !== "task_item") {
      invalidProject(`${path}[${index}]`, "lists contain only list or task items");
    }
    return result;
  });
}

function blockNode(value: unknown, path: string, depth: number, state: WalkState): BlockNode {
  countNode(state, path, depth);
  const probe = plainObjectSnapshot(value, path, state);
  const type = probe.type;
  switch (type) {
    case "document":
      invalidProject(path, "a document node may appear only at the root");
    case "heading": {
      const node = exactFields(probe, path, ["type", "level", "children"]);
      if (![1, 2, 3, 4, 5, 6].includes(node.level as number)) invalidProject(`${path}.level`, "expected a heading level from 1 through 6");
      return { type, level: node.level as 1 | 2 | 3 | 4 | 5 | 6, children: inlineNodes(node.children, `${path}.children`, depth + 1, state) };
    }
    case "paragraph": {
      const node = exactFields(probe, path, ["type", "children"]);
      return { type, children: inlineNodes(node.children, `${path}.children`, depth + 1, state) };
    }
    case "code_block": {
      const node = exactFields(probe, path, ["type", "language", "value"]);
      const literal = stringValue(node.value, `${path}.value`, state.limits.maxStringScalars, { allowNewline: true });
      if (!literal.endsWith("\n")) invalidProject(`${path}.value`, "code blocks must end with a newline");
      return {
        type,
        language: node.language === null ? null : portableName(node.language, `${path}.language`, state.limits.maxSlugScalars),
        value: literal,
      };
    }
    case "blockquote":
    case "list_item": {
      const node = exactFields(probe, path, ["type", "children"]);
      return { type, children: blockNodes(node.children, `${path}.children`, depth + 1, state) };
    }
    case "task_item": {
      const node = exactFields(probe, path, ["type", "checked", "children"]);
      if (typeof node.checked !== "boolean") invalidProject(`${path}.checked`, "expected a boolean");
      return { type, checked: node.checked, children: blockNodes(node.children, `${path}.children`, depth + 1, state) };
    }
    case "list": {
      const node = exactFields(probe, path, ["type", "ordered", "start", "tight", "children"]);
      if (typeof node.ordered !== "boolean") invalidProject(`${path}.ordered`, "expected a boolean");
      if (typeof node.tight !== "boolean") invalidProject(`${path}.tight`, "expected a boolean");
      if (node.ordered) {
        if (!Number.isSafeInteger(node.start) || (node.start as number) < 1) invalidProject(`${path}.start`, "expected a positive safe integer");
      } else if (node.start !== null) invalidProject(`${path}.start`, "must be null for an unordered list");
      return {
        type,
        ordered: node.ordered,
        start: node.start as number | null,
        tight: node.tight,
        children: listChildren(node.children, `${path}.children`, depth + 1, state),
      };
    }
    case "thematic_break":
      exactFields(probe, path, ["type"]);
      return { type };
    case "table": {
      const node = exactFields(probe, path, ["type", "align", "children"]);
      const align = denseArray(node.align, `${path}.align`, state).map((item, index) => {
        if (item !== null && item !== "left" && item !== "right" && item !== "center") invalidProject(`${path}.align[${index}]`, "expected a table alignment");
        return item as TableAlignment;
      });
      const rows = denseArray(node.children, `${path}.children`, state).map((item, index) => {
        countNode(state, `${path}.children[${index}]`, depth + 1);
        const row = plainObject(item, `${path}.children[${index}]`, ["type", "isHeader", "children"], state);
        if (row.type !== "table_row") invalidProject(`${path}.children[${index}].type`, "expected a table row");
        if (typeof row.isHeader !== "boolean") invalidProject(`${path}.children[${index}].isHeader`, "expected a boolean");
        const cells = denseArray(row.children, `${path}.children[${index}].children`, state).map((cell, cellIndex) => {
          countNode(state, `${path}.children[${index}].children[${cellIndex}]`, depth + 2);
          const entry = plainObject(cell, `${path}.children[${index}].children[${cellIndex}]`, ["type", "children"], state);
          if (entry.type !== "table_cell") invalidProject(`${path}.children[${index}].children[${cellIndex}].type`, "expected a table cell");
          return { type: "table_cell" as const, children: inlineNodes(entry.children, `${path}.children[${index}].children[${cellIndex}].children`, depth + 3, state) };
        });
        if (cells.length !== align.length) invalidProject(`${path}.children[${index}].children`, "must match the table column count");
        return { type: "table_row" as const, isHeader: row.isHeader, children: cells };
      });
      return { type, align, children: rows };
    }
    case "raw_block":
      invalidProject(path, "raw block nodes are not authorable");
    default:
      invalidProject(path, "has an unknown block node type");
  }
}

function documentNode(value: unknown, path: string, state: WalkState): DocumentNode {
  countNode(state, path, 0);
  const node = plainObject(value, path, ["type", "children"], state);
  if (node.type !== "document") invalidProject(`${path}.type`, "expected a document root");
  return { type: "document", children: blockNodes(node.children, `${path}.children`, 1, state) };
}

function resolveLimits(overrides: AuthoringLimitOverrides = {}): AuthoringLimits {
  if (overrides === null || typeof overrides !== "object") {
    throw new AuthoringError("INVALID_LIMIT", "Authoring limits must be a plain object.");
  }
  let prototype: object | null;
  let keys: readonly PropertyKey[];
  try {
    prototype = Object.getPrototypeOf(overrides);
    keys = Reflect.ownKeys(overrides);
  } catch {
    throw new AuthoringError("INVALID_LIMIT", "Authoring limits cannot be inspected safely.");
  }
  if (keys!.length > LIMIT_KEYS.length || prototype! !== Object.prototype || keys!.some((key) => typeof key !== "string")) {
    throw new AuthoringError("INVALID_LIMIT", "Authoring limits must be a plain string-keyed object.");
  }
  const snapshot: Record<string, unknown> = {};
  for (const key of keys! as string[]) {
    let descriptor: PropertyDescriptor | undefined;
    try { descriptor = Object.getOwnPropertyDescriptor(overrides, key); } catch {
      throw new AuthoringError("INVALID_LIMIT", "Authoring limits cannot be inspected safely.");
    }
    if (descriptor === undefined || !("value" in descriptor) || !descriptor.enumerable) {
      throw new AuthoringError("INVALID_LIMIT", "Authoring limits must contain enumerable data fields.");
    }
    Object.defineProperty(snapshot, key, { value: descriptor.value, enumerable: true });
  }
  for (const key of Object.keys(snapshot)) {
    if (!LIMIT_KEYS.includes(key as keyof AuthoringLimits)) throw new AuthoringError("INVALID_LIMIT", "Authoring limits contain an unknown field.");
  }
  const result = { ...HARD_AUTHORING_LIMITS, ...snapshot } as AuthoringLimits;
  for (const key of LIMIT_KEYS) {
    const value = result[key];
    if (!Number.isSafeInteger(value) || value < 1 || value > HARD_AUTHORING_LIMITS[key]) {
      throw new AuthoringError("INVALID_LIMIT", `Authoring limit ${key} must be a positive safe integer at or below the hard maximum.`);
    }
  }
  return Object.freeze(result);
}

function deepFreeze<T>(value: T): T {
  if (value !== null && typeof value === "object" && !Object.isFrozen(value)) {
    for (const child of Object.values(value)) deepFreeze(child);
    Object.freeze(value);
  }
  return value;
}

function authoringDocument(value: unknown, path: string, limits: AuthoringLimits, seen: WeakSet<object>): AuthoringDocument {
  const node = plainObject(value, path, ["id", "slug", "title", "status", "body"], { nodes: 0, seen, limits });
  const status = node.status;
  if (status !== "draft" && status !== "published") invalidProject(`${path}.status`, "expected draft or published");
  const walk: WalkState = { nodes: 0, seen, limits };
  return {
    id: uuid(node.id, `${path}.id`, limits),
    slug: portableName(node.slug, `${path}.slug`, limits.maxSlugScalars),
    title: stringValue(node.title, `${path}.title`, limits.maxTitleScalars, { nonEmpty: true, trimmed: true }),
    status: status as AuthoringDocumentStatus,
    body: documentNode(node.body, `${path}.body`, walk),
  };
}

export function validateAuthoringProject(value: unknown, overrides: AuthoringLimitOverrides = {}): AuthoringProject {
  const limits = resolveLimits(overrides);
  const snapshot = preflightCanonicalSnapshot(value, limits);
  const seen = new WeakSet<object>();
  const node = plainObject(snapshot, "$", ["schemaVersion", "projectId", "title", "site", "documents", "activeDocumentId"], { nodes: 0, seen, limits });
  if (node.schemaVersion !== 1) invalidProject("$.schemaVersion", "expected version 1");
  const site = plainObject(node.site, "$.site", ["baseUrl", "themeId"], { nodes: 0, seen, limits });
  let baseUrl: string | null = null;
  if (site.baseUrl !== null) {
    const candidate = safeDestination(site.baseUrl, "$.site.baseUrl", limits);
    let parsed: URL;
    try { parsed = new URL(candidate); } catch { invalidProject("$.site.baseUrl", "expected an absolute HTTP(S) URL"); }
    if ((parsed!.protocol !== "https:" && parsed!.protocol !== "http:") || parsed!.username !== "" || parsed!.password !== "") {
      invalidProject("$.site.baseUrl", "expected an absolute HTTP(S) URL without credentials");
    }
    baseUrl = candidate;
  }
  const values = denseArray(node.documents, "$.documents", { nodes: 0, seen, limits });
  if (values.length > limits.maxDocuments) invalidProject("$.documents", `exceeds ${limits.maxDocuments} documents`);
  const documents = values.map((entry, index) => authoringDocument(entry, `$.documents[${index}]`, limits, seen));
  const ids = new Set<string>();
  const slugs = new Set<string>();
  for (const document of documents) {
    if (ids.has(document.id)) invalidProject("$.documents", "contains a duplicate document identity");
    if (slugs.has(document.slug)) invalidProject("$.documents", "contains a duplicate document slug");
    ids.add(document.id);
    slugs.add(document.slug);
  }
  const activeDocumentId = node.activeDocumentId === null ? null : uuid(node.activeDocumentId, "$.activeDocumentId", limits);
  if (activeDocumentId !== null && !ids.has(activeDocumentId)) invalidProject("$.activeDocumentId", "does not reference a document");
  const project: AuthoringProject = {
    schemaVersion: 1,
    projectId: uuid(node.projectId, "$.projectId", limits),
    title: stringValue(node.title, "$.title", limits.maxTitleScalars, { nonEmpty: true, trimmed: true }),
    site: {
      baseUrl,
      themeId: portableName(site.themeId, "$.site.themeId", limits.maxSlugScalars),
    },
    documents,
    activeDocumentId,
  };
  return deepFreeze(project);
}

export function createAuthoringProject(input: CreateAuthoringProjectInput, overrides: AuthoringLimitOverrides = {}): AuthoringProject {
  if (input === null || typeof input !== "object") invalidProject("$input", "expected a plain object");
  const snapshot = plainObjectSnapshot(input, "$input");
  const keys = Object.keys(snapshot);
  if (!keys.includes("projectId") || !keys.includes("title") || keys.some((key) => !["projectId", "title", "themeId"].includes(key))) {
    invalidProject("$input", "has missing or unknown fields");
  }
  return validateAuthoringProject({
    schemaVersion: 1,
    projectId: snapshot.projectId,
    title: snapshot.title,
    site: { baseUrl: null, themeId: snapshot.themeId ?? "forme-classless" },
    documents: [],
    activeDocumentId: null,
  }, overrides);
}

export function canonicalAuthoringProject(project: AuthoringProject, overrides: AuthoringLimitOverrides = {}): string {
  return canonicalJson(validateAuthoringProject(project, overrides));
}

export { resolveLimits };
