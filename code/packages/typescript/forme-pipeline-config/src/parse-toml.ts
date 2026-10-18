/**
 * Declarative Forme pipeline configuration (FM03 §2.3).
 *
 * The repository TOML packages deliberately stop at a syntax tree: TOML's
 * table uniqueness and path rules belong to a semantic pass. This module is
 * that pass for pipeline configs. It first applies cheap resource ceilings,
 * then walks the grammar AST into null-prototype tables, and finally lowers
 * the closed declarative schema to the same `PipelineConfig` used by the
 * TypeScript loader.
 *
 * Keeping the three phases separate is useful: the grammar parser stays a
 * reusable teaching primitive, the TOML evaluator owns format semantics, and
 * the schema lowerer can report paths in the vocabulary a Forme user sees.
 */

import { parseTOML } from "@coding-adventures/toml-parser";
import { tokenizeTOML } from "@coding-adventures/toml-lexer";
import type {
  EdgeSpec,
  OutputSpec,
  PipelineConfig,
  PipelineSettings,
  PluginRuntimeConfig,
  StageInstanceSpec,
} from "./types.js";

export const MAX_TOML_CONFIG_BYTES = 1024 * 1024;
const MAX_TOKENS = 50_000;
const MAX_NESTING = 64;
const MAX_COLLECTION_ITEMS = 4096;
const MAX_ASSIGNMENTS = 4096;
const MAX_STRING_LENGTH = 64 * 1024;

const FORBIDDEN_SEGMENTS = new Set(["__proto__", "constructor", "prototype"]);
const LOG_LEVELS = new Set(["trace", "debug", "info", "warn", "error"]);

interface TomlToken {
  readonly type: string;
  readonly value: string;
  readonly line: number;
}

interface TomlNode {
  readonly ruleName: string;
  readonly children: readonly (TomlNode | TomlToken)[];
}

type TomlScalar = string | number | boolean;
type TomlValue = TomlScalar | TomlValue[] | TomlTable;
interface TomlTable {
  [key: string]: TomlValue;
}

function isNode(value: TomlNode | TomlToken): value is TomlNode {
  return typeof (value as TomlNode).ruleName === "string";
}

function table(): TomlTable {
  return Object.create(null) as TomlTable;
}

function fail(message: string): never {
  throw new TypeError(`parseTomlConfig: ${safeAscii(message)}`);
}

/** Parse bounded TOML text and lower it to the canonical pipeline shape. */
export function parseTomlConfig(source: string): PipelineConfig {
  if (typeof source !== "string") fail("source must be a string");
  if (source.length > MAX_TOML_CONFIG_BYTES) {
    fail(`source exceeds ${MAX_TOML_CONFIG_BYTES}-byte limit`);
  }
  const bytes = new TextEncoder().encode(source).byteLength;
  if (bytes > MAX_TOML_CONFIG_BYTES) {
    fail(`source exceeds ${MAX_TOML_CONFIG_BYTES}-byte limit (got ${bytes})`);
  }
  validateSourceScalars(source);

  let tokens: readonly TomlToken[];
  try {
    tokens = tokenizeTOML(source, MAX_TOKENS);
  } catch (error) {
    fail(`invalid TOML: ${message(error)}`);
  }
  preflight(tokens);

  let ast: TomlNode;
  try {
    ast = parseTOML(source, MAX_TOKENS) as TomlNode;
  } catch (error) {
    fail(`invalid TOML: ${message(error)}`);
  }
  return lowerPipeline(new TomlEvaluator().evaluate(ast));
}

function preflight(tokens: readonly TomlToken[]): void {
  if (tokens.length > MAX_TOKENS) fail(`token count exceeds ${MAX_TOKENS}-token limit`);
  let nesting = 0;
  for (const token of tokens) {
    if (token.value.length > MAX_STRING_LENGTH && token.type.includes("STRING")) {
      fail(`string exceeds ${MAX_STRING_LENGTH}-character limit on line ${token.line}`);
    }
    if (token.type === "LBRACKET" || token.type === "LBRACE") {
      nesting += 1;
      if (nesting > MAX_NESTING) fail(`nesting exceeds ${MAX_NESTING}-level limit`);
    } else if (token.type === "RBRACKET" || token.type === "RBRACE") {
      nesting -= 1;
    }
  }
}

function validateSourceScalars(source: string): void {
  for (let index = 0; index < source.length; index += 1) {
    const code = source.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = source.charCodeAt(index + 1);
      if (next >= 0xdc00 && next <= 0xdfff) {
        index += 1;
        continue;
      }
      fail("source contains an invalid Unicode scalar");
    }
    if (code >= 0xdc00 && code <= 0xdfff) {
      fail("source contains an invalid Unicode scalar");
    }
    if (code === 0x09 || code === 0x0a) continue;
    if (code === 0x0d && source.charCodeAt(index + 1) === 0x0a) continue;
    if (code <= 0x1f || code === 0x7f) {
      fail("source contains a forbidden raw control character");
    }
  }
}

class TomlEvaluator {
  private readonly root = table();
  private current = this.root;
  private currentPath: readonly string[] = [];
  private assignments = 0;
  private readonly declaredTables = new WeakSet<TomlTable>();
  private readonly dottedTables = new WeakSet<TomlTable>();
  private readonly arrayTables = new WeakSet<TomlValue[]>();
  private readonly inlineTables = new WeakSet<TomlTable>();

  evaluate(document: TomlNode): TomlTable {
    if (document.ruleName !== "document") fail(`expected document AST, got ${document.ruleName}`);
    for (const child of document.children) {
      if (!isNode(child)) continue;
      if (child.ruleName !== "expression") fail(`unexpected ${child.ruleName} in document`);
      this.expression(child);
    }
    return this.root;
  }

  private expression(expression: TomlNode): void {
    const inner = expression.children.find(isNode);
    if (inner === undefined) fail("empty TOML expression");
    if (inner.ruleName === "table_header") {
      const path = this.headerPath(inner);
      const target = this.ensureTable(this.root, path);
      if (this.inlineTables.has(target)) fail(`cannot extend inline table ${formatPath(path)}`);
      if (this.declaredTables.has(target)) fail(`table ${formatPath(path)} declared more than once`);
      this.declaredTables.add(target);
      this.current = target;
      this.currentPath = path;
      return;
    }
    if (inner.ruleName === "array_table_header") {
      const path = this.headerPath(inner);
      this.bumpAssignments("array-of-tables entries");
      const parent = this.ensureTable(this.root, path.slice(0, -1), false);
      const leaf = path.at(-1)!;
      const existing = parent[leaf];
      let values: TomlValue[];
      if (existing === undefined) {
        values = [];
        parent[leaf] = values;
        this.arrayTables.add(values);
      } else if (Array.isArray(existing) && this.arrayTables.has(existing)) {
        values = existing;
      } else {
        fail(`cannot redeclare ${formatPath(path)} as an array of tables`);
      }
      if (values.length >= MAX_COLLECTION_ITEMS) {
        fail(`collection exceeds ${MAX_COLLECTION_ITEMS}-item limit at ${formatPath(path)}`);
      }
      const next = table();
      values.push(next);
      this.current = next;
      this.currentPath = path;
      return;
    }
    if (inner.ruleName !== "keyval") fail(`unexpected expression node ${inner.ruleName}`);
    this.assignKeyval(this.current, inner, this.currentPath, 0);
  }

  private headerPath(header: TomlNode): readonly string[] {
    const key = header.children.find((child): child is TomlNode => isNode(child) && child.ruleName === "key");
    if (key === undefined) fail(`malformed ${header.ruleName}`);
    return this.keyPath(key);
  }

  private assignKeyval(target: TomlTable, keyval: TomlNode, base: readonly string[], depth: number): void {
    const nodes = keyval.children.filter(isNode);
    const key = nodes.find(node => node.ruleName === "key");
    const value = nodes.find(node => node.ruleName === "value");
    if (key === undefined || value === undefined) fail("malformed key/value expression");
    const path = this.keyPath(key);
    this.bumpAssignments("assignments");
    this.set(target, path, this.value(value, [...base, ...path], depth + 1), base);
  }

  private set(
    target: TomlTable,
    path: readonly string[],
    value: TomlValue,
    base: readonly string[],
  ): void {
    let current = target;
    for (let index = 0; index < path.length - 1; index += 1) {
      if (this.inlineTables.has(current)) fail(`cannot extend inline table ${formatPath([...base, ...path.slice(0, index)])}`);
      const segment = path[index]!;
      const existing = current[segment];
      if (existing === undefined) {
        const next = table();
        current[segment] = next;
        this.dottedTables.add(next);
        current = next;
      } else if (isTable(existing)) {
        current = existing;
      } else if (Array.isArray(existing) && this.arrayTables.has(existing)
          && isTable(existing[existing.length - 1])) {
        current = existing[existing.length - 1]! as TomlTable;
      } else {
        fail(`cannot descend into ${formatPath([...base, ...path.slice(0, index + 1)])}; it already has a value`);
      }
    }
    const leaf = path.at(-1)!;
    if (Object.hasOwn(current, leaf)) fail(`key ${formatPath([...base, ...path])} already assigned`);
    current[leaf] = value;
  }

  private ensureTable(root: TomlTable, path: readonly string[], declaringLeaf = true): TomlTable {
    let current = root;
    for (let index = 0; index < path.length; index += 1) {
      if (this.inlineTables.has(current)) fail(`cannot extend inline table ${formatPath(path.slice(0, index))}`);
      const segment = path[index]!;
      const existing = current[segment];
      if (existing === undefined) {
        const next = table();
        current[segment] = next;
        current = next;
      } else if (isTable(existing)) {
        if (declaringLeaf && index === path.length - 1 && this.dottedTables.has(existing)) {
          fail(`cannot redeclare dotted-key table ${formatPath(path)}`);
        }
        current = existing;
      } else if (Array.isArray(existing) && this.arrayTables.has(existing)
          && isTable(existing[existing.length - 1])) {
        if (declaringLeaf && index === path.length - 1) {
          fail(`cannot redeclare array of tables ${formatPath(path)} as a table`);
        }
        current = existing[existing.length - 1]! as TomlTable;
      } else {
        fail(`cannot redeclare ${formatPath(path.slice(0, index + 1))} as a table; it already has a value`);
      }
    }
    return current;
  }

  private keyPath(key: TomlNode): readonly string[] {
    const result: string[] = [];
    for (const child of key.children) {
      if (!isNode(child) || child.ruleName !== "simple_key") continue;
      const token = child.children.find((part): part is TomlToken => !isNode(part));
      if (token === undefined) fail("malformed TOML key");
      validateRawStringToken(token, [...result, "<key>"]);
      const segment = token.type === "BASIC_STRING"
        ? decodeBasicEscapes(token.value, token.line)
        : token.value;
      if (segment.length === 0) fail("key segments must not be empty");
      if (segment.length > 256) fail("key segment exceeds 256-character limit");
      if (FORBIDDEN_SEGMENTS.has(segment)) fail(`key segment ${JSON.stringify(segment)} is reserved`);
      result.push(segment);
    }
    if (result.length === 0) fail("key path must not be empty");
    if (result.length > MAX_NESTING) fail(`key path exceeds ${MAX_NESTING}-segment limit`);
    return result;
  }

  private value(node: TomlNode, path: readonly string[], depth: number): TomlValue {
    if (depth > MAX_NESTING) fail(`nesting exceeds ${MAX_NESTING}-level limit at ${formatPath(path)}`);
    const first = node.children[0];
    if (first === undefined) fail(`missing value at ${formatPath(path)}`);
    if (!isNode(first)) return scalar(first, path);
    if (first.ruleName === "array") return this.array(first, path, depth + 1);
    if (first.ruleName === "inline_table") return this.inlineTable(first, path, depth + 1);
    return fail(`unsupported TOML value ${first.ruleName} at ${formatPath(path)}`);
  }

  private array(node: TomlNode, path: readonly string[], depth: number): TomlValue[] {
    const valuesNode = node.children.find((child): child is TomlNode => isNode(child) && child.ruleName === "array_values");
    if (valuesNode === undefined) return [];
    const values: TomlValue[] = [];
    for (const child of valuesNode.children) {
      if (!isNode(child)) continue;
      if (child.ruleName !== "value") fail(`unexpected ${child.ruleName} in array at ${formatPath(path)}`);
      if (values.length >= MAX_COLLECTION_ITEMS) {
        fail(`collection exceeds ${MAX_COLLECTION_ITEMS}-item limit at ${formatPath(path)}`);
      }
      values.push(this.value(child, [...path, String(values.length)], depth));
    }
    return values;
  }

  private inlineTable(node: TomlNode, path: readonly string[], depth: number): TomlTable {
    const result = table();
    for (const child of node.children) {
      if (!isNode(child)) continue;
      if (child.ruleName !== "keyval") fail(`unexpected ${child.ruleName} in inline table at ${formatPath(path)}`);
      this.assignKeyval(result, child, path, depth);
    }
    this.inlineTables.add(result);
    return result;
  }

  private bumpAssignments(subject: string): void {
    this.assignments += 1;
    if (this.assignments > MAX_ASSIGNMENTS) fail(`${subject} exceed ${MAX_ASSIGNMENTS}-entry limit`);
  }
}

function scalar(token: TomlToken, path: readonly string[]): TomlScalar {
  if (token.value.length > MAX_STRING_LENGTH && token.type.includes("STRING")) {
    fail(`string exceeds ${MAX_STRING_LENGTH}-character limit at ${formatPath(path)}`);
  }
  validateRawStringToken(token, path);
  switch (token.type) {
    case "BASIC_STRING": return decodeBasicEscapes(token.value, token.line, false);
    case "ML_BASIC_STRING": return decodeBasicEscapes(token.value, token.line, true);
    case "LITERAL_STRING":
      return token.value;
    case "ML_LITERAL_STRING": return trimInitialMultilineNewline(token.value);
    case "TRUE": return true;
    case "FALSE": return false;
    case "INTEGER": return integer(token.value, path);
    case "FLOAT": {
      const value = Number(token.value.replaceAll("_", "").replace(/^\+/, ""));
      if (!Number.isFinite(value)) fail(`value at ${formatPath(path)} must be finite`);
      return value;
    }
    case "OFFSET_DATETIME":
    case "LOCAL_DATETIME":
    case "LOCAL_DATE":
    case "LOCAL_TIME":
      validateDateTime(token.value, token.type, path);
      return token.value;
    default: return fail(`unsupported scalar ${token.type} at ${formatPath(path)}`);
  }
}

function integer(raw: string, path: readonly string[]): number {
  const normalized = raw.replaceAll("_", "");
  let sign = 1;
  let body = normalized;
  if (body.startsWith("+")) body = body.slice(1);
  else if (body.startsWith("-")) { sign = -1; body = body.slice(1); }
  const radix = /^0x/i.test(body) ? 16 : /^0o/i.test(body) ? 8 : /^0b/i.test(body) ? 2 : 10;
  if (radix !== 10) body = body.slice(2);
  const value = sign * Number.parseInt(body, radix);
  if (!Number.isSafeInteger(value)) fail(`integer at ${formatPath(path)} exceeds the safe integer range`);
  return value;
}

function decodeBasicEscapes(raw: string, line: number, multiline = false): string {
  if (multiline) raw = trimInitialMultilineNewline(raw);
  let result = "";
  for (let index = 0; index < raw.length; index += 1) {
    const char = raw[index]!;
    if (char !== "\\") { result += char; continue; }
    if (multiline) {
      let continuation = index + 1;
      while (raw[continuation] === " " || raw[continuation] === "\t") continuation += 1;
      if (raw[continuation] === "\r" && raw[continuation + 1] === "\n") continuation += 2;
      else if (raw[continuation] === "\n") continuation += 1;
      else continuation = -1;
      if (continuation !== -1) {
        while (continuation < raw.length) {
          if (raw[continuation] === " " || raw[continuation] === "\t" || raw[continuation] === "\n") {
            continuation += 1;
            continue;
          }
          if (raw[continuation] === "\r" && raw[continuation + 1] === "\n") {
            continuation += 2;
            continue;
          }
          break;
        }
        index = continuation - 1;
        continue;
      }
    }
    const escape = raw[++index];
    if (escape === undefined) fail(`trailing string escape on line ${line}`);
    const simple: Readonly<Record<string, string>> = {
      "\\": "\\", '"': '"', n: "\n", t: "\t", r: "\r", b: "\b", f: "\f",
    };
    if (Object.hasOwn(simple, escape)) { result += simple[escape]!; continue; }
    if (escape === "u" || escape === "U") {
      const length = escape === "u" ? 4 : 8;
      const hex = raw.slice(index + 1, index + 1 + length);
      if (hex.length !== length || !/^[0-9a-fA-F]+$/.test(hex)) {
        fail(`invalid \\${escape} escape on line ${line}`);
      }
      const codePoint = Number.parseInt(hex, 16);
      if (codePoint > 0x10ffff || (codePoint >= 0xd800 && codePoint <= 0xdfff)) {
        fail(`invalid Unicode scalar in \\${escape} escape on line ${line}`);
      }
      result += String.fromCodePoint(codePoint);
      index += length;
      continue;
    }
    fail(`unsupported escape \\${escape} on line ${line}`);
  }
  return result;
}

function trimInitialMultilineNewline(raw: string): string {
  if (raw.startsWith("\r\n")) return raw.slice(2);
  return raw.startsWith("\n") ? raw.slice(1) : raw;
}

function validateRawStringToken(token: TomlToken, path: readonly string[]): void {
  if (!token.type.includes("STRING")) return;
  const multiline = token.type.startsWith("ML_");
  for (let index = 0; index < token.value.length; index += 1) {
    const code = token.value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdfff) {
      const next = token.value.charCodeAt(index + 1);
      if (code <= 0xdbff && next >= 0xdc00 && next <= 0xdfff) {
        index += 1;
        continue;
      }
      fail(`string at ${formatPath(path)} contains an invalid Unicode scalar`);
    }
    if (code === 0x09) continue;
    if (multiline && code === 0x0a) continue;
    if (multiline && code === 0x0d && token.value.charCodeAt(index + 1) === 0x0a) continue;
    if (code <= 0x1f || code === 0x7f) {
      fail(`string at ${formatPath(path)} contains a forbidden raw control character`);
    }
  }
}

function validateDateTime(raw: string, type: string, path: readonly string[]): void {
  const dateMatch = /^(\d{4})-(\d{2})-(\d{2})/.exec(raw);
  if (type !== "LOCAL_TIME") {
    if (dateMatch === null || !validDate(Number(dateMatch[1]), Number(dateMatch[2]), Number(dateMatch[3]))) {
      fail(`date/time at ${formatPath(path)} has an invalid calendar date`);
    }
  }
  if (type === "LOCAL_DATE") return;
  const timeMatch = /(?:^|[T ])(\d{2}):(\d{2}):(\d{2})(?:\.\d+)?/.exec(raw);
  if (timeMatch === null || Number(timeMatch[1]) > 23 || Number(timeMatch[2]) > 59 || Number(timeMatch[3]) > 60) {
    fail(`date/time at ${formatPath(path)} has an invalid clock time`);
  }
  if (type === "OFFSET_DATETIME" && !raw.endsWith("Z")) {
    const offset = /[+-](\d{2}):(\d{2})$/.exec(raw);
    if (offset === null || Number(offset[1]) > 23 || Number(offset[2]) > 59) {
      fail(`date/time at ${formatPath(path)} has an invalid UTC offset`);
    }
  }
}

function validDate(year: number, month: number, day: number): boolean {
  if (month < 1 || month > 12 || day < 1) return false;
  const leap = year % 4 === 0 && (year % 100 !== 0 || year % 400 === 0);
  const days = [31, leap ? 29 : 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
  return day <= days[month - 1]!;
}

function lowerPipeline(raw: TomlTable): PipelineConfig {
  closed(raw, ["name", "settings", "stages", "wires", "outputs"], "$" );
  const name = requiredString(raw["name"], "name");
  const settings = lowerSettings(requiredTable(raw["settings"], "settings"));
  const stages = requiredTableArray(raw["stages"], "stages").map(lowerStage);
  if (stages.length === 0) fail("stages must contain at least one table");
  const wires = raw["wires"] === undefined
    ? undefined
    : requiredTableArray(raw["wires"], "wires").map(lowerWire);
  const outputs = raw["outputs"] === undefined
    ? undefined
    : requiredTableArray(raw["outputs"], "outputs").map(lowerOutput);
  return {
    name,
    settings,
    stages,
    ...(wires === undefined ? {} : { wires }),
    ...(outputs === undefined ? {} : { outputs }),
  };
}

function lowerSettings(raw: TomlTable): PipelineSettings {
  closed(raw, [
    "storage-root", "cache-dir", "reproducible-build", "max-concurrency",
    "log-level", "best-effort", "deadline-ms", "plugin-runtimes",
  ], "settings");
  const logLevel = requiredString(raw["log-level"], "settings.log-level");
  if (!LOG_LEVELS.has(logLevel)) fail("settings.log-level must be one of trace|debug|info|warn|error");
  const maxConcurrency = optionalPositiveInteger(raw["max-concurrency"], "settings.max-concurrency");
  const deadlineMs = optionalPositiveNumber(raw["deadline-ms"], "settings.deadline-ms");
  const pluginRuntimes = raw["plugin-runtimes"] === undefined
    ? undefined
    : lowerPluginRuntimes(requiredTable(raw["plugin-runtimes"], "settings.plugin-runtimes"));
  return {
    storageRoot: requiredString(raw["storage-root"], "settings.storage-root"),
    cacheDir: requiredString(raw["cache-dir"], "settings.cache-dir"),
    reproducibleBuild: requiredBoolean(raw["reproducible-build"], "settings.reproducible-build"),
    maxConcurrency,
    logLevel: logLevel as PipelineSettings["logLevel"],
    bestEffort: requiredBoolean(raw["best-effort"], "settings.best-effort"),
    deadlineMs,
    ...(pluginRuntimes === undefined ? {} : { pluginRuntimes }),
  };
}

function lowerPluginRuntimes(raw: TomlTable): NonNullable<PipelineSettings["pluginRuntimes"]> {
  closed(raw, ["deno", "bun", "python"], "settings.plugin-runtimes");
  const result: Partial<Record<"deno" | "bun" | "python", PluginRuntimeConfig>> = {};
  for (const kind of ["deno", "bun", "python"] as const) {
    if (raw[kind] === undefined) continue;
    const runtime = requiredTable(raw[kind], `settings.plugin-runtimes.${kind}`);
    closed(runtime, ["executable", "root"], `settings.plugin-runtimes.${kind}`);
    result[kind] = {
      executable: requiredString(runtime["executable"], `settings.plugin-runtimes.${kind}.executable`),
      root: requiredString(runtime["root"], `settings.plugin-runtimes.${kind}.root`),
    };
  }
  return result;
}

function lowerStage(raw: TomlTable, index: number): StageInstanceSpec {
  const path = `stages[${index}]`;
  closed(raw, ["stage", "export", "id", "config", "capabilities"], path);
  const packageName = requiredString(raw["stage"], `${path}.stage`);
  const exported = optionalString(raw["export"], `${path}.export`);
  const id = optionalString(raw["id"], `${path}.id`);
  const capabilities = raw["capabilities"] === undefined
    ? undefined
    : stringArray(raw["capabilities"], `${path}.capabilities`);
  return {
    stage: { kind: "stage-ref", packageName, ...(exported === undefined ? {} : { export: exported }) },
    ...(id === undefined ? {} : { id }),
    ...(raw["config"] === undefined ? {} : { config: raw["config"] }),
    ...(capabilities === undefined ? {} : { capabilities }),
  };
}

function lowerWire(raw: TomlTable, index: number): EdgeSpec {
  const path = `wires[${index}]`;
  closed(raw, ["from", "to"], path);
  return {
    from: lowerEndpoint(requiredTable(raw["from"], `${path}.from`), `${path}.from`),
    to: lowerEndpoint(requiredTable(raw["to"], `${path}.to`), `${path}.to`),
  };
}

function lowerEndpoint(raw: TomlTable, path: string): EdgeSpec["from"] {
  closed(raw, ["id", "port"], path);
  const port = optionalString(raw["port"], `${path}.port`);
  return { id: requiredString(raw["id"], `${path}.id`), ...(port === undefined ? {} : { port }) };
}

function lowerOutput(raw: TomlTable, index: number): OutputSpec {
  const path = `outputs[${index}]`;
  closed(raw, ["from-instance", "name"], path);
  return {
    fromInstance: requiredString(raw["from-instance"], `${path}.from-instance`),
    name: requiredString(raw["name"], `${path}.name`),
  };
}

function closed(raw: TomlTable, allowed: readonly string[], path: string): void {
  const accepted = new Set(allowed);
  for (const key of Object.keys(raw)) {
    if (!accepted.has(key)) fail(`${path === "$" ? "" : `${path}.`}${key} is not supported`);
  }
}

function requiredTable(value: TomlValue | undefined, path: string): TomlTable {
  if (!isTable(value)) fail(`${path} must be a table`);
  return value;
}

function requiredTableArray(value: TomlValue | undefined, path: string): TomlTable[] {
  if (!Array.isArray(value) || !value.every(isTable)) fail(`${path} must be an array of tables`);
  return value;
}

function requiredString(value: TomlValue | undefined, path: string): string {
  if (typeof value !== "string" || value.length === 0) fail(`${path} must be a non-empty string`);
  return value;
}

function optionalString(value: TomlValue | undefined, path: string): string | undefined {
  return value === undefined ? undefined : requiredString(value, path);
}

function requiredBoolean(value: TomlValue | undefined, path: string): boolean {
  if (typeof value !== "boolean") fail(`${path} must be a boolean`);
  return value;
}

function optionalPositiveInteger(value: TomlValue | undefined, path: string): number | null {
  if (value === undefined) return null;
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 1) {
    fail(`${path} must be a positive safe integer when present`);
  }
  return value;
}

function optionalPositiveNumber(value: TomlValue | undefined, path: string): number | null {
  if (value === undefined) return null;
  if (typeof value !== "number" || !Number.isFinite(value) || value <= 0) {
    fail(`${path} must be a positive finite number when present`);
  }
  return value;
}

function stringArray(value: TomlValue, path: string): string[] {
  if (!Array.isArray(value) || !value.every(item => typeof item === "string")) {
    fail(`${path} must be an array of strings`);
  }
  return value as string[];
}

function isTable(value: TomlValue | undefined): value is TomlTable {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function formatPath(path: readonly string[]): string {
  return path.length === 0 ? "$" : path.join(".");
}

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function safeAscii(value: string, limit = 4096): string {
  let result = "";
  for (const character of value) {
    const point = character.codePointAt(0)!;
    const rendered = point >= 0x20 && point <= 0x7e
      ? character
      : `\\u{${point.toString(16)}}`;
    if (result.length + rendered.length > limit - 3) return `${result}...`;
    result += rendered;
  }
  return result;
}
