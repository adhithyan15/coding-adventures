/**
 * forme-types — Kind constants and descriptor tests
 *
 * Verifies the Kinds object exposes a descriptor for every built-in
 * kind (modulo Stream, which is built dynamically), tests the streamOf
 * helper round-trips, and pins the API version to its initial value
 * so any unintended bump is caught in code review.
 */

import { describe, it, expect } from "vitest";
import {
  KERNEL_API_VERSION,
  KINDS,
  Kinds,
  streamOf,
  isStreamDescriptor,
} from "../src/index.js";
import type { KindDescriptor, KindName } from "../src/index.js";

describe("KERNEL_API_VERSION", () => {
  it("starts at 1 — bumping requires explicit migration plan", () => {
    expect(KERNEL_API_VERSION).toBe(1);
  });

  it("is a numeric literal type, not a widening number", () => {
    // Type-level: this assignment is OK only if the version is `1`.
    const v: 1 = KERNEL_API_VERSION;
    expect(v).toBe(1);
  });
});

describe("KINDS", () => {
  it("includes all 13 built-in kind names (12 data kinds + Void)", () => {
    expect(KINDS.length).toBe(13);
  });

  it("includes Void, Stream, and the canonical 11 data kinds", () => {
    const expected = new Set([
      "Void",
      "ContentSource", "ContentNode", "Collection", "Asset",
      "Document", "RenderedPage", "PrintForme",
      "RequestHandler", "SearchIndex", "Feed", "DeployArtifact",
      "Stream",
    ]);
    expect(new Set(KINDS)).toEqual(expected);
  });

  it("is frozen at runtime — push/splice throw rather than corrupt", () => {
    expect(() => {
      // @ts-expect-error — KINDS is readonly at the type level.
      KINDS.push("Bogus");
    }).toThrow(TypeError);
    expect(KINDS.length).toBe(13);
  });
});

describe("Kinds canonical descriptors", () => {
  it("provides a v1.0 descriptor for every built-in non-Stream kind", () => {
    // Stream is built via streamOf(); every other kind has an entry.
    const expectedKeys = KINDS.filter(k => k !== "Stream");
    const actualKeys = Object.keys(Kinds);
    expect(new Set(actualKeys)).toEqual(new Set(expectedKeys));
  });

  it("uses semver-compatible version strings", () => {
    for (const key of Object.keys(Kinds) as Array<keyof typeof Kinds>) {
      expect(Kinds[key].version).toMatch(/^\d+\.\d+(\.\d+)?$/);
    }
  });

  it("has matching .name fields", () => {
    for (const key of Object.keys(Kinds) as Array<keyof typeof Kinds>) {
      expect(Kinds[key].name).toBe(key);
    }
  });
});

describe("streamOf / isStreamDescriptor", () => {
  it("wraps a descriptor with name=Stream and inner=...", () => {
    const wrapped = streamOf(Kinds.ContentSource);
    expect(wrapped).toEqual({
      name: "Stream",
      version: "1.0",
      inner: Kinds.ContentSource,
    });
  });

  it("recognises stream descriptors", () => {
    expect(isStreamDescriptor(streamOf(Kinds.ContentNode))).toBe(true);
    expect(isStreamDescriptor(Kinds.ContentNode)).toBe(false);
    expect(isStreamDescriptor(Kinds.Void)).toBe(false);
  });

  it("can wrap an extension kind name", () => {
    const ext: KindDescriptor = { name: "ext:youtube-embed", version: "1.0" };
    const wrapped = streamOf(ext);
    expect(wrapped.inner).toBe(ext);
    expect(isStreamDescriptor(wrapped)).toBe(true);
  });

  it("nests — streamOf(streamOf(X)) yields a stream of streams", () => {
    const inner = streamOf(Kinds.ContentSource);
    const outer = streamOf(inner);
    expect(outer.inner).toBe(inner);
    expect(outer.inner?.inner).toBe(Kinds.ContentSource);
  });
});

describe("KindName", () => {
  it("accepts built-in names", () => {
    const a: KindName = "ContentSource";
    expect(a).toBe("ContentSource");
  });

  it("accepts ext:-prefixed extension names", () => {
    const a: KindName = "ext:youtube-embed";
    expect(a).toMatch(/^ext:/);
  });

  it("rejects arbitrary strings without ext: prefix at compile time", () => {
    // @ts-expect-error — "MyKind" is neither built-in nor ext:-prefixed.
    const a: KindName = "MyKind";
    expect(a).toBe("MyKind");
  });
});
