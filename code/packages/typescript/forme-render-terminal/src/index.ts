/** Pure terminal renderer and terminal DeployArtifact packager (FM-B017). */

import { types as utilTypes } from "node:util";
import { parseFragment } from "parse5";
import type { Node } from "@coding-adventures/document-ast";
import {
  createOutputProvenance,
  computeRevisionId,
  isLogicalIdShape,
  isRevisionIdShape,
} from "@coding-adventures/forme-identity";
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
  type JsonValue,
  type LogicalId,
  type RevisionId,
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

interface SnapshotBudget {
  nodes: number;
  stringCodeUnits: number;
  readonly seen: WeakSet<object>;
  readonly maxNodes: number;
  readonly maxStringCodeUnits: number;
}

const MAX_AST_DEPTH = 256;
const MAX_AST_NODES = 20_000;
const MAX_STYLE_RULES = 256;
const MAX_RAW_HTML_BYTES = 1_048_576;
const MAX_TERMINAL_TEXT_BYTES = 8_388_608;
const MAX_HTML_NODES = 20_000;
const MAX_PACKAGE_BUFFERS = 1_000;
const MAX_EVIDENCE_BYTES = 1_048_576;
const MAX_ARTIFACT_BYTES = 67_108_864;
const MAX_PROPERTY_KEY_CODE_UNITS = 256;
const MAX_DIAGNOSTIC_CODE_POINTS = 160;
const MAX_METADATA_CODE_POINTS = 256;
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
    if (validated.document.rules.length > MAX_STYLE_RULES) {
      throw new Error(`forme-render-terminal: StyleDocument exceeds the ${MAX_STYLE_RULES}-rule limit`);
    }
    for (const rule of validated.document.rules) {
      if (!isSafeMetadataString(rule.id)) throw new Error("forme-render-terminal: StyleRule.id contains terminal presentation controls");
    }
    if (validated.document.theme !== null) {
      throw new Error(`forme-render-terminal: StyleDocument theme ${asciiDiagnostic(validated.document.theme)} is unresolved`);
    }
    const interactivity = prepareInteractivity(config.interactivity);
    const backendConfigRevision = computeRevisionId({
      domain: "forme-terminal-backend-config-v1",
      style: (config.style ?? emptyStyleDocument()) as unknown as JsonValue,
      activeStyleContexts: [...(config.activeStyleContexts ?? ["screen", "dark"])],
      interactivity: (config.interactivity ?? []) as unknown as JsonValue,
    });
    const input = rawInput as AsyncIterable<unknown>;
    for await (const rawNode of input) {
      ctx.cancellation.throwIfCancelled();
      const node = snapshotContentNode(rawNode);
      if (node.route === null) {
        throw new Error(`forme-render-terminal: ContentNode ${node.identity} (${asciiDiagnostic(node.sourcePath)}) has no route`);
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
      let text = boundedConcat([renderNode(node.document, [], main, state).replace(/[ \t]+\n/g, "\n").replace(/\n{3,}/g, "\n\n").trimEnd(), "\n"]);
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
          ruleId: sanitizeMetadata(warning.ruleId) as StyleRuleId,
          propertyKind: sanitizeMetadata(warning.propertyKind),
          message: sanitizeMetadata(warning.message),
        }));
      }
      const document = interactivity.get(node.route);
      if (document !== undefined) appendInteractivityDegradations(node.route, document, state);

      const usedStyle = Object.freeze(compiled.emittedRules.filter(id => state.used.has(id)));
      const usedAssets = Object.freeze([...new Set(node.assetRefs.map(ref => ref.id))]);
      const degradations = Object.freeze(state.degradations);
      const sourceProvenance = createOutputProvenance([{ identity: node.identity, revision: node.revision }]);
      const outputRevision = computeRevisionId({
        domain: "forme-terminal-buffer-v1",
        sourceRevision: node.revision,
        backendConfigRevision,
        route: node.route,
        text,
        usedStyle: [...usedStyle],
        usedAssets: [...usedAssets],
        degradations: degradations as unknown as JsonValue,
      });
      const provenance = Object.freeze({
        contributors: sourceProvenance.contributors,
        revision: sourceProvenance.revision,
      });
      yield Object.freeze({
        route: node.route,
        text,
        usedStyle,
        usedAssets,
        degradations,
        revision: outputRevision,
        provenance,
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
    const filePaths = new Map<string, string>();
    const directoryPaths = new Set<string>();
    const identityFiles: Array<{ path: string; content: string }> = [];
    const encoder = new TextEncoder();
    let bufferCount = 0;
    let artifactBytes = 0;
    for await (const rawBuffer of rawInput as AsyncIterable<unknown>) {
      ctx.cancellation.throwIfCancelled();
      if (++bufferCount > MAX_PACKAGE_BUFFERS) {
        throw new Error(`forme-render-terminal/package: exceeds the ${MAX_PACKAGE_BUFFERS}-buffer limit`);
      }
      const buffer = snapshotTerminalBuffer(rawBuffer);
      if (seen.has(buffer.route)) throw new Error(`forme-render-terminal/package: duplicate route ${asciiDiagnostic(buffer.route)}`);
      seen.add(buffer.route);
      const relative = portableRoute(buffer.route);
      const ansiPath = `${root}/${relative}.ansi`;
      const evidencePath = `${root}/${relative}.degradations.json`;
      for (const path of [ansiPath, evidencePath]) {
        if (path.length > 2_048) throw new Error("forme-render-terminal/package: generated artifact path exceeds the 2048-character portable limit");
        registerPortableFile(path, buffer.route, filePaths, directoryPaths);
      }
      const evidence = `${JSON.stringify(buffer.degradations, null, 2)}\n`;
      const ansiBytes = encoder.encode(buffer.text);
      const evidenceBytes = encoder.encode(evidence);
      if (ansiBytes.byteLength > MAX_TERMINAL_TEXT_BYTES) {
        throw new Error("forme-render-terminal/package: ANSI file exceeds the 8 MiB limit");
      }
      if (evidenceBytes.byteLength > MAX_EVIDENCE_BYTES) {
        throw new Error("forme-render-terminal/package: degradation evidence exceeds the 1 MiB limit");
      }
      artifactBytes += ansiBytes.byteLength + evidenceBytes.byteLength;
      if (artifactBytes > MAX_ARTIFACT_BYTES) {
        throw new Error("forme-render-terminal/package: artifact exceeds the 64 MiB aggregate limit");
      }
      files[ansiPath] = ansiBytes;
      files[evidencePath] = evidenceBytes;
      identityFiles.push({ path: ansiPath, content: buffer.text }, { path: evidencePath, content: evidence });
      routes.push({ pattern: buffer.route, target: { kind: "file", path: ansiPath }, islands: [], css: [] });
    }
    routes.sort((a, b) => compare(a.pattern, b.pattern));
    const orderedFiles = Object.fromEntries(Object.entries(files).sort(([a], [b]) => compare(a, b)));
    identityFiles.sort((a, b) => compare(a.path, b.path));
    const buildId = computeRevisionId({ domain: "forme-terminal-artifact-v1", files: identityFiles });
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
    ...(object.has("style") ? { style: snapshotJson(object.get("style"), "config.style", snapshotBudget(100_000, 4_194_304)) as unknown as StyleDocument } : {}),
    ...(contexts === undefined ? {} : { activeStyleContexts: stringArray(contexts, "config.activeStyleContexts") }),
    ...(entries === undefined ? {} : { interactivity: snapshotJson(entries, "config.interactivity", snapshotBudget(100_000, 4_194_304)) as unknown as readonly RouteInteractivity[] }),
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
    if (result.has(route)) throw new TypeError(`forme-render-terminal: duplicate interactivity route ${asciiDiagnostic(route)}`);
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
    ? boundedJoin(node.children.map((child, index) => renderNode(child as Node, [...path, index], current, state, depth + 1)), separator)
    : "";
  let output: string;
  switch (node.type) {
    case "document": output = children(); break;
    case "heading": output = boundedConcat([children(), "\n\n"]); break;
    case "paragraph": output = boundedConcat([children(), "\n\n"]); break;
    case "code_block": output = boundedConcat([safeText(node.value), "\n\n"]); break;
    case "blockquote": output = boundedConcat([boundedJoin(children().trimEnd().split("\n").map(line => boundedConcat(["> ", line])), "\n"), "\n\n"]); break;
    case "list": output = boundedConcat([boundedJoin(node.children.map((child, index) => boundedConcat([node.ordered ? `${(node.start ?? 1) + index}.` : "-", " ", renderNode(child, [...path, index], current, state, depth + 1).trim(), "\n"]))), "\n"]); break;
    case "list_item": output = children(); break;
    case "task_item": output = boundedConcat([`[${node.checked ? "x" : " "}] `, children()]); break;
    case "thematic_break": output = "---\n\n"; break;
    case "raw_block": output = renderRaw(node.format, node.value, path, true, state); break;
    case "table": output = children(); break;
    case "table_row": output = boundedConcat([children("\t"), "\n"]); break;
    case "table_cell": output = children(); break;
    case "text": output = safeText(node.value); break;
    case "emphasis": case "strong": case "strikethrough": output = children(); break;
    case "code_span": output = safeText(node.value); break;
    case "link": output = boundedConcat([children(), " <", safeText(node.destination), ">"]); break;
    case "image": output = boundedConcat(["[image: ", safeText(node.alt || node.destination), "]"]); break;
    case "autolink": output = safeText(node.destination); break;
    case "raw_inline": output = renderRaw(node.format, node.value, path, false, state); break;
    case "hard_break": output = "\n"; break;
    case "soft_break": output = " "; break;
  }
  return match === null ? output : wrapSynthetic(match, output, state);
}

function renderRaw(format: string, value: string, path: readonly number[], block: boolean, state: RenderState): string {
  const safeFormat = sanitizeMetadata(format);
  state.degradations.push(Object.freeze({
    code: "raw-node-dropped", format: safeFormat, nodePath: Object.freeze([...path]),
    message: `terminal backend extracted fallback text and dropped raw ${safeFormat} markup`,
  }));
  if (format !== "html") return "";
  if (value.length > MAX_RAW_HTML_BYTES) throw new Error("forme-render-terminal: raw HTML exceeds the 1 MiB per-document limit");
  state.rawHtmlBytes += new TextEncoder().encode(value).byteLength;
  if (state.rawHtmlBytes > MAX_RAW_HTML_BYTES) throw new Error("forme-render-terminal: raw HTML exceeds the 1 MiB per-document limit");
  const fragment = parseFragment(value) as unknown as HtmlNode;
  const text = htmlFallback(fragment, state, 0).trim();
  return text.length === 0 ? "" : boundedConcat([text, block ? "\n\n" : ""]);
}

interface HtmlNode { readonly nodeName?: string; readonly value?: string; readonly attrs?: readonly { name: string; value: string }[]; readonly childNodes?: readonly HtmlNode[] }

function htmlFallback(node: HtmlNode, state: RenderState, depth: number): string {
  if (depth > MAX_AST_DEPTH || ++state.htmlNodes > MAX_HTML_NODES) throw new Error("forme-render-terminal: raw HTML exceeds the structural limit");
  if (node.nodeName === "script" || node.nodeName === "style") return "";
  for (const attribute of node.attrs ?? []) if (attribute.name === "id") state.htmlIds.set(attribute.value, (state.htmlIds.get(attribute.value) ?? 0) + 1);
  if (node.nodeName === "#text") return safeText(node.value ?? "");
  const content = boundedJoin((node.childNodes ?? []).map(child => htmlFallback(child, state, depth + 1)));
  return ["p", "div", "li", "section", "article"].includes(node.nodeName ?? "") ? boundedConcat([content, "\n"]) : content;
}

function appendInteractivityDegradations(route: string, document: InteractivityDocument, state: RenderState): void {
  for (const island of document.islands) {
    if (island.fallback.kind === "element") {
      const count = state.htmlIds.get(island.fallback.id) ?? 0;
      if (count !== 1) throw new Error(`forme-render-terminal: route ${asciiDiagnostic(route)} fallback id ${asciiDiagnostic(island.fallback.id)} has ${count} matches`);
    }
    state.degradations.push(Object.freeze({
      code: "interactivity-dropped", islandId: sanitizeMetadata(island.id) as never,
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
    output = boundedConcat([style.prefix, output, style.suffix]);
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

function snapshotBudget(maxNodes: number, maxStringCodeUnits: number): SnapshotBudget {
  return { nodes: 0, stringCodeUnits: 0, seen: new WeakSet(), maxNodes, maxStringCodeUnits };
}

function snapshotJson(value: unknown, path: string, budget: SnapshotBudget, depth = 0): JsonValue {
  if (depth > MAX_AST_DEPTH || ++budget.nodes > budget.maxNodes) {
    throw new TypeError(`forme-render-terminal: ${path} exceeds the structural limit`);
  }
  if (value === null || typeof value === "boolean") return value;
  if (typeof value === "number") {
    if (!Number.isFinite(value)) throw new TypeError(`forme-render-terminal: ${path} must contain finite JSON numbers`);
    return value;
  }
  if (typeof value === "string") {
    if (hasLoneSurrogate(value)) throw new TypeError(`forme-render-terminal: ${path} contains malformed Unicode`);
    budget.stringCodeUnits += value.length;
    if (budget.stringCodeUnits > budget.maxStringCodeUnits) throw new TypeError(`forme-render-terminal: ${path} exceeds the string limit`);
    return value;
  }
  if (typeof value !== "object" || utilTypes.isProxy(value)) {
    throw new TypeError(`forme-render-terminal: ${path} must be descriptor-safe JSON`);
  }
  if (budget.seen.has(value)) throw new TypeError(`forme-render-terminal: ${path} must not contain cycles`);
  budget.seen.add(value);
  if (Array.isArray(value)) {
    const raw = arraySnapshot(value, budget.maxNodes, path);
    const output = Object.freeze(raw.map((item, index) => snapshotJson(item, `${path}[${index}]`, budget, depth + 1)));
    budget.seen.delete(value);
    return output;
  }
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) throw new TypeError(`forme-render-terminal: ${path} must contain plain objects`);
  const output: Record<string, JsonValue> = Object.create(null) as Record<string, JsonValue>;
  let count = 0;
  for (const key in value) {
    if (key.length > MAX_PROPERTY_KEY_CODE_UNITS || hasLoneSurrogate(key) || !isSafeMetadataString(key)) {
      throw new TypeError(`forme-render-terminal: ${path} contains an unsafe field name`);
    }
    budget.stringCodeUnits += key.length;
    if (budget.stringCodeUnits > budget.maxStringCodeUnits) throw new TypeError(`forme-render-terminal: ${path} exceeds the string limit`);
    if (!Object.prototype.hasOwnProperty.call(value, key)) throw new TypeError(`forme-render-terminal: ${path} contains an inherited field`);
    if (++count > 64) throw new TypeError(`forme-render-terminal: ${path} has too many fields`);
    const descriptor = Object.getOwnPropertyDescriptor(value, key);
    if (descriptor === undefined || !("value" in descriptor)) throw new TypeError(`forme-render-terminal: ${path} contains an accessor field`);
    output[key] = snapshotJson(descriptor.value, `${path}.field`, budget, depth + 1);
  }
  budget.seen.delete(value);
  return Object.freeze(output);
}

function snapshotContentNode(value: unknown): ContentNode {
  const safe = snapshotJson(value, "ContentNode", snapshotBudget(40_000, MAX_TERMINAL_TEXT_BYTES)) as unknown;
  const node = recordValue(safe, "ContentNode");
  assertOnlyKeys(node, ["identity", "revision", "document", "frontmatter", "route", "assetRefs", "sourcePath"], "ContentNode");
  if (typeof node.identity !== "string" || !isLogicalIdShape(node.identity)) throw new TypeError("forme-render-terminal: ContentNode.identity is invalid");
  if (typeof node.revision !== "string" || !isRevisionIdShape(node.revision)) throw new TypeError("forme-render-terminal: ContentNode.revision is invalid");
  if (node.route !== null && typeof node.route !== "string") throw new TypeError("forme-render-terminal: ContentNode.route must be a string or null");
  if (typeof node.sourcePath !== "string") throw new TypeError("forme-render-terminal: ContentNode.sourcePath must be a string");
  recordValue(node.frontmatter, "ContentNode.frontmatter");
  const assets = arrayValue(node.assetRefs, "ContentNode.assetRefs");
  for (let index = 0; index < assets.length; index++) validateAssetRef(assets[index], `ContentNode.assetRefs[${index}]`);
  validateDocumentNode(node.document, "ContentNode.document", 0);
  return safe as ContentNode;
}

function validateAssetRef(value: unknown, path: string): void {
  const ref = recordValue(value, path);
  assertOnlyKeys(ref, ["id", "nodePath", "role", "sourcePath", "urlSuffix"], path);
  if (typeof ref.id !== "string" || !isLogicalIdShape(ref.id)) throw new TypeError(`forme-render-terminal: ${path}.id is invalid`);
  const nodePath = arrayValue(ref.nodePath, `${path}.nodePath`);
  if (nodePath.length > MAX_AST_DEPTH || nodePath.some(part => !Number.isSafeInteger(part) || (part as number) < 0)) throw new TypeError(`forme-render-terminal: ${path}.nodePath is invalid`);
  if (!["image", "video", "audio", "font", "script", "embed", "binary"].includes(ref.role as string)) throw new TypeError(`forme-render-terminal: ${path}.role is invalid`);
  for (const optional of ["sourcePath", "urlSuffix"] as const) if (ref[optional] !== undefined && typeof ref[optional] !== "string") throw new TypeError(`forme-render-terminal: ${path}.${optional} is invalid`);
}

function validateDocumentNode(value: unknown, path: string, depth: number): void {
  if (depth > MAX_AST_DEPTH) throw new TypeError(`forme-render-terminal: ${path} exceeds the structural limit`);
  const node = recordValue(value, path);
  if (typeof node.type !== "string") throw new TypeError(`forme-render-terminal: ${path}.type is invalid`);
  const children = (keys: readonly string[]) => {
    assertOnlyKeys(node, ["type", ...keys], path);
    const values = arrayValue(node.children, `${path}.children`);
    for (let index = 0; index < values.length; index++) validateDocumentNode(values[index], `${path}.children[${index}]`, depth + 1);
  };
  const string = (key: string) => { if (typeof node[key] !== "string") throw new TypeError(`forme-render-terminal: ${path}.${key} must be a string`); };
  const nullableString = (key: string) => { if (node[key] !== null && typeof node[key] !== "string") throw new TypeError(`forme-render-terminal: ${path}.${key} must be a string or null`); };
  switch (node.type) {
    case "document": case "paragraph": case "blockquote": case "list_item": case "emphasis": case "strong": case "strikethrough": case "table_cell":
      children(["children"]); break;
    case "heading":
      children(["level", "children"]);
      if (!Number.isInteger(node.level) || (node.level as number) < 1 || (node.level as number) > 6) throw new TypeError(`forme-render-terminal: ${path}.level is invalid`);
      break;
    case "code_block": assertOnlyKeys(node, ["type", "language", "value"], path); nullableString("language"); string("value"); break;
    case "list":
      children(["ordered", "start", "tight", "children"]);
      if (typeof node.ordered !== "boolean" || typeof node.tight !== "boolean" || (node.start !== null && !Number.isSafeInteger(node.start))) throw new TypeError(`forme-render-terminal: ${path} list fields are invalid`);
      break;
    case "task_item": children(["checked", "children"]); if (typeof node.checked !== "boolean") throw new TypeError(`forme-render-terminal: ${path}.checked is invalid`); break;
    case "thematic_break": case "hard_break": case "soft_break": assertOnlyKeys(node, ["type"], path); break;
    case "raw_block": case "raw_inline": assertOnlyKeys(node, ["type", "format", "value"], path); string("format"); string("value"); break;
    case "table":
      children(["align", "children"]);
      if (arrayValue(node.align, `${path}.align`).some(item => item !== null && item !== "left" && item !== "right" && item !== "center")) throw new TypeError(`forme-render-terminal: ${path}.align is invalid`);
      break;
    case "table_row": children(["isHeader", "children"]); if (typeof node.isHeader !== "boolean") throw new TypeError(`forme-render-terminal: ${path}.isHeader is invalid`); break;
    case "text": case "code_span": assertOnlyKeys(node, ["type", "value"], path); string("value"); break;
    case "link": children(["destination", "title", "children"]); string("destination"); nullableString("title"); break;
    case "image": assertOnlyKeys(node, ["type", "destination", "title", "alt"], path); string("destination"); nullableString("title"); string("alt"); break;
    case "autolink": assertOnlyKeys(node, ["type", "destination", "isEmail"], path); string("destination"); if (typeof node.isEmail !== "boolean") throw new TypeError(`forme-render-terminal: ${path}.isEmail is invalid`); break;
    default: throw new TypeError(`forme-render-terminal: ${path}.type ${asciiDiagnostic(node.type)} is unknown`);
  }
}

function snapshotTerminalBuffer(value: unknown): TerminalBuffer {
  const safe = snapshotJson(value, "TerminalBuffer", snapshotBudget(50_000, MAX_TERMINAL_TEXT_BYTES + MAX_EVIDENCE_BYTES)) as unknown;
  const buffer = recordValue(safe, "TerminalBuffer");
  assertOnlyKeys(buffer, ["route", "text", "usedStyle", "usedAssets", "degradations", "revision", "provenance"], "TerminalBuffer");
  if (typeof buffer.route !== "string" || typeof buffer.text !== "string") throw new TypeError("forme-render-terminal/package: TerminalBuffer route/text is invalid");
  validateTerminalText(buffer.text);
  const styles = arrayValue(buffer.usedStyle, "TerminalBuffer.usedStyle");
  if (styles.some(value => typeof value !== "string" || !isSafeMetadataString(value))) throw new TypeError("forme-render-terminal/package: TerminalBuffer.usedStyle is invalid");
  const assets = arrayValue(buffer.usedAssets, "TerminalBuffer.usedAssets");
  if (assets.some(value => typeof value !== "string" || !isLogicalIdShape(value))) throw new TypeError("forme-render-terminal/package: TerminalBuffer.usedAssets is invalid");
  const degradations = arrayValue(buffer.degradations, "TerminalBuffer.degradations");
  for (let index = 0; index < degradations.length; index++) validateDegradation(degradations[index], `TerminalBuffer.degradations[${index}]`);
  if (typeof buffer.revision !== "string" || !isRevisionIdShape(buffer.revision)) throw new TypeError("forme-render-terminal/package: TerminalBuffer.revision is invalid");
  validateOutputProvenance(buffer.provenance, "TerminalBuffer.provenance");
  return safe as TerminalBuffer;
}

function validateDegradation(value: unknown, path: string): void {
  const item = recordValue(value, path);
  if (typeof item.code !== "string" || typeof item.message !== "string" || !isSafeMetadataString(item.message)) throw new TypeError(`forme-render-terminal/package: ${path} is invalid`);
  switch (item.code) {
    case "style-property-dropped": assertOnlyKeys(item, ["code", "ruleId", "propertyKind", "message"], path); if (typeof item.ruleId !== "string" || !isSafeMetadataString(item.ruleId) || typeof item.propertyKind !== "string" || !isSafeMetadataString(item.propertyKind)) throw new TypeError(`forme-render-terminal/package: ${path} is invalid`); break;
    case "interactivity-dropped": assertOnlyKeys(item, ["code", "islandId", "message"], path); if (item.islandId !== null && (typeof item.islandId !== "string" || !isSafeMetadataString(item.islandId))) throw new TypeError(`forme-render-terminal/package: ${path} is invalid`); break;
    case "raw-node-dropped": assertOnlyKeys(item, ["code", "format", "nodePath", "message"], path); if (typeof item.format !== "string" || !isSafeMetadataString(item.format)) throw new TypeError(`forme-render-terminal/package: ${path} is invalid`); validateIndexPath(item.nodePath, `${path}.nodePath`); break;
    case "asset-reference-dropped": assertOnlyKeys(item, ["code", "asset", "nodePath", "message"], path); if (typeof item.asset !== "string" || !isLogicalIdShape(item.asset)) throw new TypeError(`forme-render-terminal/package: ${path} is invalid`); validateIndexPath(item.nodePath, `${path}.nodePath`); break;
    default: throw new TypeError(`forme-render-terminal/package: ${path}.code is unknown`);
  }
}

function validateIndexPath(value: unknown, path: string): void {
  const indexes = arrayValue(value, path);
  if (indexes.length > MAX_AST_DEPTH || indexes.some(index => !Number.isSafeInteger(index) || (index as number) < 0)) throw new TypeError(`forme-render-terminal/package: ${path} is invalid`);
}

function validateOutputProvenance(value: unknown, path: string): void {
  const provenance = recordValue(value, path);
  assertOnlyKeys(provenance, ["contributors", "revision"], path);
  if (typeof provenance.revision !== "string" || !isRevisionIdShape(provenance.revision)) throw new TypeError(`forme-render-terminal/package: ${path}.revision is invalid`);
  const contributors = arrayValue(provenance.contributors, `${path}.contributors`);
  const checked: Array<{ identity: LogicalId; revision: RevisionId }> = [];
  for (let index = 0; index < contributors.length; index++) {
    const contributor = recordValue(contributors[index], `${path}.contributors[${index}]`);
    assertOnlyKeys(contributor, ["identity", "revision"], `${path}.contributors[${index}]`);
    if (typeof contributor.identity !== "string" || !isLogicalIdShape(contributor.identity) || typeof contributor.revision !== "string" || !isRevisionIdShape(contributor.revision)) throw new TypeError(`forme-render-terminal/package: ${path}.contributors[${index}] is invalid`);
    checked.push({ identity: contributor.identity as LogicalId, revision: contributor.revision as RevisionId });
  }
  const canonical = createOutputProvenance(checked);
  if (
    provenance.revision !== canonical.revision ||
    contributors.length !== canonical.contributors.length ||
    canonical.contributors.some((contributor, index) => {
      const original = contributors[index] as Record<string, unknown>;
      return original.identity !== contributor.identity || original.revision !== contributor.revision;
    })
  ) throw new TypeError(`forme-render-terminal/package: ${path} is not canonical`);
}

function recordValue(value: unknown, path: string): Record<string, unknown> {
  if (value === null || typeof value !== "object" || Array.isArray(value)) throw new TypeError(`forme-render-terminal: ${path} must be an object`);
  return value as Record<string, unknown>;
}

function arrayValue(value: unknown, path: string): readonly unknown[] {
  if (!Array.isArray(value)) throw new TypeError(`forme-render-terminal: ${path} must be an array`);
  return value;
}

function assertOnlyKeys(value: Record<string, unknown>, allowed: readonly string[], path: string): void {
  const set = new Set(allowed);
  for (const key of Object.keys(value)) {
    if (key.length > MAX_PROPERTY_KEY_CODE_UNITS || hasLoneSurrogate(key) || !isSafeMetadataString(key)) {
      throw new TypeError(`forme-render-terminal: ${path} contains an unsafe field name`);
    }
    if (!set.has(key)) throw new TypeError(`forme-render-terminal: ${path} contains unknown field ${asciiDiagnostic(key)}`);
  }
  for (const key of allowed) if (!(key in value) && !["sourcePath", "urlSuffix"].includes(key)) throw new TypeError(`forme-render-terminal: ${path}.${key} is required`);
}

function exactObject(value: unknown, keys: readonly string[], path: string): ReadonlyMap<string, unknown> {
  if (value === undefined) return new Map();
  if (utilTypes.isProxy(value) || value === null || typeof value !== "object" || Array.isArray(value)) throw new TypeError(`forme-render-terminal: ${path} must be a plain object`);
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) throw new TypeError(`forme-render-terminal: ${path} must be a plain object`);
  const allowed = new Set(keys);
  const result = new Map<string, unknown>();
  let count = 0;
  for (const key in value) {
    if (key.length > MAX_PROPERTY_KEY_CODE_UNITS || hasLoneSurrogate(key) || !isSafeMetadataString(key)) {
      throw new TypeError(`forme-render-terminal: ${path} contains an unsafe field name`);
    }
    if (!Object.prototype.hasOwnProperty.call(value, key)) throw new TypeError(`forme-render-terminal: ${path} contains an inherited field`);
    if (++count > keys.length) throw new TypeError(`forme-render-terminal: ${path} has too many fields`);
    if (!allowed.has(key)) throw new TypeError(`forme-render-terminal: ${path} contains unknown field ${asciiDiagnostic(key)}`);
  }
  for (const key of keys) {
    const descriptor = Object.getOwnPropertyDescriptor(value, key)!;
    if (descriptor === undefined) continue;
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
  if (route.length > 2_048 || !route.startsWith("/") || route.startsWith("//") || route.includes("\\")) throw new Error(`forme-render-terminal/package: unsafe route ${asciiDiagnostic(route)}`);
  const parts = route.slice(1).split("/");
  validatePortableSegments(parts, "route", ".degradations.json".length);
  return parts.join("/");
}

function registerPortableFile(
  path: string,
  route: string,
  filePaths: Map<string, string>,
  directoryPaths: Set<string>,
): void {
  const folded = path.toLowerCase();
  const previous = filePaths.get(folded);
  if (previous !== undefined || directoryPaths.has(folded)) {
    throw new Error(`forme-render-terminal/package: portable path collision between ${asciiDiagnostic(previous ?? "an existing parent path")} and ${asciiDiagnostic(route)}`);
  }
  const segments = folded.split("/");
  let ancestor = "";
  for (let index = 0; index < segments.length - 1; index++) {
    ancestor = ancestor.length === 0 ? segments[index]! : `${ancestor}/${segments[index]!}`;
    const ancestorRoute = filePaths.get(ancestor);
    if (ancestorRoute !== undefined) {
      throw new Error(`forme-render-terminal/package: portable path collision between ${asciiDiagnostic(ancestorRoute)} and ${asciiDiagnostic(route)}`);
    }
    directoryPaths.add(ancestor);
  }
  filePaths.set(folded, route);
}

function arraySnapshot(value: unknown, maxLength: number, path: string): readonly unknown[] {
  if (utilTypes.isProxy(value) || !Array.isArray(value)) throw new TypeError(`forme-render-terminal: ${path} must be a bounded array`);
  const lengthDescriptor = Object.getOwnPropertyDescriptor(value, "length");
  const length = lengthDescriptor !== undefined && "value" in lengthDescriptor ? lengthDescriptor.value : -1;
  if (!Number.isSafeInteger(length) || length < 0 || length > maxLength) throw new TypeError(`forme-render-terminal: ${path} must be a bounded array`);
  const result: unknown[] = [];
  let count = 0;
  for (const name in value) {
    if (name.length > 32 || hasLoneSurrogate(name) || !isSafeMetadataString(name)) {
      throw new TypeError(`forme-render-terminal: ${path} contains an unsafe array property`);
    }
    if (!Object.prototype.hasOwnProperty.call(value, name)) throw new TypeError(`forme-render-terminal: ${path} contains an inherited array property`);
    if (++count > length) throw new TypeError(`forme-render-terminal: ${path} has too many elements`);
    if (!/^(0|[1-9][0-9]*)$/.test(name) || Number(name) >= length) throw new TypeError(`forme-render-terminal: ${path} contains an unknown array property`);
  }
  for (let index = 0; index < length; index++) {
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
      throw new TypeError(`forme-render-terminal/package: ${field} contains an unsafe portable path segment ${asciiDiagnostic(part)}`);
    }
  }
}

function safeText(value: string): string {
  return value
    .replace(/\r\n?/g, "\n")
    .replace(/[\u0000-\u0008\u000b-\u001f\u007f-\u009f]/g, "")
    .replace(/\p{Cf}/gu, "");
}

function sanitizeMetadata(value: string): string {
  let output = "";
  let index = 0;
  let points = 0;
  while (index < value.length && points < MAX_METADATA_CODE_POINTS) {
    const point = value.codePointAt(index)!;
    const character = String.fromCodePoint(point);
    if (!/[\p{Cc}\p{Cf}\p{Cs}]/u.test(character)) output += character;
    index += character.length;
    points++;
  }
  return output;
}

function asciiDiagnostic(value: string): string {
  let output = '"';
  let index = 0;
  let points = 0;
  while (index < value.length && points < MAX_DIAGNOSTIC_CODE_POINTS) {
    const point = value.codePointAt(index)!;
    const character = String.fromCodePoint(point);
    if (point >= 0x20 && point <= 0x7e) {
      output += character === "\\" || character === '"' ? `\\${character}` : character;
    } else {
      output += `\\u{${point.toString(16)}}`;
    }
    index += character.length;
    points++;
  }
  if (index < value.length) output += "...";
  return `${output}\"`;
}

function isSafeMetadataString(value: string): boolean {
  return !/[\p{Cc}\p{Cf}\p{Cs}]/u.test(value);
}

function validateTerminalText(value: string): void {
  const sgr = /\u001b\[[0-9]+(?:;[0-9]+)*m/y;
  for (let index = 0; index < value.length;) {
    if (value.charCodeAt(index) === 0x1b) {
      sgr.lastIndex = index;
      if (sgr.exec(value) === null) throw new TypeError("forme-render-terminal/package: TerminalBuffer.text contains a non-SGR terminal control sequence");
      index = sgr.lastIndex;
      continue;
    }
    const point = value.codePointAt(index)!;
    const character = String.fromCodePoint(point);
    if (character !== "\n" && character !== "\t" && /[\p{Cc}\p{Cf}\p{Cs}]/u.test(character)) {
      throw new TypeError("forme-render-terminal/package: TerminalBuffer.text contains authored terminal presentation controls");
    }
    index += character.length;
  }
}

function hasLoneSurrogate(value: string): boolean {
  for (let index = 0; index < value.length; index++) {
    const code = value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      if (index + 1 >= value.length) return true;
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) return true;
      index++;
    } else if (code >= 0xdc00 && code <= 0xdfff) return true;
  }
  return false;
}

function boundedConcat(parts: readonly string[]): string {
  let length = 0;
  for (const part of parts) {
    length += part.length;
    if (length > MAX_TERMINAL_TEXT_BYTES) throw new Error("forme-render-terminal: rendered terminal text exceeds the 8 MiB limit");
  }
  return parts.join("");
}

function boundedJoin(parts: readonly string[], separator = ""): string {
  if (parts.length === 0) return "";
  let length = separator.length * (parts.length - 1);
  for (const part of parts) {
    length += part.length;
    if (length > MAX_TERMINAL_TEXT_BYTES) throw new Error("forme-render-terminal: rendered terminal text exceeds the 8 MiB limit");
  }
  return parts.join(separator);
}

function compare(left: string, right: string): number { return left < right ? -1 : left > right ? 1 : 0; }

export default renderTerminal;
