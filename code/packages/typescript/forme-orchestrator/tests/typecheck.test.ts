/**
 * forme-orchestrator — typecheck tests
 */

import { describe, it, expect } from "vitest";
import { Kinds, streamOf } from "@coding-adventures/forme-types";
import type {
  KindDescriptor,
  KindName,
  KindRegistry,
  RegisteredKind,
} from "@coding-adventures/forme-types";
import { areKindsCompatible } from "../src/index.js";

const blogPost = { name: "ext:blog-post", version: "1.1" } as const;
const featuredPost = { name: "ext:featured-post", version: "1.2" } as const;

function extensionRegistry(
  featured: KindDescriptor = featuredPost,
): KindRegistry {
  const registrations = new Map<KindName, RegisteredKind>([
    ["ext:blog-post", { descriptor: blogPost, parent: Kinds.ContentNode }],
    ["ext:featured-post", { descriptor: featured, parent: blogPost }],
  ]);
  return { lookup: name => registrations.get(name) };
}

describe("areKindsCompatible — basic", () => {
  it("identical descriptors match", () => {
    expect(areKindsCompatible(Kinds.ContentSource, Kinds.ContentSource)).toBe(true);
  });

  it("name mismatch fails", () => {
    expect(areKindsCompatible(Kinds.ContentSource, Kinds.ContentNode)).toBe(false);
  });
});

describe("areKindsCompatible — stream wrapping", () => {
  it("Stream<X> can feed single-X consumer (executor iterates)", () => {
    expect(areKindsCompatible(streamOf(Kinds.ContentSource), Kinds.ContentSource)).toBe(true);
  });

  it("single-X cannot feed Stream<X> consumer", () => {
    expect(areKindsCompatible(Kinds.ContentSource, streamOf(Kinds.ContentSource))).toBe(false);
  });

  it("Stream<X> → Stream<X> matches", () => {
    expect(areKindsCompatible(streamOf(Kinds.ContentNode), streamOf(Kinds.ContentNode))).toBe(true);
  });

  it("Stream<X> → Stream<Y> mismatches when inner kinds differ", () => {
    expect(areKindsCompatible(streamOf(Kinds.ContentSource), streamOf(Kinds.ContentNode))).toBe(false);
  });

  it("Stream without inner is treated as incompatible", () => {
    const bogus = { name: "Stream" as const, version: "1.0" };
    expect(areKindsCompatible(bogus, Kinds.ContentSource)).toBe(false);
    expect(areKindsCompatible(streamOf(Kinds.ContentSource), bogus)).toBe(false);
  });
});

describe("areKindsCompatible — version compatibility", () => {
  it("refuses the legacy RenderedPage v1.1 contract at a v2 edge", () => {
    expect(areKindsCompatible(
      { name: "RenderedPage", version: "1.1" },
      Kinds.RenderedPage,
    )).toBe(false);
    expect(areKindsCompatible(
      Kinds.RenderedPage,
      { name: "RenderedPage", version: "1.1" },
    )).toBe(false);
  });

  it("major mismatch fails", () => {
    expect(areKindsCompatible(
      { name: "ContentSource", version: "2.0" },
      { name: "ContentSource", version: "1.0" },
    )).toBe(false);
  });

  it("producer minor ≥ consumer minor passes", () => {
    expect(areKindsCompatible(
      { name: "ContentSource", version: "1.2" },
      { name: "ContentSource", version: "1.1" },
    )).toBe(true);
  });

  it("producer minor < consumer minor fails", () => {
    expect(areKindsCompatible(
      { name: "ContentSource", version: "1.1" },
      { name: "ContentSource", version: "1.2" },
    )).toBe(false);
  });

  it("non-semver strings require exact match", () => {
    expect(areKindsCompatible(
      { name: "ContentSource", version: "v1" },
      { name: "ContentSource", version: "v1" },
    )).toBe(true);
    expect(areKindsCompatible(
      { name: "ContentSource", version: "v1" },
      { name: "ContentSource", version: "v2" },
    )).toBe(false);
  });
});

describe("areKindsCompatible — discriminants", () => {
  it("consumer with no discriminant accepts anything", () => {
    expect(areKindsCompatible(
      { name: "Feed", version: "1.0", discriminant: "rss" },
      { name: "Feed", version: "1.0" },
    )).toBe(true);
  });

  it("matching discriminants pass", () => {
    expect(areKindsCompatible(
      { name: "Feed", version: "1.0", discriminant: "rss" },
      { name: "Feed", version: "1.0", discriminant: "rss" },
    )).toBe(true);
  });

  it("differing discriminants fail", () => {
    expect(areKindsCompatible(
      { name: "Feed", version: "1.0", discriminant: "rss" },
      { name: "Feed", version: "1.0", discriminant: "atom" },
    )).toBe(false);
  });

  it("producer missing discriminant fails when consumer demands one", () => {
    expect(areKindsCompatible(
      { name: "Feed", version: "1.0" },
      { name: "Feed", version: "1.0", discriminant: "atom" },
    )).toBe(false);
  });
});

describe("areKindsCompatible — registered extension subtypes", () => {
  it("allows direct and transitive producer widening", () => {
    const registry = extensionRegistry();
    expect(areKindsCompatible(blogPost, Kinds.ContentNode, registry)).toBe(true);
    expect(areKindsCompatible(featuredPost, Kinds.ContentNode, registry)).toBe(true);
    expect(areKindsCompatible(featuredPost, blogPost, registry)).toBe(true);
  });

  it("never narrows a generic producer into a specialized consumer", () => {
    const registry = extensionRegistry();
    expect(areKindsCompatible(Kinds.ContentNode, blogPost, registry)).toBe(false);
    expect(areKindsCompatible(blogPost, featuredPost, registry)).toBe(false);
  });

  it("preserves stream covariance and stream shape", () => {
    const registry = extensionRegistry();
    expect(areKindsCompatible(streamOf(featuredPost), streamOf(Kinds.ContentNode), registry))
      .toBe(true);
    expect(areKindsCompatible(streamOf(Kinds.ContentNode), streamOf(featuredPost), registry))
      .toBe(false);
    expect(areKindsCompatible(featuredPost, streamOf(Kinds.ContentNode), registry))
      .toBe(false);
  });

  it("rejects unregistered extensions and descriptor-version drift", () => {
    const registry = extensionRegistry();
    expect(areKindsCompatible(
      { name: "ext:unknown", version: "1.0" },
      Kinds.ContentNode,
      registry,
    )).toBe(false);
    expect(areKindsCompatible(
      { name: "ext:blog-post", version: "1.2" },
      Kinds.ContentNode,
      registry,
    )).toBe(false);
  });

  it("checks version compatibility across every ancestry edge", () => {
    const incompatibleFeatured = { name: "ext:featured-post", version: "2.0" } as const;
    expect(areKindsCompatible(
      incompatibleFeatured,
      Kinds.ContentNode,
      extensionRegistry(incompatibleFeatured),
    )).toBe(false);
  });
});
