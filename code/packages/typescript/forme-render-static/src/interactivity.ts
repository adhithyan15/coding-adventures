import { types as utilTypes } from "node:util";
import { isLogicalIdShape } from "@coding-adventures/forme-identity";
import { parseFragment } from "parse5";
import {
  canonicalInteractivityDocument,
  validateInteractivityDocument,
  type InteractivityDocument,
} from "@coding-adventures/forme-interactivity-ir";
import type {
  IslandId,
  IslandModuleUse,
  LogicalId,
} from "@coding-adventures/forme-types";

export interface RouteInteractivity {
  readonly route: string;
  readonly document: unknown;
}

export interface ReviewedIslandModule {
  readonly packageName: string;
  readonly export: string;
  readonly assetId: LogicalId;
  readonly sha256: string;
}

export interface PreparedInteractivity {
  readonly byRoute: ReadonlyMap<string, InteractivityDocument>;
  readonly modules: ReadonlyMap<string, Readonly<{ readonly asset: LogicalId; readonly sha256: string }>>;
}

export interface PageInteractivitySelection {
  readonly usedIslands: readonly IslandId[];
  readonly islandModules: readonly IslandModuleUse[];
  readonly moduleAssets: readonly LogicalId[];
}

const PACKAGE_NAME = /^(?:@[a-z0-9][a-z0-9._-]*\/[a-z0-9][a-z0-9._-]*|[a-z0-9][a-z0-9._-]*)$/;
const EXPORT_NAME = /^[A-Za-z_][A-Za-z0-9_]{0,63}$/;
const SHA256 = /^[0-9a-f]{64}$/;
const MAX_ROUTE_DOCUMENT_BYTES = 16 * 1024 * 1024;
const MAX_INTERACTIVE_BODY_BYTES = 8 * 1024 * 1024;

export function prepareInteractivity(
  routeEntries: unknown,
  moduleEntries: unknown,
): PreparedInteractivity {
  const routes = routeEntries === undefined ? [] : exactArray(routeEntries, "config.interactivity", 1_024);
  const modules = moduleEntries === undefined ? [] : exactArray(moduleEntries, "config.islandModules");
  const byRoute = new Map<string, InteractivityDocument>();
  let documentBytes = 0;
  for (let index = 0; index < routes.length; index++) {
    const path = `config.interactivity[${index}]`;
    const fields = exactObject(routes[index], ["route", "document"], path);
    const route = boundedString(fields.get("route"), `${path}.route`, 2_048);
    if (
      !route.startsWith("/") || route.startsWith("//") || route.includes("\\") ||
      /[\u0000-\u001f\u007f?#]/.test(route) ||
      route.split("/").some(segment => segment === "." || segment === "..")
    ) {
      throw new TypeError(`forme-render-static: ${path}.route must be a root-relative route`);
    }
    if (byRoute.has(route)) {
      throw new TypeError(`forme-render-static: duplicate interactivity route ${quoteBounded(route)}`);
    }
    const document = validateInteractivityDocument(fields.get("document"));
    documentBytes += Buffer.byteLength(canonicalInteractivityDocument(document), "utf8");
    if (documentBytes > MAX_ROUTE_DOCUMENT_BYTES) {
      throw new TypeError("forme-render-static: config.interactivity exceeds the 16 MiB aggregate document budget");
    }
    byRoute.set(route, document);
  }

  const moduleMap = new Map<string, Readonly<{ readonly asset: LogicalId; readonly sha256: string }>>();
  for (let index = 0; index < modules.length; index++) {
    const path = `config.islandModules[${index}]`;
    const fields = exactObject(modules[index], ["packageName", "export", "assetId", "sha256"], path);
    const packageName = boundedString(fields.get("packageName"), `${path}.packageName`, 214);
    const exportName = boundedString(fields.get("export"), `${path}.export`, 64);
    const assetId = boundedString(fields.get("assetId"), `${path}.assetId`, 36);
    const sha256 = boundedString(fields.get("sha256"), `${path}.sha256`, 64);
    if (!PACKAGE_NAME.test(packageName)) {
      throw new TypeError(`forme-render-static: ${path}.packageName is invalid`);
    }
    if (!EXPORT_NAME.test(exportName)) {
      throw new TypeError(`forme-render-static: ${path}.export is invalid`);
    }
    if (!isLogicalIdShape(assetId)) {
      throw new TypeError(`forme-render-static: ${path}.assetId must be a lowercase UUIDv7 LogicalId`);
    }
    if (!SHA256.test(sha256)) {
      throw new TypeError(`forme-render-static: ${path}.sha256 must be 64 lowercase hexadecimal characters`);
    }
    const key = moduleKey(packageName, exportName);
    if (moduleMap.has(key)) {
      throw new TypeError(
        `forme-render-static: duplicate island module mapping for ${JSON.stringify(packageName)} export ${JSON.stringify(exportName)}`,
      );
    }
    moduleMap.set(key, Object.freeze({ asset: assetId as LogicalId, sha256 }));
  }
  return Object.freeze({ byRoute, modules: moduleMap });
}

export function selectPageInteractivity(
  route: string,
  bodyHtml: string,
  prepared: PreparedInteractivity,
): PageInteractivitySelection {
  const document = prepared.byRoute.get(route);
  if (document === undefined) {
    return Object.freeze({ usedIslands: [], islandModules: [], moduleAssets: [] });
  }
  if (bodyHtml.length > MAX_INTERACTIVE_BODY_BYTES || Buffer.byteLength(bodyHtml, "utf8") > MAX_INTERACTIVE_BODY_BYTES) {
    throw new Error("forme-render-static: interactive rendered body exceeds the 8 MiB HTML5 parsing budget");
  }

  const referencedIds = referencedElementIds(document);
  const ids = collectElementIdCounts(bodyHtml, referencedIds);
  for (const id of referencedIds) {
    const count = ids.get(id) ?? 0;
    if (count === 0) {
      throw new Error(`forme-render-static: route ${quoteBounded(route)} element id ${JSON.stringify(id)} is missing`);
    }
    if (count !== 1) {
      throw new Error(`forme-render-static: route ${quoteBounded(route)} element id ${JSON.stringify(id)} is ambiguous (${count} matches)`);
    }
  }

  const usedIslands: IslandId[] = [];
  const islandModules: IslandModuleUse[] = [];
  const moduleAssets: LogicalId[] = [];
  const seenAssets = new Set<LogicalId>();
  for (const island of document.islands) {
    const module = prepared.modules.get(moduleKey(island.packageName, island.export));
    if (module === undefined) {
      throw new Error(
        `forme-render-static: island ${JSON.stringify(island.id)} on route ${quoteBounded(route)} has no reviewed module mapping for ${JSON.stringify(island.packageName)} export ${JSON.stringify(island.export)}`,
      );
    }
    const id = island.id as IslandId;
    usedIslands.push(id);
    islandModules.push(Object.freeze({
      island: id,
      asset: module.asset,
      packageName: island.packageName,
      export: island.export,
      sha256: module.sha256,
    }));
    if (!seenAssets.has(module.asset)) {
      seenAssets.add(module.asset);
      moduleAssets.push(module.asset);
    }
  }
  return Object.freeze({
    usedIslands: Object.freeze(usedIslands),
    islandModules: Object.freeze(islandModules),
    moduleAssets: Object.freeze(moduleAssets),
  });
}

function referencedElementIds(document: InteractivityDocument): Set<string> {
  const ids = new Set<string>();
  const add = (ref: { readonly kind: string; readonly id?: string } | undefined): void => {
    if (ref?.kind === "element") ids.add(ref.id!);
  };
  for (const state of document.state) add(state.owner);
  for (const binding of document.bindings) add(binding.target);
  for (const handler of document.handlers) add(handler.target);
  for (const island of document.islands) {
    add(island.target);
    add(island.fallback);
  }
  return ids;
}

function moduleKey(packageName: string, exportName: string): string {
  return `${packageName}\u0000${exportName}`;
}

function exactArray(value: unknown, path: string, maxLength = 4_096): readonly unknown[] {
  if (isProxy(value) || !Array.isArray(value)) {
    throw new TypeError(`forme-render-static: ${path} must be an array`);
  }
  const length = value.length;
  if (length > maxLength) throw new TypeError(`forme-render-static: ${path} exceeds ${maxLength} entries`);
  if (Object.getOwnPropertySymbols(value).length !== 0) {
    throw new TypeError(`forme-render-static: ${path} must not contain symbol keys`);
  }
  for (const key of Object.getOwnPropertyNames(value)) {
    if (key === "length") continue;
    const numeric = Number(key);
    if (!Number.isInteger(numeric) || numeric < 0 || numeric >= length || String(numeric) !== key) {
      throw new TypeError(`forme-render-static: ${path}${formatKey(key)} is unknown`);
    }
  }
  const copy: unknown[] = new Array(length);
  for (let index = 0; index < length; index++) {
    const descriptor = Object.getOwnPropertyDescriptor(value, String(index));
    if (descriptor === undefined) throw new TypeError(`forme-render-static: ${path}[${index}] is sparse`);
    if (!("value" in descriptor)) throw new TypeError(`forme-render-static: ${path}[${index}] must not be an accessor`);
    copy[index] = descriptor.value;
  }
  return copy;
}

function exactObject(value: unknown, keys: readonly string[], path: string): ReadonlyMap<string, unknown> {
  if (isProxy(value) || value === null || typeof value !== "object" || Array.isArray(value)) {
    throw new TypeError(`forme-render-static: ${path} must be a plain object`);
  }
  const prototype = Object.getPrototypeOf(value);
  if (prototype !== Object.prototype && prototype !== null) {
    throw new TypeError(`forme-render-static: ${path} must be a plain object`);
  }
  if (Object.getOwnPropertySymbols(value).length !== 0) {
    throw new TypeError(`forme-render-static: ${path} must not contain symbol keys`);
  }
  const allowed = new Set(keys);
  for (const key of Object.getOwnPropertyNames(value)) {
    if (!allowed.has(key)) throw new TypeError(`forme-render-static: ${path}${formatKey(key)} is unknown`);
  }
  const result = new Map<string, unknown>();
  for (const key of keys) {
    const descriptor = Object.getOwnPropertyDescriptor(value, key);
    if (descriptor === undefined) throw new TypeError(`forme-render-static: ${path}.${key} is required`);
    if (!("value" in descriptor)) throw new TypeError(`forme-render-static: ${path}.${key} must not be an accessor`);
    result.set(key, descriptor.value);
  }
  return result;
}

function boundedString(value: unknown, path: string, maxLength: number): string {
  if (typeof value !== "string" || value.length === 0 || value.length > maxLength) {
    throw new TypeError(`forme-render-static: ${path} must be a non-empty string of at most ${maxLength} characters`);
  }
  return value;
}

function isProxy(value: unknown): boolean {
  return (typeof value === "object" && value !== null) || typeof value === "function"
    ? utilTypes.isProxy(value)
    : false;
}

function formatKey(key: string): string {
  return /^[A-Za-z_][A-Za-z0-9_]{0,63}$/.test(key)
    ? `.${key}`
    : `[${quoteBounded(key)}]`;
}

function quoteBounded(value: string): string {
  let shown = "";
  let count = 0;
  let truncated = false;
  for (const point of value) {
    if (count === 64) {
      truncated = true;
      break;
    }
    const code = point.codePointAt(0)!;
    if (code >= 0x20 && code <= 0x7e) {
      shown += point === "\\" ? "\\\\" : point === '"' ? '\\"' : point;
    } else if (code <= 0xffff) {
      shown += `\\u${code.toString(16).padStart(4, "0")}`;
    } else {
      shown += `\\u{${code.toString(16)}}`;
    }
    count++;
  }
  return `"${shown}${truncated ? "..." : ""}"`;
}

/** Count active-document element IDs with HTML5 tree-construction semantics. */
export function collectElementIdCounts(
  html: string,
  wanted?: ReadonlySet<string>,
): ReadonlyMap<string, number> {
  const counts = new Map<string, number>();
  const fragment = parseFragment(html);
  type HtmlNode = { readonly nodeName: string; readonly attrs?: readonly { readonly name: string; readonly value: string }[]; readonly childNodes?: readonly unknown[] };
  const stack: HtmlNode[] = [fragment as typeof fragment & HtmlNode];
  let visited = 0;
  while (stack.length !== 0) {
    const node = stack.pop()!;
    if (++visited > 1_000_000) {
      throw new Error("forme-render-static: interactive rendered body exceeds the 1000000-node traversal budget");
    }
    if (node.nodeName === "#template") continue;
    const id = node.attrs?.find(attribute => attribute.name === "id")?.value;
    if (id !== undefined && (wanted === undefined || wanted.has(id))) {
      counts.set(id, (counts.get(id) ?? 0) + 1);
    }
    const children = node.childNodes ?? [];
    for (let index = children.length - 1; index >= 0; index--) stack.push(children[index] as HtmlNode);
  }
  return counts;
}
