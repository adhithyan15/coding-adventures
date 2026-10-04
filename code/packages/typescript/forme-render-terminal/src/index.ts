/** Pure terminal renderer and terminal DeployArtifact packager (FM-B017). */

import { types as utilTypes } from "node:util";
import { parseFragment } from "parse5";
import type { Node } from "@coding-adventures/document-ast";
import { createOutputProvenance, computeRevisionId } from "@coding-adventures/forme-identity";
import {
  validateInteractivityDocument,
  type InteractivityDocument,
} from "@coding-adventures/forme-interactivity-ir";
import { defineStage } from "@coding-adventures/forme-stage";
import {
  emptyStyleDocument,
  validateStyleDocument,
  type Selector,
  type StyleDocument,
  type StyleRule,
} from "@coding-adventures/forme-style-ir";
import { compileTerminalStyles, type AnsiStyle } from "@coding-adventures/forme-style-to-terminal";
import {
  Kinds,
  streamOf,
  type ContentNode,
  type DeployArtifact,
  type StyleRuleId,
  type TerminalBuffer,
  type TerminalDegradation,
} from "@coding-adventures/forme-types";

export interface RouteInteractivity {
  readonly route: string;
  readonly document: unknown;
}

export interface RenderTerminalConfig {
  readonly style?: StyleDocument;
  readonly activeStyleContexts?: readonly string[];
  readonly interactivity?: readonly RouteInteractivity[];
}

export interface PackageTerminalConfig {
  readonly root?: string;
}

interface MatchNode {
  readonly type: string;
  readonly level?: number;
  readonly parent: MatchNode | null;
}

interface RenderState {
  readonly rules: readonly StyleRule[];
  readonly styles: ReadonlyMap<string, AnsiStyle>;
  readonly used: Set<string>;
  readonly degradations: TerminalDegradation[];
  readonly htmlIds: Map<string, number>;
  visitedNodes: number;
  rawHtmlBytes: number;
  htmlNodes: number;
}

const MAX_AST_DEPTH = 512;
const MAX_AST_NODES = 100_000;
const MAX_RAW_HTML_BYTES = 1_048_576;
const MAX_TERMINAL_TEXT_BYTES = 8_388_608;
const MAX_HTML_NODES = 100_000;
const PATH_SEGMENT_RE = /^[A-Za-z0-9._~!$&'()*+,;=@\-]+$/;
const WIN_RESERVED_RE = /^(con|prn|aux|nul|com[1-9]|lpt[1-9])(\..*)?$/i;

const renderTerminal = defineStage({
  name: "@coding-adventures/forme-render-terminal",
  version: "0.1.0",
  apiVersion: 1,
  description: "Render routed ContentNode values as ANSI TerminalBuffer values with explicit degradations.",
  consumes: streamOf(Kinds.ContentNode),
  produces: streamOf(Kinds.TerminalBuffer),
  capabilities: [],
  configSchema: {
    type: "object",
    properties: {
      style: { type: "object" },
      activeStyleContexts: { type: "array", items: { type: "string" } },
      interactivity: { type: "array", items: { type: "object" } },
    },
  },
  async *run(rawInput, rawConfig, ctx) {
    const config = snapshotRenderConfig(rawConfig);
    const validated = validateStyleDocument(config.style ?? emptyStyleDocument());
    if (validated.document.theme !== null) {
      throw new Error(`forme-render-terminal: StyleDocument theme ${JSON.stringify(validated.document.theme)} is unresolved`);
    }
    const interactivity = prepareInteractivity(config.interactivity);
    const input = rawInput as AsyncIterable<ContentNode>;
    for await (const node of input) {
      ctx.cancellation.throwIfCancelled();
      if (node.route === null) {
        throw new Error(`forme-render-terminal: ContentNode ${node.identity} (${node.sourcePath}) has no route`);
      }

      const usedRuleIds = collectUsedRules(node.document, validated.document.rules);
      const compiled = compileTerminalStyles(validated.document, {
        activeContexts: config.activeStyleContexts ?? ["screen", "dark"],
        usedRuleIds,
      });
      const state: RenderState = {
        rules: validated.document.rules,
        styles: compiled.styles,
        used: new Set(),
        degradations: [],
        htmlIds: new Map(),
        visitedNodes: 0,
        rawHtmlBytes: 0,
        htmlNodes: 0,
      };
      const main: MatchNode = { type: "main", parent: { type: "body", parent: { type: "html", parent: null } } };
      let text = renderNode(node.document, [], main, state).replace(/[ \t]+\n/g, "\n").replace(/\n{3,}/g, "\n\n").trimEnd() + "\n";
      text = wrapSynthetic(main.parent!.parent!, wrapSynthetic(main.parent!, wrapSynthetic(main, text, state), state), state);
      if (new TextEncoder().encode(text).byteLength > MAX_TERMINAL_TEXT_BYTES) {
        throw new Error("forme-render-terminal: rendered terminal text exceeds the 8 MiB limit");
      }

      for (const ref of node.assetRefs) {
        state.degradations.push(Object.freeze({
          code: "asset-reference-dropped",
          asset: ref.id,
          nodePath: Object.freeze([...ref.nodePath]),
          message: `terminal output preserves authored fallback text for ${ref.role} asset`,
        }));
      }
      for (const warning of compiled.warnings) {
        if (warning.ruleId === undefined || warning.propertyKind === undefined) continue;
        state.degradations.push(Object.freeze({
          code: "style-property-dropped",
          ruleId: warning.ruleId as StyleRuleId,
          propertyKind: warning.propertyKind,
          message: warning.message,
        }));
      }
      const document = interactivity.get(node.route);
      if (document !== undefined) appendInteractivityDegradations(node.route, document, state);

      yield Object.freeze({
        route: node.route,
        text,
        usedStyle: Object.freeze(compiled.emittedRules.filter(id => state.used.has(id))),
        usedAssets: Object.freeze([...new Set(node.assetRefs.map(ref => ref.id))]),
        degradations: Object.freeze(state.degradations),
        provenance: createOutputProvenance([node]),
      }) satisfies TerminalBuffer;
    }
  },
});

export const packageTerminal = defineStage({
  name: "@coding-adventures/forme-render-terminal/package",
  version: "0.1.0",
  apiVersion: 1,
  description: "Package TerminalBuffer values into a deterministic in-memory DeployArtifact.",
  consumes: streamOf(Kinds.TerminalBuffer),
  produces: Kinds.DeployArtifact,
  capabilities: [],
  configSchema: { type: "object", properties: { root: { type: "string" } } },
  async run(rawInput, rawConfig, ctx) {
    const root = snapshotPackageConfig(rawConfig).root ?? "terminal";
    const files: Record<string, Uint8Array> = {};
    const routes: Array<{ pattern: string; target: { kind: "file"; path: string }; islands: []; css: [] }> = [];
    const seen = new Set<string>();
    const seenPaths = new Map<string, string>();
    const revisions: Array<{ route: string; revision: string }> = [];
    const encoder = new TextEncoder();
    for await (const buffer of rawInput as AsyncIterable<TerminalBuffer>) {
      ctx.cancellation.throwIfCancelled();
      if (seen.has(buffer.route)) throw new Error(`forme-render-terminal/package: duplicate route ${JSON.stringify(buffer.route)}`);
      seen.add(buffer.route);
      const relative = portableRoute(buffer.route);
      const ansiPath = `${root}/${relative}.ansi`;
      const evidencePath = `${root}/${relative}.degradations.json`;
      for (const path of [ansiPath, evidencePath]) {
        const folded = path.toLowerCase();
        const previous = seenPaths.get(folded);
        if (previous !== undefined) {
          throw new Error(`forme-render-terminal/package: portable path collision between ${JSON.stringify(previous)} and ${JSON.stringify(buffer.route)}`);
        }
        seenPaths.set(folded, buffer.route);
      }
      files[ansiPath] = encoder.encode(buffer.text);
      files[evidencePath] = encoder.encode(`${JSON.stringify(buffer.degradations, null, 2)}\n`);
      routes.push({ pattern: buffer.route, target: { kind: "file", path: ansiPath }, islands: [], css: [] });
      revisions.push({ route: buffer.route, revision: buffer.provenance.revision });
    }
    routes.sort((a, b) => compare(a.pattern, b.pattern));
    const orderedFiles = Object.fromEntries(Object.entries(files).sort(([a], [b]) => compare(a, b)));
    const buildId = computeRevisionId({ terminal: revisions.sort((a, b) => compare(a.route, b.route)) });
    return Object.freeze({
      variant: Object.freeze({ kind: "dist-tree" as const }),
      files: Object.freeze(orderedFiles),
      manifest: Object.freeze({
        routes: Object.freeze(routes),
        assets: Object.freeze([]),
        buildTime: await ctx.time.nowIso(),
        buildId,
      }),
    }) satisfies DeployArtifact;
  },
});

function snapshotRenderConfig(value: unknown): RenderTerminalConfig {
  const object = exactObject(value, ["style", "activeStyleContexts", "interactivity"], "config");
  const contexts = object.get("activeStyleContexts");
  const entries = object.get("interactivity");
  return Object.freeze({
    ...(object.has("style") ? { style: object.get("style") as StyleDocument } : {}),
    ...(contexts === undefined ? {} : { activeStyleContexts: stringArray(contexts, "config.activeStyleContexts") }),
    ...(entries === undefined ? {} : { interactivity: entries as readonly RouteInteractivity[] }),
  });
}

function snapshotPackageConfig(value: unknown): PackageTerminalConfig {
  const object = exactObject(value, ["root"], "config");
  if (!object.has("root")) return Object.freeze({});
  const root = object.get("root");
  if (typeof root !== "string" || root.length === 0 || root.length > 2_048 || root.startsWith("/") || root.includes("\\")) {
    throw new TypeError("forme-render-terminal/package: config.root must be a portable relative path");
  }
  validatePortableSegments(root.split("/"), "config.root", 0);
  return Object.freeze({ root });
}

function prepareInteractivity(value: unknown): ReadonlyMap<string, InteractivityDocument> {
  if (value === undefined) return new Map();
  const values = arraySnapshot(value, 1_024, "config.interactivity");
  const result = new Map<string, InteractivityDocument>();
  for (let index = 0; index < values.length; index++) {
    const entry = exactObject(values[index], ["route", "document"], `config.interactivity[${index}]`);
    const route = entry.get("route");
    if (typeof route !== "string" || !route.startsWith("/") || route.includes("\\") || route.includes("..")) {
      throw new TypeError(`forme-render-terminal: config.interactivity[${index}].route is invalid`);
    }
    if (result.has(route)) throw new TypeError(`forme-render-terminal: duplicate interactivity route ${JSON.stringify(route)}`);
    result.set(route, validateInteractivityDocument(entry.get("document")));
  }
  return result;
}

function collectUsedRules(document: Node, rules: readonly StyleRule[]): readonly StyleRule["id"][] {
  const nodes: MatchNode[] = [
    { type: "html", parent: null },
    { type: "body", parent: { type: "html", parent: null } },
    { type: "main", parent: { type: "body", parent: { type: "html", parent: null } } },
  ];
  let visited = 0;
  const walk = (node: Node, parent: MatchNode, depth: number): void => {
    if (depth > MAX_AST_DEPTH || ++visited > MAX_AST_NODES) throw new Error("forme-render-terminal: document exceeds the structural limit");
    const match = matchNode(node, parent);
    if (match !== null) nodes.push(match);
    const next = match ?? parent;
    if ("children" in node) for (const child of node.children) walk(child as Node, next, depth + 1);
  };
  walk(document, nodes[2]!, 0);
  return rules.filter(rule => nodes.some(node => matches(rule.selector, node))).map(rule => rule.id);
}

function renderNode(node: Node, path: readonly number[], parent: MatchNode, state: RenderState, depth = 0): string {
  if (depth > MAX_AST_DEPTH || ++state.visitedNodes > MAX_AST_NODES) throw new Error("forme-render-terminal: document exceeds the structural limit");
  const match = matchNode(node, parent);
  const current = match ?? parent;
  const children = (separator = "") => "children" in node
    ? node.children.map((child, index) => renderNode(child as Node, [...path, index], current, state, depth + 1)).join(separator)
    : "";
  let output: string;
  switch (node.type) {
    case "document": output = children(); break;
    case "heading": output = `${children()}\n\n`; break;
    case "paragraph": output = `${children()}\n\n`; break;
    case "code_block": output = `${safeText(node.value)}\n\n`; break;
    case "blockquote": output = children().trimEnd().split("\n").map(line => `> ${line}`).join("\n") + "\n\n"; break;
    case "list": output = node.children.map((child, index) => `${node.ordered ? `${(node.start ?? 1) + index}.` : "-"} ${renderNode(child, [...path, index], current, state, depth + 1).trim()}\n`).join("") + "\n"; break;
    case "list_item": output = children(); break;
    case "task_item": output = `[${node.checked ? "x" : " "}] ${children()}`; break;
    case "thematic_break": output = "---\n\n"; break;
    case "raw_block": output = renderRaw(node.format, node.value, path, true, state); break;
    case "table": output = children(); break;
    case "table_row": output = `${children("\t")}\n`; break;
    case "table_cell": output = children(); break;
    case "text": output = safeText(node.value); break;
    case "emphasis": case "strong": case "strikethrough": output = children(); break;
    case "code_span": output = safeText(node.value); break;
    case "link": output = `${children()} <${safeText(node.destination)}>`; break;
    case "image": output = `[image: ${safeText(node.alt || node.destination)}]`; break;
    case "autolink": output = safeText(node.destination); break;
    case "raw_inline": output = renderRaw(node.format, node.value, path, false, state); break;
    case "hard_break": output = "\n"; break;
    case "soft_break": output = " "; break;
  }
  return match === null ? output : wrapSynthetic(match, output, state);
}

function renderRaw(format: string, value: string, path: readonly number[], block: boolean, state: RenderState): string {
  state.degradations.push(Object.freeze({
    code: "raw-node-dropped", format, nodePath: Object.freeze([...path]),
    message: `terminal backend extracted fallback text and dropped raw ${format} markup`,
  }));
  if (format !== "html") return "";
  state.rawHtmlBytes += new TextEncoder().encode(value).byteLength;
  if (state.rawHtmlBytes > MAX_RAW_HTML_BYTES) throw new Error("forme-render-terminal: raw HTML exceeds the 1 MiB per-document limit");
  const fragment = parseFragment(value) as unknown as HtmlNode;
  const text = htmlFallback(fragment, state, 0).trim();
  return text.length === 0 ? "" : `${text}${block ? "\n\n" : ""}`;
}

interface HtmlNode { readonly nodeName?: string; readonly value?: string; readonly attrs?: readonly { name: string; value: string }[]; readonly childNodes?: readonly HtmlNode[] }

function htmlFallback(node: HtmlNode, state: RenderState, depth: number): string {
  if (depth > MAX_AST_DEPTH || ++state.htmlNodes > MAX_HTML_NODES) throw new Error("forme-render-terminal: raw HTML exceeds the structural limit");
  for (const attribute of node.attrs ?? []) if (attribute.name === "id") state.htmlIds.set(attribute.value, (state.htmlIds.get(attribute.value) ?? 0) + 1);
  if (node.nodeName === "script" || node.nodeName === "style") return "";
  if (node.nodeName === "#text") return safeText(node.value ?? "");
  const content = (node.childNodes ?? []).map(child => htmlFallback(child, state, depth + 1)).join("");
  return ["p", "div", "li", "section", "article"].includes(node.nodeName ?? "") ? `${content}\n` : content;
}

function appendInteractivityDegradations(route: string, document: InteractivityDocument, state: RenderState): void {
  for (const island of document.islands) {
    if (island.fallback.kind === "element") {
      const count = state.htmlIds.get(island.fallback.id) ?? 0;
      if (count !== 1) throw new Error(`forme-render-terminal: route ${JSON.stringify(route)} fallback id ${JSON.stringify(island.fallback.id)} has ${count} matches`);
    }
    state.degradations.push(Object.freeze({
      code: "interactivity-dropped", islandId: island.id as never,
      message: "terminal backend preserves fallback content and drops executable island behavior",
    }));
  }
  if (document.state.length + document.bindings.length + document.handlers.length > 0 && document.islands.length === 0) {
    state.degradations.push(Object.freeze({
      code: "interactivity-dropped", islandId: null,
      message: "terminal backend drops declarative state, bindings, and handlers",
    }));
  }
}

function wrapSynthetic(node: MatchNode, value: string, state: RenderState): string {
  let output = value;
  for (const rule of state.rules) {
    if (!matches(rule.selector, node)) continue;
    const style = state.styles.get(rule.id);
    if (style === undefined || style.prefix.length === 0) continue;
    state.used.add(rule.id);
    output = `${style.prefix}${output}${style.suffix}`;
  }
  return output;
}

function matchNode(node: Node, parent: MatchNode): MatchNode | null {
  switch (node.type) {
    case "document": return null;
    case "heading": return { type: `h${node.level}`, level: node.level, parent };
    case "paragraph": return { type: "p", parent };
    case "code_block": return { type: "pre", parent };
    case "blockquote": return { type: "blockquote", parent };
    case "list": return { type: node.ordered ? "ol" : "ul", parent };
    case "list_item": case "task_item": return { type: "li", parent };
    case "thematic_break": return { type: "hr", parent };
    case "table": return { type: "table", parent };
    case "table_row": return { type: "tr", parent };
    case "table_cell": return { type: node.children.length > 0 ? "td" : "td", parent };
    case "emphasis": return { type: "em", parent };
    case "strong": return { type: "strong", parent };
    case "strikethrough": return { type: "del", parent };
    case "code_span": return { type: "code", parent };
    case "link": case "autolink": return { type: "a", parent };
    case "image": return { type: "img", parent };
    case "hard_break": return { type: "br", parent };
    case "raw_block": case "raw_inline": case "text": case "soft_break": return null;
  }
}

function matches(selector: Selector, node: MatchNode): boolean {
  switch (selector.kind) {
    case "node-type": return node.type === selector.type;
    case "node-type-level": return node.level === selector.level;
    case "and": return selector.all.every(item => matches(item, node));
    case "or": return selector.any.some(item => matches(item, node));
    case "not": return !matches(selector.inner, node);
    case "child-of": return matches(selector.child, node) && node.parent !== null && matches(selector.parent, node.parent);
    case "descendant-of": {
      if (!matches(selector.descendant, node)) return false;
      for (let parent = node.parent; parent !== null; parent = parent.parent) if (matches(selector.ancestor, parent)) return true;
      return false;
    }
    case "custom-kind": case "tag": case "id": case "role": case "nth": case "adjacent": return false;
  }
}

function exactObject(value: unknown, keys: readonly string[], path: string): ReadonlyMap<string, unknown> {
  if (value === undefined) return new Map();
  if (utilTypes.isProxy(value) || value === null || typeof value !== "object" || Array.isArray(value)) throw new TypeError(`forme-render-terminal: ${path} must be a plain object`);
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) throw new TypeError(`forme-render-terminal: ${path} must be a plain object`);
  if (Object.getOwnPropertySymbols(value).length !== 0) throw new TypeError(`forme-render-terminal: ${path} must not contain symbol keys`);
  const allowed = new Set(keys);
  const result = new Map<string, unknown>();
  for (const key of Object.getOwnPropertyNames(value)) {
    if (!allowed.has(key)) throw new TypeError(`forme-render-terminal: ${path}.${key} is unknown`);
    const descriptor = Object.getOwnPropertyDescriptor(value, key)!;
    if (!("value" in descriptor)) throw new TypeError(`forme-render-terminal: ${path}.${key} must not be an accessor`);
    result.set(key, descriptor.value);
  }
  return result;
}

function stringArray(value: unknown, path: string): readonly string[] {
  const values = arraySnapshot(value, 256, path);
  return Object.freeze(values.map((item, index) => {
    if (typeof item !== "string" || item.length > 128) throw new TypeError(`forme-render-terminal: ${path}[${index}] must be a bounded string`);
    return item;
  }));
}

function portableRoute(route: string): string {
  if (route.length > 2_048 || !route.startsWith("/") || route.startsWith("//") || route.includes("\\")) throw new Error(`forme-render-terminal/package: unsafe route ${JSON.stringify(route)}`);
  const parts = route.slice(1).split("/");
  validatePortableSegments(parts, "route", ".degradations.json".length);
  return parts.join("/");
}

function arraySnapshot(value: unknown, maxLength: number, path: string): readonly unknown[] {
  if (utilTypes.isProxy(value) || !Array.isArray(value) || value.length > maxLength) throw new TypeError(`forme-render-terminal: ${path} must be a bounded array`);
  if (Object.getOwnPropertySymbols(value).length !== 0) throw new TypeError(`forme-render-terminal: ${path} must not contain symbol keys`);
  const result: unknown[] = [];
  const names = Object.getOwnPropertyNames(value);
  for (const name of names) {
    if (name === "length") continue;
    if (!/^(0|[1-9][0-9]*)$/.test(name) || Number(name) >= value.length) throw new TypeError(`forme-render-terminal: ${path}.${name} is unknown`);
  }
  for (let index = 0; index < value.length; index++) {
    const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
    if (descriptor === undefined || !("value" in descriptor)) throw new TypeError(`forme-render-terminal: ${path}[${index}] must be a data element`);
    result.push(descriptor.value);
  }
  return Object.freeze(result);
}

function validatePortableSegments(parts: readonly string[], field: string, finalSuffixLength: number): void {
  for (let index = 0; index < parts.length; index++) {
    const part = parts[index]!;
    const suffix = index === parts.length - 1 ? finalSuffixLength : 0;
    if (
      part.length === 0 || part === "." || part === ".." || !PATH_SEGMENT_RE.test(part) ||
      part.endsWith(".") || part.endsWith(" ") || WIN_RESERVED_RE.test(part) ||
      part === "__proto__" || part === "constructor" || part === "prototype" ||
      new TextEncoder().encode(part).byteLength + suffix > 255
    ) {
      throw new TypeError(`forme-render-terminal/package: ${field} contains an unsafe portable path segment ${JSON.stringify(part)}`);
    }
  }
}

function safeText(value: string): string {
  return value.replace(/[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/g, "");
}

function compare(left: string, right: string): number { return left < right ? -1 : left > right ? 1 : 0; }

export default renderTerminal;
