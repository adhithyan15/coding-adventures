import { createHash } from "node:crypto";
import { closeSync, fstatSync, openSync, readSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  Barcode1DError,
  computeLayoutV1,
  expandBinaryV1,
  expandWidthV1,
  projectSceneV1,
  type Barcode1DRun,
  type Barcode1DRunColor,
  type Barcode1DRunRole,
  type Barcode1DSymbolDescriptor,
  type PaintBarcode1DV1Options,
} from "../src/index.js";

type JsonObject = Record<string, any>;
const casesUrl = new URL("../../../../specs/fixtures/barcode-layout-1d-v1/cases.json", import.meta.url);
const expectedCorpusSha = "be95aa0381041ef3bd729b36bb4292f7a20692e139b53adca97af4e157cb7388";
const MAX_FIXTURE_BYTES = 131_072;
const MAX_FIXTURE_DEPTH = 8;

function sha256(value: string | Uint8Array): string {
  return createHash("sha256").update(value).digest("hex");
}

class FixtureLoadError extends Error {
  constructor(public readonly errorId: string) {
    super(errorId);
    this.name = "FixtureLoadError";
  }
}

function failFixture(errorId: string): never {
  throw new FixtureLoadError(errorId);
}

function assertScalarString(value: string): void {
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (!(next >= 0xdc00 && next <= 0xdfff)) failFixture("fixture-invalid-scalar");
      index += 1;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      failFixture("fixture-invalid-scalar");
    }
  }
  if (Array.from(value).length > 65_568) failFixture("fixture-scalar-limit");
}

/** Lexically validates depth and duplicate names before JSON.parse can erase evidence. */
function scanJsonEnvelope(text: string): void {
  let index = 0;
  const skipWhitespace = (): void => {
    while (index < text.length && /[\t\n\r ]/.test(text[index])) index += 1;
  };
  const parseString = (): string => {
    const start = index;
    index += 1;
    while (index < text.length) {
      const character = text[index];
      if (character === '"') {
        index += 1;
        try {
          const value = JSON.parse(text.slice(start, index)) as unknown;
          if (typeof value !== "string") failFixture("fixture-invalid-json");
          assertScalarString(value);
          return value;
        } catch (error) {
          if (error instanceof FixtureLoadError) throw error;
          failFixture("fixture-invalid-json");
        }
      }
      if (character.charCodeAt(0) <= 0x1f) failFixture("fixture-invalid-json");
      if (character === "\\") {
        index += 1;
        const escape = text[index];
        if (escape === "u") {
          if (!/^[0-9a-fA-F]{4}$/.test(text.slice(index + 1, index + 5))) failFixture("fixture-invalid-json");
          index += 5;
          continue;
        }
        if (escape === undefined || !'"\\/bfnrt'.includes(escape)) failFixture("fixture-invalid-json");
      }
      index += 1;
    }
    failFixture("fixture-invalid-json");
  };
  const scanValue = (depth: number): void => {
    skipWhitespace();
    const character = text[index];
    if (character === '"') {
      parseString();
      return;
    }
    if (character === "{" || character === "[") {
      const nextDepth = depth + 1;
      if (nextDepth > MAX_FIXTURE_DEPTH) failFixture("fixture-depth-limit");
      const object = character === "{";
      const close = object ? "}" : "]";
      index += 1;
      skipWhitespace();
      if (text[index] === close) {
        index += 1;
        return;
      }
      const names = new Set<string>();
      while (index < text.length) {
        if (object) {
          if (text[index] !== '"') failFixture("fixture-invalid-json");
          const name = parseString();
          if (names.has(name)) failFixture("fixture-duplicate-key");
          names.add(name);
          skipWhitespace();
          if (text[index] !== ":") failFixture("fixture-invalid-json");
          index += 1;
        }
        scanValue(nextDepth);
        skipWhitespace();
        if (text[index] === close) {
          index += 1;
          return;
        }
        if (text[index] !== ",") failFixture("fixture-invalid-json");
        index += 1;
        skipWhitespace();
      }
      failFixture("fixture-invalid-json");
    }
    const remainder = text.slice(index);
    const token = remainder.match(/^(?:true|false|null|-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?)/)?.[0];
    if (token === undefined) failFixture("fixture-invalid-json");
    index += token.length;
  };
  scanValue(0);
  skipWhitespace();
  if (index !== text.length) failFixture("fixture-invalid-json");
}

function validateTree(value: unknown, depth = 0): void {
  if (depth > MAX_FIXTURE_DEPTH) failFixture("fixture-depth-limit");
  if (typeof value === "string") {
    assertScalarString(value);
  } else if (typeof value === "number") {
    if (!Number.isSafeInteger(value)) failFixture("fixture-invalid-type");
  } else if (Array.isArray(value)) {
    for (const item of value) validateTree(item, depth + 1);
  } else if (value !== null && typeof value === "object") {
    for (const [key, item] of Object.entries(value)) {
      assertScalarString(key);
      if (Array.from(key).length > 128) failFixture("fixture-scalar-limit");
      validateTree(item, depth + 1);
    }
  } else if (value !== null && typeof value !== "boolean") {
    failFixture("fixture-invalid-type");
  }
}

function record(value: unknown): JsonObject {
  if (value === null || typeof value !== "object" || Array.isArray(value)) failFixture("fixture-invalid-type");
  return value as JsonObject;
}

function validateDocumentEnvelope(value: unknown): { cases: JsonObject[] } {
  const root = record(value);
  if (root.schema_version !== 1 || root.profile !== "barcode-layout-1d-v1") failFixture("fixture-invalid-type");
  record(root.limits);
  if (!Array.isArray(root.error_ids) || !root.error_ids.every((item: unknown) => typeof item === "string")) {
    failFixture("fixture-invalid-type");
  }
  if (!Array.isArray(root.cases) || root.cases.length > 64) failFixture("fixture-invalid-type");
  const ids = new Set<string>();
  const operations = new Set(["expand-binary", "expand-width", "compute-layout", "project-scene"]);
  for (const item of root.cases) {
    const testCase = record(item);
    if (typeof testCase.id !== "string" || ids.has(testCase.id)) failFixture("fixture-invalid-type");
    ids.add(testCase.id);
    if (typeof testCase.operation !== "string" || !operations.has(testCase.operation)) failFixture("fixture-invalid-type");
    record(testCase.input);
    record(testCase.expected);
  }
  return root as { cases: JsonObject[] };
}

function loadCorpus(raw: Uint8Array, expectedSha: string): { cases: JsonObject[] } {
  if (raw.byteLength > MAX_FIXTURE_BYTES) failFixture("fixture-size-limit");
  if (sha256(raw) !== expectedSha) failFixture("fixture-sha256-mismatch");
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(raw);
  } catch {
    failFixture("fixture-invalid-json");
  }
  scanJsonEnvelope(text);
  let parsed: unknown;
  try {
    parsed = JSON.parse(text) as unknown;
  } catch {
    failFixture("fixture-invalid-json");
  }
  validateTree(parsed);
  return validateDocumentEnvelope(parsed);
}

function readBoundedFile(url: URL): Uint8Array {
  const descriptor = openSync(url, "r");
  try {
    const size = fstatSync(descriptor).size;
    if (size > MAX_FIXTURE_BYTES) failFixture("fixture-size-limit");
    const result = new Uint8Array(size);
    let offset = 0;
    while (offset < size) {
      const count = readSync(descriptor, result, offset, size - offset, offset);
      if (count === 0) failFixture("fixture-invalid-json");
      offset += count;
    }
    return result;
  } finally {
    closeSync(descriptor);
  }
}

function fixtureInteger(value: unknown): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value) || value < 0) failFixture("fixture-invalid-type");
  return value;
}

function fixtureString(value: unknown): string {
  if (typeof value !== "string") failFixture("fixture-invalid-type");
  return value;
}

const rawCases = readBoundedFile(casesUrl);
const document = loadCorpus(rawCases, expectedCorpusSha);

function materializePattern(input: JsonObject): string {
  if (typeof input.pattern === "string") return input.pattern;
  const repeat = record(input.repeat);
  const token = fixtureString(repeat.token);
  const count = fixtureInteger(repeat.count);
  const suffix = repeat.suffix === undefined ? "" : fixtureString(repeat.suffix);
  const scalarCount = Array.from(token).length * count + Array.from(suffix).length;
  if (!Number.isSafeInteger(scalarCount)) throw new Barcode1DError("pattern-too-long");
  if (scalarCount > 65_567) throw new Barcode1DError("pattern-too-long");
  return token.repeat(count) + suffix;
}

function run(value: JsonObject): Barcode1DRun {
  return {
    color: value.color as Barcode1DRunColor,
    modules: value.modules as number,
    sourceLabel: value.sourceLabel as string,
    sourceIndex: value.sourceIndex as number,
    role: value.role as Barcode1DRunRole,
  };
}

function materializeRuns(input: JsonObject): Barcode1DRun[] {
  if (Array.isArray(input.runs)) {
    if (input.runs.length > 40_979) throw new Barcode1DError("too-many-runs");
    return input.runs.map((value: unknown) => run(record(value)));
  }
  const repeated = record(input.repeatRuns);
  const count = fixtureInteger(repeated.count);
  if (count > 40_979) throw new Barcode1DError("too-many-runs");
  fixtureString(repeated.firstColor);
  fixtureInteger(repeated.modules);
  fixtureString(repeated.sourceLabel);
  if (typeof repeated.sourceIndex !== "number" || !Number.isSafeInteger(repeated.sourceIndex)) failFixture("fixture-invalid-type");
  fixtureString(repeated.role);
  const result: Barcode1DRun[] = [];
  let color = repeated.firstColor as Barcode1DRunColor;
  for (let index = 0; index < count; index += 1) {
    result.push({
      color,
      modules: repeated.modules,
      sourceLabel: repeated.sourceLabel,
      sourceIndex: repeated.sourceIndex,
      role: repeated.role,
    });
    color = color === "bar" ? "space" : "bar";
  }
  return result;
}

function symbols(input: JsonObject): Barcode1DSymbolDescriptor[] | undefined {
  if (Array.isArray(input.symbols)) {
    if (input.symbols.length > 40_979) throw new Barcode1DError("too-many-symbols");
    return input.symbols.map((value: unknown) => ({ ...record(value) })) as Barcode1DSymbolDescriptor[];
  }
  if (input.repeatSymbols === undefined) return undefined;
  const repeated = record(input.repeatSymbols);
  const count = fixtureInteger(repeated.count);
  if (count > 40_979) throw new Barcode1DError("too-many-symbols");
  fixtureString(repeated.label);
  fixtureInteger(repeated.modules);
  fixtureString(repeated.role);
  return Array.from({ length: count }, (_, sourceIndex) => ({
    label: repeated.label,
    modules: repeated.modules,
    sourceIndex,
    role: repeated.role,
  }));
}

function canonicalJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(",")}]`;
  if (value !== null && typeof value === "object") {
    const entries = Object.entries(value as Record<string, unknown>).sort(([left], [right]) => {
      const a = Array.from(left, (character) => character.codePointAt(0) ?? 0);
      const b = Array.from(right, (character) => character.codePointAt(0) ?? 0);
      for (let index = 0; index < Math.min(a.length, b.length); index += 1) {
        if (a[index] !== b[index]) return a[index] - b[index];
      }
      return a.length - b.length;
    });
    return `{${entries.map(([key, item]) => `${JSON.stringify(key)}:${canonicalJson(item)}`).join(",")}}`;
  }
  return JSON.stringify(value);
}

function runResult(runs: Barcode1DRun[], digest: boolean): JsonObject {
  if (!digest) return { runs };
  return {
    runDigest: {
      runCount: runs.length,
      contentModules: runs.reduce((sum, item) => sum + item.modules, 0),
      firstRun: runs[0],
      lastRun: runs[runs.length - 1],
      runsSha256: sha256(canonicalJson(runs)),
    },
  };
}

function sceneResult(scene: ReturnType<typeof projectSceneV1>): JsonObject {
  return {
    width: scene.width,
    height: scene.height,
    background: scene.background,
    rectangles: scene.instructions.map((instruction) => {
      if (instruction.kind !== "rect") throw new Error("portable scene emitted a non-rectangle instruction");
      return {
        x: instruction.x,
        y: instruction.y,
        width: instruction.width,
        height: instruction.height,
        fill: instruction.fill,
        metadata: instruction.metadata,
      };
    }),
    metadata: scene.metadata,
  };
}

function execute(testCase: JsonObject): JsonObject {
  const input = testCase.input as JsonObject;
  switch (testCase.operation) {
    case "expand-binary": {
      const runs = expandBinaryV1(materializePattern(input), {
        sourceLabel: input.sourceLabel,
        sourceIndex: input.sourceIndex,
        role: input.role,
      });
      return runResult(runs, testCase.expected.runDigest !== undefined);
    }
    case "expand-width": {
      const runs = expandWidthV1(materializePattern(input), {
        sourceLabel: input.sourceLabel,
        sourceIndex: input.sourceIndex,
        role: input.role,
        narrowMarker: input.narrowMarker,
        wideMarker: input.wideMarker,
        narrowModules: input.narrowModules,
        wideModules: input.wideModules,
        startingColor: input.startingColor,
      });
      return runResult(runs, testCase.expected.runDigest !== undefined);
    }
    case "compute-layout":
      return {
        layout: computeLayoutV1(materializeRuns(input), {
          quietZoneModules: input.quietZoneModules,
          symbols: symbols(input),
        }),
      };
    case "project-scene": {
      const options: PaintBarcode1DV1Options = {
        renderConfig: input.renderConfig,
        quietZoneModules: input.quietZoneModules,
        humanReadableText: input.humanReadableText,
        metadata: input.metadata,
        label: input.label,
        symbols: symbols(input),
      };
      return { scene: sceneResult(projectSceneV1(materializeRuns(input), options)) };
    }
    default:
      throw new Error(`unknown operation ${String(testCase.operation)}`);
  }
}

describe("barcode-layout-1d-v1 portable corpus", () => {
  it("binds the exact corpus and complete operation inventory", () => {
    expect(sha256(rawCases)).toBe(expectedCorpusSha);
    expect(document.cases).toHaveLength(56);
    const counts = Object.fromEntries(
      ["expand-binary", "expand-width", "compute-layout", "project-scene"].map((operation) => [
        operation,
        document.cases.filter((testCase) => testCase.operation === operation).length,
      ]),
    );
    expect(counts).toEqual({ "expand-binary": 12, "expand-width": 12, "compute-layout": 19, "project-scene": 13 });
  });

  for (const testCase of document.cases) {
    it(testCase.id, () => {
      if (typeof testCase.expected.error === "string") {
        try {
          execute(testCase);
          throw new Error("expected portable-v1 failure");
        } catch (error) {
          expect(error).toBeInstanceOf(Barcode1DError);
          expect((error as Barcode1DError).errorId).toBe(testCase.expected.error);
        }
      } else {
        expect(execute(testCase)).toEqual(testCase.expected);
      }
    });
  }

  it("rejects both text forms before native dispatch", () => {
    const invalidRuns: Barcode1DRun[] = [{ color: "bar", modules: 0, sourceLabel: "A", sourceIndex: 0, role: "data" }];
    expect(() => projectSceneV1(invalidRuns, { renderConfig: { includeHumanReadableText: true } })).toThrowError(
      expect.objectContaining({ errorId: "human-readable-text-unsupported" }),
    );
    expect(() => projectSceneV1(invalidRuns, { humanReadableText: "A" })).toThrowError(
      expect.objectContaining({ errorId: "human-readable-text-unsupported" }),
    );
  });

  it("returns deep-owned values", () => {
    const options = { sourceLabel: "A", sourceIndex: 0, role: "data" as const };
    const first = expandBinaryV1("101", options);
    const second = expandBinaryV1("101", options);
    first[0].sourceLabel = "mutated";
    expect(second[0].sourceLabel).toBe("A");
    expect(options.sourceLabel).toBe("A");
  });

  it("rejects runtime enum escapes at every v1 ingress", () => {
    const invalidColor = [{ color: "ink", modules: 1, sourceLabel: "A", sourceIndex: 0, role: "data" }] as unknown as Barcode1DRun[];
    expect(() => computeLayoutV1(invalidColor, { quietZoneModules: 1 })).toThrowError(
      expect.objectContaining({ errorId: "invalid-source-attribution" }),
    );

    const invalidRole = [{ color: "bar", modules: 1, sourceLabel: "A", sourceIndex: 0, role: "payload" }] as unknown as Barcode1DRun[];
    expect(() => computeLayoutV1(invalidRole, { quietZoneModules: 1 })).toThrowError(
      expect.objectContaining({ errorId: "invalid-source-attribution" }),
    );
    expect(() => expandBinaryV1("1", { sourceLabel: "A", sourceIndex: 0, role: "payload" as Barcode1DRunRole })).toThrowError(
      expect.objectContaining({ errorId: "invalid-source-attribution" }),
    );
    expect(() => expandWidthV1("N", { sourceLabel: "A", sourceIndex: 0, role: "payload" as Barcode1DRunRole })).toThrowError(
      expect.objectContaining({ errorId: "invalid-source-attribution" }),
    );
    expect(() => expandWidthV1("N", {
      sourceLabel: "A",
      sourceIndex: 0,
      role: "data",
      startingColor: "ink" as Barcode1DRunColor,
    })).toThrowError(expect.objectContaining({ errorId: "invalid-marker-configuration" }));

    const validRuns: Barcode1DRun[] = [{ color: "bar", modules: 1, sourceLabel: "A", sourceIndex: 0, role: "data" }];
    const invalidSymbols = [{ label: "A", modules: 1, sourceIndex: 0, role: "inter-character-gap" }] as unknown as Barcode1DSymbolDescriptor[];
    expect(() => computeLayoutV1(validRuns, { quietZoneModules: 1, symbols: invalidSymbols })).toThrowError(
      expect.objectContaining({ errorId: "invalid-source-attribution" }),
    );
  });

  it("maps malformed UTF-16 patterns to the operation-specific token error", () => {
    const malformed = String.fromCharCode(0xd800);
    const options = { sourceLabel: "A", sourceIndex: 0, role: "data" as const };
    expect(() => expandBinaryV1(malformed, options)).toThrowError(
      expect.objectContaining({ errorId: "invalid-binary-token" }),
    );
    expect(() => expandWidthV1(malformed, options)).toThrowError(
      expect.objectContaining({ errorId: "invalid-width-token" }),
    );
  });
});

describe("bounded target corpus loader", () => {
  const encode = (text: string): Uint8Array => new TextEncoder().encode(text);
  const loadWithOwnDigest = (text: string): { cases: JsonObject[] } => {
    const raw = encode(text);
    return loadCorpus(raw, sha256(raw));
  };

  it("enforces the byte ceiling before digest or parsing", () => {
    const oversized = new Uint8Array(MAX_FIXTURE_BYTES + 1);
    expect(() => loadCorpus(oversized, "not-a-digest")).toThrowError(
      expect.objectContaining({ errorId: "fixture-size-limit" }),
    );
  });

  it("verifies the raw SHA before decoding or parsing", () => {
    expect(() => loadCorpus(encode("{"), "not-the-raw-sha")).toThrowError(
      expect.objectContaining({ errorId: "fixture-sha256-mismatch" }),
    );
  });

  it("rejects duplicate object names before JSON.parse can erase them", () => {
    const duplicate = '{"a":1,"a":2}';
    expect(() => loadWithOwnDigest(duplicate)).toThrowError(
      expect.objectContaining({ errorId: "fixture-duplicate-key" }),
    );
  });

  it("rejects excessive depth, malformed scalars, and invalid structural types", () => {
    const deep = `${"[".repeat(MAX_FIXTURE_DEPTH + 1)}${"]".repeat(MAX_FIXTURE_DEPTH + 1)}`;
    expect(() => loadWithOwnDigest(deep)).toThrowError(
      expect.objectContaining({ errorId: "fixture-depth-limit" }),
    );
    expect(() => loadWithOwnDigest('"\\ud800"')).toThrowError(
      expect.objectContaining({ errorId: "fixture-invalid-scalar" }),
    );
    const invalidType = JSON.stringify({
      schema_version: 1,
      profile: "barcode-layout-1d-v1",
      limits: {},
      error_ids: [],
      cases: {},
    });
    expect(() => loadWithOwnDigest(invalidType)).toThrowError(
      expect.objectContaining({ errorId: "fixture-invalid-type" }),
    );
  });

  it("rejects invalid repeat shapes before materialization", () => {
    expect(() => materializePattern({ repeat: { token: "1", count: -1 } })).toThrowError(
      expect.objectContaining({ errorId: "fixture-invalid-type" }),
    );
    expect(() => materializeRuns({ repeatRuns: { count: 40_980 } })).toThrowError(
      expect.objectContaining({ errorId: "too-many-runs" }),
    );
    expect(() => symbols({ repeatSymbols: { count: 40_980 } })).toThrowError(
      expect.objectContaining({ errorId: "too-many-symbols" }),
    );
  });
});
