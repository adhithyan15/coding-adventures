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

function plainObject(value: unknown, path: string, keys: readonly string[], state?: WalkState): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Object.getPrototypeOf(value) !== Object.prototype) {
    invalidProject(path, "expected a plain object");
  }
  if (state !== undefined) {
    if (state.seen.has(value)) invalidProject(path, "contains a cycle or shared object");
    state.seen.add(value);
  }
  const descriptors = Object.getOwnPropertyDescriptors(value);
  const actual = Object.keys(descriptors).sort();
  const expected = [...keys].sort();
  if (actual.length !== expected.length || actual.some((key, index) => key !== expected[index])) {
    invalidProject(path, "has missing or unknown fields");
  }
  for (const descriptor of Object.values(descriptors)) {
    if (!("value" in descriptor) || !descriptor.enumerable) invalidProject(path, "contains an accessor or hidden field");
  }
  return value as Record<string, unknown>;
}

function denseArray(value: unknown, path: string, state: WalkState): readonly unknown[] {
  if (!Array.isArray(value)) invalidProject(path, "expected an array");
  if (state.seen.has(value)) invalidProject(path, "contains a cycle or shared array");
  state.seen.add(value);
  for (let index = 0; index < value.length; index += 1) {
    if (!Object.hasOwn(value, index)) invalidProject(`${path}[${index}]`, "array is sparse");
  }
  return value;
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
  if (value === null || typeof value !== "object") invalidProject(path, "expected an inline node");
  const type = (value as { type?: unknown }).type;
  switch (type) {
    case "text": {
      const node = plainObject(value, path, ["type", "value"], state);
      return { type, value: stringValue(node.value, `${path}.value`, state.limits.maxStringScalars, { allowNewline: true }) };
    }
    case "emphasis":
    case "strong":
    case "strikethrough": {
      const node = plainObject(value, path, ["type", "children"], state);
      return { type, children: inlineNodes(node.children, `${path}.children`, depth + 1, state, inLink) };
    }
    case "code_span": {
      const node = plainObject(value, path, ["type", "value"], state);
      return { type, value: stringValue(node.value, `${path}.value`, state.limits.maxStringScalars, { allowNewline: true }) };
    }
    case "link": {
      if (inLink) invalidProject(path, "links must not be nested");
      const node = plainObject(value, path, ["type", "destination", "title", "children"], state);
      return {
        type,
        destination: safeDestination(node.destination, `${path}.destination`, state.limits),
        title: node.title === null ? null : stringValue(node.title, `${path}.title`, state.limits.maxTitleScalars),
        children: inlineNodes(node.children, `${path}.children`, depth + 1, state, true),
      };
    }
    case "image": {
      const node = plainObject(value, path, ["type", "destination", "title", "alt"], state);
      return {
        type,
        destination: safeDestination(node.destination, `${path}.destination`, state.limits),
        title: node.title === null ? null : stringValue(node.title, `${path}.title`, state.limits.maxTitleScalars),
        alt: stringValue(node.alt, `${path}.alt`, state.limits.maxStringScalars, { allowNewline: true }),
      };
    }
    case "autolink": {
      const node = plainObject(value, path, ["type", "destination", "isEmail"], state);
      if (typeof node.isEmail !== "boolean") invalidProject(`${path}.isEmail`, "expected a boolean");
      return {
        type,
        destination: safeDestination(node.destination, `${path}.destination`, state.limits, node.isEmail),
        isEmail: node.isEmail,
      };
    }
    case "hard_break":
    case "soft_break":
      plainObject(value, path, ["type"], state);
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
  if (value === null || typeof value !== "object") invalidProject(path, "expected a block node");
  const type = (value as { type?: unknown }).type;
  switch (type) {
    case "document":
      invalidProject(path, "a document node may appear only at the root");
    case "heading": {
      const node = plainObject(value, path, ["type", "level", "children"], state);
      if (![1, 2, 3, 4, 5, 6].includes(node.level as number)) invalidProject(`${path}.level`, "expected a heading level from 1 through 6");
      return { type, level: node.level as 1 | 2 | 3 | 4 | 5 | 6, children: inlineNodes(node.children, `${path}.children`, depth + 1, state) };
    }
    case "paragraph": {
      const node = plainObject(value, path, ["type", "children"], state);
      return { type, children: inlineNodes(node.children, `${path}.children`, depth + 1, state) };
    }
    case "code_block": {
      const node = plainObject(value, path, ["type", "language", "value"], state);
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
      const node = plainObject(value, path, ["type", "children"], state);
      return { type, children: blockNodes(node.children, `${path}.children`, depth + 1, state) };
    }
    case "task_item": {
      const node = plainObject(value, path, ["type", "checked", "children"], state);
      if (typeof node.checked !== "boolean") invalidProject(`${path}.checked`, "expected a boolean");
      return { type, checked: node.checked, children: blockNodes(node.children, `${path}.children`, depth + 1, state) };
    }
    case "list": {
      const node = plainObject(value, path, ["type", "ordered", "start", "tight", "children"], state);
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
      plainObject(value, path, ["type"], state);
      return { type };
    case "table": {
      const node = plainObject(value, path, ["type", "align", "children"], state);
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
  if (overrides === null || typeof overrides !== "object" || Object.getPrototypeOf(overrides) !== Object.prototype) {
    throw new AuthoringError("INVALID_LIMIT", "Authoring limits must be a plain object.");
  }
  for (const key of Object.keys(overrides)) {
    if (!LIMIT_KEYS.includes(key as keyof AuthoringLimits)) throw new AuthoringError("INVALID_LIMIT", "Authoring limits contain an unknown field.");
  }
  const result = { ...HARD_AUTHORING_LIMITS, ...overrides };
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
  const seen = new WeakSet<object>();
  const node = plainObject(value, "$", ["schemaVersion", "projectId", "title", "site", "documents", "activeDocumentId"], { nodes: 0, seen, limits });
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
  const byteLength = new TextEncoder().encode(canonicalJson(project)).byteLength;
  if (byteLength > limits.maxJsonBytes) invalidProject("$", `exceeds ${limits.maxJsonBytes} canonical JSON bytes`);
  return deepFreeze(project);
}

export function createAuthoringProject(input: CreateAuthoringProjectInput, overrides: AuthoringLimitOverrides = {}): AuthoringProject {
  return validateAuthoringProject({
    schemaVersion: 1,
    projectId: input.projectId,
    title: input.title,
    site: { baseUrl: null, themeId: input.themeId ?? "forme-classless" },
    documents: [],
    activeDocumentId: null,
  }, overrides);
}

export function canonicalAuthoringProject(project: AuthoringProject, overrides: AuthoringLimitOverrides = {}): string {
  return canonicalJson(validateAuthoringProject(project, overrides));
}

export { resolveLimits };
