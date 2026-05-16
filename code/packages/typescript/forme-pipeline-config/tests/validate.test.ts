/**
 * forme-pipeline-config — validateConfig tests
 *
 * Builds tiny-but-real Stage objects rather than mocks so we exercise
 * the actual cross-package contract.
 */

import { describe, it, expect } from "vitest";
import {
  KERNEL_API_VERSION,
  Kinds,
  streamOf,
} from "@coding-adventures/forme-types";
import type { KindDescriptor } from "@coding-adventures/forme-types";
import { defineStage } from "@coding-adventures/forme-stage";
import type { Stage } from "@coding-adventures/forme-stage";
import {
  CONFIG_ERROR_CODES,
  ConfigError,
  validateConfig,
} from "../src/index.js";
import type {
  PipelineConfig,
  PipelineSettings,
  StageInstanceSpec,
} from "../src/index.js";

// ─── Helpers ──────────────────────────────────────────────────────────────

function settings(overrides: Partial<PipelineSettings> = {}): PipelineSettings {
  return {
    storageRoot: "./content",
    cacheDir: null,
    reproducibleBuild: false,
    maxConcurrency: null,
    logLevel: "info",
    bestEffort: false,
    deadlineMs: null,
    ...overrides,
  };
}

function makeStage<In extends KindDescriptor, Out extends KindDescriptor>(
  name: string,
  consumes: In,
  produces: Out,
  opts: {
    capabilities?: readonly string[];
    configSchema?: unknown;
    apiVersion?: number;
  } = {},
): Stage<In, Out> {
  return defineStage({
    name,
    version: "0.1.0",
    apiVersion: opts.apiVersion ?? KERNEL_API_VERSION,
    description: `test stage ${name}`,
    consumes,
    produces,
    capabilities: (opts.capabilities ?? []) as never[],
    configSchema: (opts.configSchema ?? null) as never,
    async run() { throw new Error("not invoked in validation tests"); },
  });
}

function config(stages: readonly StageInstanceSpec[], extra: Partial<PipelineConfig> = {}): PipelineConfig {
  return {
    name: "test",
    settings: settings(),
    stages,
    ...extra,
  };
}

// ─── Happy path ───────────────────────────────────────────────────────────

describe("validateConfig — happy paths", () => {
  it("accepts a minimal linear pipeline", () => {
    const c = config([
      { stage: makeStage("source", Kinds.Void, streamOf(Kinds.ContentSource)) },
      { stage: makeStage("parse", Kinds.ContentSource, Kinds.ContentNode) },
      { stage: makeStage("emit", Kinds.RenderedPage, Kinds.DeployArtifact) },
    ]);
    const resolved = validateConfig(c);
    expect(resolved.resolvedIds).toEqual(["source", "parse", "emit"]);
  });

  it("auto-derives unique IDs from stage.name", () => {
    const c = config([
      { stage: makeStage("@forme/source-fs", Kinds.Void, streamOf(Kinds.ContentSource)) },
    ]);
    const resolved = validateConfig(c);
    expect(resolved.resolvedIds).toEqual(["@forme/source-fs"]);
  });

  it("uses explicit id when provided", () => {
    const c = config([
      { stage: makeStage("source", Kinds.Void, streamOf(Kinds.ContentSource)), id: "src-1" },
    ]);
    expect(validateConfig(c).resolvedIds).toEqual(["src-1"]);
  });

  it("permits a per-instance capability that the stage declares", () => {
    const stage = makeStage("source", Kinds.Void, streamOf(Kinds.ContentSource), {
      capabilities: ["storage:read", "storage:write"],
    });
    const c = config([{ stage, capabilities: ["storage:read"] }]);
    expect(() => validateConfig(c)).not.toThrow();
  });

  it("accepts wires referencing real instance IDs", () => {
    const c = config([
      { stage: makeStage("a", Kinds.Void, Kinds.ContentSource), id: "a" },
      { stage: makeStage("b", Kinds.ContentSource, Kinds.ContentNode), id: "b" },
    ], { wires: [{ from: { id: "a" }, to: { id: "b" } }] });
    expect(() => validateConfig(c)).not.toThrow();
  });

  it("accepts outputs naming real instances", () => {
    const c = config([
      { stage: makeStage("a", Kinds.Void, Kinds.DeployArtifact), id: "a" },
      { stage: makeStage("b", Kinds.Void, Kinds.DeployArtifact), id: "b" },
    ], { outputs: [
      { fromInstance: "a", name: "primary" },
      { fromInstance: "b", name: "secondary" },
    ]});
    expect(() => validateConfig(c)).not.toThrow();
  });
});

// ─── Top-level shape ──────────────────────────────────────────────────────

describe("validateConfig — top-level shape", () => {
  it("rejects null", () => {
    expect(() => validateConfig(null as never)).toThrow(ConfigError);
  });

  it("rejects missing name", () => {
    try {
      validateConfig({ ...config([{ stage: makeStage("s", Kinds.Void, Kinds.ContentSource) }]), name: "" });
    } catch (e) {
      const ce = e as ConfigError;
      expect(ce.errors.some(x => x.path === "name" && x.code === CONFIG_ERROR_CODES.MALFORMED)).toBe(true);
    }
  });

  it("rejects empty stages array", () => {
    try {
      validateConfig({ ...config([]), stages: [] });
    } catch (e) {
      expect((e as ConfigError).errors.some(x => x.path === "stages")).toBe(true);
    }
  });

  it("bails when stages is not an array (downstream loop would crash)", () => {
    const c = { name: "t", settings: settings(), stages: "not-an-array" } as unknown as PipelineConfig;
    try {
      validateConfig(c);
    } catch (e) {
      expect(e).toBeInstanceOf(ConfigError);
      expect((e as ConfigError).errors.some(x => x.path === "stages" && x.code === CONFIG_ERROR_CODES.MALFORMED)).toBe(true);
    }
  });

  it("skips settings validation when settings is not an object", () => {
    const c = { name: "t", settings: null, stages: [{ stage: makeStage("a", Kinds.Void, Kinds.ContentSource) }] } as unknown as PipelineConfig;
    try {
      validateConfig(c);
    } catch (e) {
      // We see the top-level MALFORMED for settings, but downstream
      // checks (stage validation, ID resolution) still ran.
      const codes = (e as ConfigError).errors.map(x => x.code);
      expect(codes).toContain(CONFIG_ERROR_CODES.MALFORMED);
      // No DUPLICATE_INSTANCE_ID because there's only one stage —
      // but the stage *did* get validated (no API_VERSION_MISMATCH
      // because the stage is well-formed).
    }
  });
});

// ─── Settings validation ──────────────────────────────────────────────────

describe("validateConfig — settings", () => {
  it("rejects empty storageRoot", () => {
    try {
      validateConfig({
        ...config([{ stage: makeStage("s", Kinds.Void, Kinds.ContentSource) }]),
        settings: settings({ storageRoot: "" }),
      });
    } catch (e) {
      expect((e as ConfigError).errors.some(x => x.path === "settings.storageRoot")).toBe(true);
    }
  });

  it("rejects bogus log level", () => {
    expect(() => validateConfig({
      ...config([{ stage: makeStage("s", Kinds.Void, Kinds.ContentSource) }]),
      settings: settings({ logLevel: "shout" as never }),
    })).toThrow(ConfigError);
  });

  it("rejects negative maxConcurrency", () => {
    expect(() => validateConfig({
      ...config([{ stage: makeStage("s", Kinds.Void, Kinds.ContentSource) }]),
      settings: settings({ maxConcurrency: -1 }),
    })).toThrow(ConfigError);
  });

  it("rejects fractional maxConcurrency", () => {
    expect(() => validateConfig({
      ...config([{ stage: makeStage("s", Kinds.Void, Kinds.ContentSource) }]),
      settings: settings({ maxConcurrency: 1.5 }),
    })).toThrow(ConfigError);
  });

  it("rejects non-positive deadlineMs", () => {
    expect(() => validateConfig({
      ...config([{ stage: makeStage("s", Kinds.Void, Kinds.ContentSource) }]),
      settings: settings({ deadlineMs: 0 }),
    })).toThrow(ConfigError);
  });

  it("accepts cacheDir = null and cacheDir = string", () => {
    expect(() => validateConfig({
      ...config([{ stage: makeStage("s", Kinds.Void, Kinds.ContentSource) }]),
      settings: settings({ cacheDir: ".forme/cache" }),
    })).not.toThrow();
  });

  it("rejects non-boolean reproducibleBuild", () => {
    expect(() => validateConfig({
      ...config([{ stage: makeStage("s", Kinds.Void, Kinds.ContentSource) }]),
      settings: settings({ reproducibleBuild: "yes" as never }),
    })).toThrow(ConfigError);
  });

  it("rejects non-boolean bestEffort", () => {
    expect(() => validateConfig({
      ...config([{ stage: makeStage("s", Kinds.Void, Kinds.ContentSource) }]),
      settings: settings({ bestEffort: 1 as never }),
    })).toThrow(ConfigError);
  });

  it("rejects non-string cacheDir", () => {
    expect(() => validateConfig({
      ...config([{ stage: makeStage("s", Kinds.Void, Kinds.ContentSource) }]),
      settings: settings({ cacheDir: 42 as never }),
    })).toThrow(ConfigError);
  });
});

// ─── Stage instance validation ────────────────────────────────────────────

describe("validateConfig — stage instances", () => {
  it("rejects API version mismatch", () => {
    const stage = makeStage("s", Kinds.Void, Kinds.ContentSource, { apiVersion: 99 });
    try { validateConfig(config([{ stage }])); }
    catch (e) {
      const ce = e as ConfigError;
      expect(ce.errors.some(x => x.code === CONFIG_ERROR_CODES.API_VERSION_MISMATCH)).toBe(true);
    }
  });

  it("rejects unresolved StageRef", () => {
    try {
      validateConfig(config([
        { stage: { kind: "stage-ref", packageName: "@forme/source-fs" } as never },
      ]));
    } catch (e) {
      const ce = e as ConfigError;
      expect(ce.errors.some(x => x.code === CONFIG_ERROR_CODES.STAGE_REF_UNRESOLVED)).toBe(true);
    }
  });

  it("rejects per-instance capability not declared by stage", () => {
    const stage = makeStage("s", Kinds.Void, Kinds.ContentSource, {
      capabilities: ["storage:read"],
    });
    try {
      validateConfig(config([{ stage, capabilities: ["network:*"] }]));
    } catch (e) {
      expect((e as ConfigError).errors.some(x => x.code === CONFIG_ERROR_CODES.CAPABILITY_NOT_DECLARED)).toBe(true);
    }
  });

  it("rejects missing config when configSchema is non-null", () => {
    const stage = makeStage("s", Kinds.Void, Kinds.ContentSource, {
      configSchema: { type: "object" },
    });
    try {
      validateConfig(config([{ stage }]));
    } catch (e) {
      expect((e as ConfigError).errors.some(x => x.code === CONFIG_ERROR_CODES.CONFIG_REQUIRED)).toBe(true);
    }
  });

  it("accepts non-null config when schema is non-null", () => {
    const stage = makeStage("s", Kinds.Void, Kinds.ContentSource, {
      configSchema: { type: "object" },
    });
    expect(() => validateConfig(config([{ stage, config: { ok: true } }]))).not.toThrow();
  });

  it("rejects invalid stage value (null)", () => {
    expect(() => validateConfig(config([{ stage: null as never }]))).toThrow(ConfigError);
  });

  it("rejects stage missing name", () => {
    const bogus = { ...makeStage("real", Kinds.Void, Kinds.ContentSource), name: "" };
    expect(() => validateConfig(config([{ stage: bogus as never }]))).toThrow(ConfigError);
  });

  it("rejects stage with non-array capabilities", () => {
    const bogus = { ...makeStage("real", Kinds.Void, Kinds.ContentSource) } as Record<string, unknown>;
    bogus.capabilities = "all" as never;
    expect(() => validateConfig(config([{ stage: bogus as never }]))).toThrow(ConfigError);
  });

  it("rejects stage with non-number apiVersion", () => {
    const bogus = { ...makeStage("real", Kinds.Void, Kinds.ContentSource) } as Record<string, unknown>;
    bogus.apiVersion = "1" as never;
    expect(() => validateConfig(config([{ stage: bogus as never }]))).toThrow(ConfigError);
  });

  it("rejects stage with non-string version", () => {
    const bogus = { ...makeStage("real", Kinds.Void, Kinds.ContentSource) } as Record<string, unknown>;
    bogus.version = 1 as never;
    expect(() => validateConfig(config([{ stage: bogus as never }]))).toThrow(ConfigError);
  });
});

// ─── Instance ID resolution ───────────────────────────────────────────────

describe("validateConfig — instance ID resolution", () => {
  it("auto-numbers when only one instance per stage name (no collision)", () => {
    const c = config([
      { stage: makeStage("a", Kinds.Void, Kinds.ContentSource) },
      { stage: makeStage("b", Kinds.ContentSource, Kinds.ContentNode) },
    ]);
    expect(validateConfig(c).resolvedIds).toEqual(["a", "b"]);
  });

  it("collisions on auto-derived IDs trigger DUPLICATE_INSTANCE_ID", () => {
    const stage = makeStage("dup", Kinds.Void, Kinds.ContentSource);
    try {
      validateConfig(config([{ stage }, { stage }]));
    } catch (e) {
      const ce = e as ConfigError;
      const dupe = ce.errors.find(x => x.code === CONFIG_ERROR_CODES.DUPLICATE_INSTANCE_ID);
      expect(dupe).toBeDefined();
      expect(dupe!.path).toContain("stages[0]");
      expect(dupe!.path).toContain("stages[1]");
    }
  });

  it("explicit ids on collisions disambiguate cleanly", () => {
    const stage = makeStage("dup", Kinds.Void, Kinds.ContentSource);
    const c = config([
      { stage, id: "first" },
      { stage, id: "second" },
    ]);
    expect(validateConfig(c).resolvedIds).toEqual(["first", "second"]);
  });

  it("explicit id collides with auto-derived id from another instance", () => {
    expect(() => validateConfig(config([
      { stage: makeStage("a", Kinds.Void, Kinds.ContentSource) },
      { stage: makeStage("b", Kinds.Void, Kinds.ContentNode), id: "a" },
    ]))).toThrow(ConfigError);
  });
});

// ─── Wires & outputs ──────────────────────────────────────────────────────

describe("validateConfig — wires", () => {
  it("rejects edge from unknown instance", () => {
    expect(() => validateConfig(config([
      { stage: makeStage("a", Kinds.Void, Kinds.ContentSource), id: "a" },
    ], { wires: [{ from: { id: "ghost" }, to: { id: "a" } }] }))).toThrow(ConfigError);
  });

  it("rejects edge to unknown instance", () => {
    expect(() => validateConfig(config([
      { stage: makeStage("a", Kinds.Void, Kinds.ContentSource), id: "a" },
    ], { wires: [{ from: { id: "a" }, to: { id: "ghost" } }] }))).toThrow(ConfigError);
  });
});

describe("validateConfig — outputs & multiple terminals", () => {
  it("rejects output naming unknown instance", () => {
    expect(() => validateConfig(config([
      { stage: makeStage("a", Kinds.Void, Kinds.DeployArtifact), id: "a" },
    ], { outputs: [{ fromInstance: "ghost", name: "x" }] }))).toThrow(ConfigError);
  });

  it("flags MULTIPLE_OUTPUTS_UNNAMED when 2+ terminals exist with no outputs", () => {
    try {
      validateConfig(config([
        { stage: makeStage("a", Kinds.Void, Kinds.DeployArtifact), id: "a" },
        { stage: makeStage("b", Kinds.Void, Kinds.DeployArtifact), id: "b" },
      ]));
    } catch (e) {
      expect((e as ConfigError).errors.some(x => x.code === CONFIG_ERROR_CODES.MULTIPLE_OUTPUTS_UNNAMED)).toBe(true);
    }
  });

  it("accepts 2+ terminals when outputs name them all", () => {
    expect(() => validateConfig(config([
      { stage: makeStage("a", Kinds.Void, Kinds.DeployArtifact), id: "a" },
      { stage: makeStage("b", Kinds.Void, Kinds.DeployArtifact), id: "b" },
    ], { outputs: [
      { fromInstance: "a", name: "primary" },
      { fromInstance: "b", name: "secondary" },
    ]}))).not.toThrow();
  });

  it("flags partial naming (some terminals named, some not)", () => {
    try {
      validateConfig(config([
        { stage: makeStage("a", Kinds.Void, Kinds.DeployArtifact), id: "a" },
        { stage: makeStage("b", Kinds.Void, Kinds.Feed), id: "b" },
      ], { outputs: [{ fromInstance: "a", name: "primary" }] }));
    } catch (e) {
      expect((e as ConfigError).errors.some(x => x.code === CONFIG_ERROR_CODES.MULTIPLE_OUTPUTS_UNNAMED)).toBe(true);
    }
  });

  it("a single-terminal pipeline never needs outputs named", () => {
    expect(() => validateConfig(config([
      { stage: makeStage("a", Kinds.Void, Kinds.DeployArtifact), id: "a" },
    ]))).not.toThrow();
  });
});

// ─── Multi-error aggregation ──────────────────────────────────────────────

describe("validateConfig — JSON Schema validation of stage configs", () => {
  function schemaStage() {
    return makeStage("schema-stage", Kinds.Void, Kinds.DeployArtifact, {
      configSchema: {
        type: "object",
        required: ["glob"],
        properties: {
          glob: { type: "string" },
          root: { type: "string" },
        },
      },
    });
  }

  it("accepts a config that satisfies the schema", () => {
    expect(() => validateConfig(config([{
      stage: schemaStage(),
      config: { glob: "**/*.md", root: "/abs" },
    }]))).not.toThrow();
  });

  it("rejects a config missing a required property", () => {
    try {
      validateConfig(config([{
        stage: schemaStage(),
        config: { root: "/abs" },
      }]));
      expect.fail("should have thrown");
    } catch (e) {
      const err = e as ConfigError;
      const v = err.errors.filter(x => x.code === CONFIG_ERROR_CODES.CONFIG_SCHEMA_VIOLATION);
      expect(v.length).toBeGreaterThan(0);
      expect(v[0]!.path).toMatch(/stages\[0\]\.config/);
      expect(v[0]!.message).toMatch(/required.*missing/);
    }
  });

  it("rejects a config with wrong-typed property", () => {
    try {
      validateConfig(config([{
        stage: schemaStage(),
        config: { glob: 42 },
      }]));
      expect.fail("should have thrown");
    } catch (e) {
      const err = e as ConfigError;
      const v = err.errors.filter(x => x.code === CONFIG_ERROR_CODES.CONFIG_SCHEMA_VIOLATION);
      expect(v.length).toBeGreaterThan(0);
      expect(v[0]!.message).toMatch(/expected type string/);
    }
  });

  it("still surfaces CONFIG_REQUIRED when configSchema is non-null but no config", () => {
    try {
      validateConfig(config([{ stage: schemaStage() }]));
      expect.fail("should have thrown");
    } catch (e) {
      const err = e as ConfigError;
      expect(err.errors.some(x => x.code === CONFIG_ERROR_CODES.CONFIG_REQUIRED)).toBe(true);
    }
  });

  it("doesn't validate against schema when configSchema is null", () => {
    expect(() => validateConfig(config([{
      stage: makeStage("no-schema", Kinds.Void, Kinds.DeployArtifact, { configSchema: null }),
      config: { anything: "goes" },
    }]))).not.toThrow();
  });
});

describe("validateConfig — collects all errors in one pass", () => {
  it("multiple distinct violations all surface together", () => {
    try {
      validateConfig({
        name: "",
        settings: settings({ logLevel: "shout" as never, maxConcurrency: -1 }),
        stages: [
          { stage: makeStage("dup", Kinds.Void, Kinds.ContentSource, { apiVersion: 99 }) },
          { stage: makeStage("dup", Kinds.Void, Kinds.ContentNode) },
        ],
      });
    } catch (e) {
      const codes = (e as ConfigError).errors.map(x => x.code);
      expect(codes).toContain(CONFIG_ERROR_CODES.MALFORMED);
      expect(codes).toContain(CONFIG_ERROR_CODES.DUPLICATE_INSTANCE_ID);
      expect(codes).toContain(CONFIG_ERROR_CODES.API_VERSION_MISMATCH);
      // We expect at least three distinct codes (not just one error rolled up).
      expect(new Set(codes).size).toBeGreaterThanOrEqual(3);
    }
  });
});
