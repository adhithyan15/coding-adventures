import { describe, expect, it } from "vitest";
import { CapabilityError, CancellationError, StageError } from "@coding-adventures/forme-errors";
import type { Manifest } from "@coding-adventures/forme-manifest";
import { PluginHostError, RpcRemoteError } from "../src/index.js";
import { __testing } from "../src/host.js";

describe("plugin host bounded internals", () => {
  it("queues buffered and waiting values, closure, failures, and bounds", async () => {
    const buffered = new __testing.AsyncQueue<number>(1);
    buffered.push(1);
    expect(await buffered[Symbol.asyncIterator]().next()).toEqual({ value: 1, done: false });
    const waiting = buffered[Symbol.asyncIterator]().next();
    buffered.push(2);
    expect(await waiting).toEqual({ value: 2, done: false });
    const completion = buffered[Symbol.asyncIterator]().next();
    buffered.close();
    buffered.close();
    expect(await completion).toEqual({ value: undefined, done: true });
    await expect(buffered[Symbol.asyncIterator]().next()).resolves.toEqual({ value: undefined, done: true });
    expect(() => buffered.push(3)).toThrow(/after stream completion/);

    const overflow = new __testing.AsyncQueue<number>(1, 8, () => 8);
    overflow.push(1);
    expect(() => overflow.push(2)).toThrow(/bounded count or byte buffer/);
    await expect(overflow[Symbol.asyncIterator]().next()).rejects.toThrow(/bounded count or byte buffer/);

    const byteOverflow = new __testing.AsyncQueue<string>(4, 3, value => value.length);
    byteOverflow.push("ab");
    expect(() => byteOverflow.push("cd")).toThrow(/bounded count or byte buffer/);
    await expect(byteOverflow[Symbol.asyncIterator]().next()).rejects.toThrow(/bounded count or byte buffer/);

    const failed = new __testing.AsyncQueue<number>(1);
    const rejected = failed[Symbol.asyncIterator]().next();
    failed.fail(new Error("bad stream"));
    failed.fail(new Error("ignored"));
    await expect(rejected).rejects.toThrow("bad stream");
    await expect(failed[Symbol.asyncIterator]().next()).rejects.toThrow("bad stream");
  });

  it("validates limits and identifies helper shapes", async () => {
    expect(__testing.positive(undefined, 4)).toBe(4);
    expect(__testing.positive(2, 4)).toBe(2);
    for (const invalid of [0, -1, 1.2, Number.MAX_SAFE_INTEGER + 1]) {
      expect(() => __testing.positive(invalid, 4)).toThrow(RangeError);
    }
    expect(__testing.isAsyncIterable({ async *[Symbol.asyncIterator]() {} })).toBe(true);
    expect(__testing.isAsyncIterable(null)).toBe(false);
    expect(__testing.isAsyncIterable({})).toBe(false);
    expect(__testing.isJsonRecord({})).toBe(true);
    expect(__testing.isJsonRecord([])).toBe(false);
    expect(__testing.isJsonRecord(null)).toBe(false);
    for (const level of ["trace", "debug", "info", "warn", "error"]) {
      expect(__testing.isLogLevel(level)).toBe(true);
    }
    expect(__testing.isLogLevel("fatal")).toBe(false);
    await expect(__testing.settlesWithin(Promise.resolve(), 20)).resolves.toBe(true);
    await expect(__testing.settlesWithin(Promise.reject(new Error("x")), 20)).resolves.toBe(true);
    await expect(__testing.settlesWithin(new Promise(() => {}), 1)).resolves.toBe(false);
  });

  it("validates every supported config-schema keyword shape", () => {
    expect(() => __testing.assertConfigSchema({})).not.toThrow();
    expect(() => __testing.assertConfigSchema({
      type: ["string", "null"], enum: ["x"], required: ["x"],
      additionalProperties: false,
      properties: { x: { type: "string" } }, items: { type: "number" },
      oneOf: [{}], anyOf: [{}], allOf: [{}],
      minLength: 0, maxLength: 10, minItems: 0, maxItems: 10,
      minimum: 0, maximum: 10,
    })).not.toThrow();
    for (const invalid of [
      null, 42, [], { type: [] }, { type: "missing" }, { enum: {} },
      { required: {} }, { required: [1] }, { additionalProperties: {} },
      { properties: [] }, { properties: { x: false } }, { items: false },
      { oneOf: [] }, { anyOf: [false] }, { allOf: "bad" },
      { minLength: -1 }, { maxLength: 1.5 }, { minItems: "1" }, { maxItems: -1 },
      { minimum: "0" }, { maximum: null }, { pattern: 7 },
      { pattern: "^x$" }, { pattern: "^(a+)+$" },
    ]) expect(() => __testing.assertConfigSchema(invalid)).toThrow();
    let deep: Record<string, unknown> = {};
    const root = deep;
    for (let index = 0; index < 258; index += 1) {
      const child: Record<string, unknown> = {};
      deep.items = child;
      deep = child;
    }
    expect(() => __testing.assertConfigSchema(root)).toThrow(/bounded/);
  });

  it("selects explicit, sole, and package-suffix stage ids", () => {
    const base = {
      plugin: { name: "@example/default-stage" },
      contributes: { stages: [{ id: "one" }, { id: "two" }] },
    } as unknown as Manifest;
    expect(__testing.selectStageId(base, "two")).toBe("two");
    expect(__testing.selectStageId(base, undefined)).toBe("default-stage");
    expect(__testing.selectStageId({
      ...base,
      contributes: { ...base.contributes, stages: [{ id: "only" }] },
    }, undefined)).toBe("only");
  });

  it("translates every remote error family without leaking unknown values", () => {
    expect(__testing.translateRemoteError(new RpcRemoteError(-32800, "cancelled")))
      .toBeInstanceOf(CancellationError);
    expect(__testing.translateRemoteError(new RpcRemoteError(-32001, "denied", { capability: "env:X" })))
      .toBeInstanceOf(CapabilityError);
    expect(__testing.translateRemoteError(new RpcRemoteError(-32001, "denied", [])))
      .toBeInstanceOf(CapabilityError);
    const stage = __testing.translateRemoteError(new RpcRemoteError(-32900, "failed", {
      stageErrorCode: "CUSTOM",
      stageName: "x",
      inputPath: "a.md",
      recoverable: true,
      fields: { detail: 1 },
    }));
    expect(stage).toBeInstanceOf(StageError);
    expect(__testing.translateRemoteError(new RpcRemoteError(-32900, "failed", [])))
      .toBeInstanceOf(StageError);
    expect(__testing.translateRemoteError(new RpcRemoteError(-1, "other")))
      .toBeInstanceOf(StageError);
    const local = new Error("local");
    expect(__testing.translateRemoteError(local)).toBe(local);
    expect(__testing.translateRemoteError("string")).toEqual(new Error("string"));
  });

  it("hashes exact schema bytes deterministically", () => {
    expect(__testing.hashBytes(new TextEncoder().encode("{\"a\":1}")))
      .toBe("sha256:015abd7f5cc57a2dd94b7590f04ad8084273905ee33ec5cebeae62276a97f862");
    expect(__testing.hashBytes(new TextEncoder().encode("{ \"a\": 1 }")))
      .not.toBe(__testing.hashBytes(new TextEncoder().encode("{\"a\":1}")));
    expect(__testing.stageImplementationIdentity("blake2b:x", null)).toBe("blake2b:x");
    expect(__testing.stageImplementationIdentity("blake2b:x", "sha256:y"))
      .toMatch(/^sha256:[0-9a-f]{64}$/);
    expect(__testing.estimateRetainedBytes({ value: "abc" }, 1_024)).toBeGreaterThan(3);
    expect(__testing.estimateRetainedBytes([true, null, undefined], 1_024)).toBeGreaterThan(24);
    expect(__testing.estimateRetainedBytes(Uint8Array.of(1, 2, 3), 1_024)).toBeGreaterThan(3);
    expect(() => __testing.estimateRetainedBytes("large", 1)).toThrow(/memory budget/);
    const cyclic: unknown[] = [];
    cyclic.push(cyclic);
    expect(() => __testing.estimateRetainedBytes(cyclic, 1_024)).toThrow(/cyclic/);
    expect(new PluginHostError("PLUGIN_NOT_FOUND", "missing").details).toEqual({});
    expect(__testing.capabilityContext(null, {})).toBeNull();
  });

  it("requires and expands storage-root capability templates", () => {
    const plugin = {
      rootDirectory: "/plugin",
      manifest: {
        plugin: { name: "@example/template" },
        capabilities: {
          required: [{ realm: "filesystem", scope: "read", detail: "$storageRoot" }],
          optional: [{ realm: "env", scope: "OPTIONAL" }],
        },
      },
    } as never;
    expect(() => __testing.resolveDeclaredCapabilities(plugin, {
      storageRoot: undefined,
    } as never)).toThrow(/explicit storageRoot/);
    expect(__testing.resolveDeclaredCapabilities(plugin, {
      storageRoot: "/content",
      cacheDirectory: null,
    } as never)).toEqual({
      required: ["filesystem:read:/content"],
      all: ["filesystem:read:/content", "env:OPTIONAL"],
    });
  });
});
