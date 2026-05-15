/**
 * forme-types — KindPayload mapped-type tests
 *
 * These tests are entirely compile-time: they verify the `KindPayload`
 * type infers the correct value type from a runtime descriptor.  Each
 * `assertType<E, A>()` call is a no-op at runtime; failures appear as
 * TypeScript errors when running `tsc` (vitest invokes this).
 */

import { describe, it, expect } from "vitest";
import { Kinds, streamOf } from "../src/index.js";
import type {
  Asset, Collection, ContentNode, ContentSource,
  DeployArtifact, Document, Feed,
  KindPayload, PrintForme, RenderedPage, RequestHandler, SearchIndex,
} from "../src/index.js";

// Compile-time equality assertion using a conditional-type trick.
type Equal<A, B> =
  (<T>() => T extends A ? 1 : 2) extends (<T>() => T extends B ? 1 : 2)
    ? true
    : false;

function assertType<_T extends true>(): void {
  /* compile-time only */
}

describe("KindPayload<typeof Kinds.X>", () => {
  it("Void → void", () => {
    assertType<Equal<KindPayload<typeof Kinds.Void>, void>>();
    expect(true).toBe(true);
  });

  it("ContentSource → ContentSource", () => {
    assertType<Equal<KindPayload<typeof Kinds.ContentSource>, ContentSource>>();
    expect(true).toBe(true);
  });

  it("ContentNode → ContentNode", () => {
    assertType<Equal<KindPayload<typeof Kinds.ContentNode>, ContentNode>>();
    expect(true).toBe(true);
  });

  it("Collection → Collection", () => {
    assertType<Equal<KindPayload<typeof Kinds.Collection>, Collection>>();
    expect(true).toBe(true);
  });

  it("Asset → Asset", () => {
    assertType<Equal<KindPayload<typeof Kinds.Asset>, Asset>>();
    expect(true).toBe(true);
  });

  it("Document → Document", () => {
    assertType<Equal<KindPayload<typeof Kinds.Document>, Document>>();
    expect(true).toBe(true);
  });

  it("RenderedPage → RenderedPage", () => {
    assertType<Equal<KindPayload<typeof Kinds.RenderedPage>, RenderedPage>>();
    expect(true).toBe(true);
  });

  it("PrintForme → PrintForme", () => {
    assertType<Equal<KindPayload<typeof Kinds.PrintForme>, PrintForme>>();
    expect(true).toBe(true);
  });

  it("RequestHandler → RequestHandler", () => {
    assertType<Equal<KindPayload<typeof Kinds.RequestHandler>, RequestHandler>>();
    expect(true).toBe(true);
  });

  it("SearchIndex → SearchIndex", () => {
    assertType<Equal<KindPayload<typeof Kinds.SearchIndex>, SearchIndex>>();
    expect(true).toBe(true);
  });

  it("Feed → Feed", () => {
    assertType<Equal<KindPayload<typeof Kinds.Feed>, Feed>>();
    expect(true).toBe(true);
  });

  it("DeployArtifact → DeployArtifact", () => {
    assertType<Equal<KindPayload<typeof Kinds.DeployArtifact>, DeployArtifact>>();
    expect(true).toBe(true);
  });
});

describe("KindPayload<streamOf(...)>", () => {
  it("a stream descriptor maps to AsyncIterable<unknown>", () => {
    // streamOf returns KindDescriptor (its return type); the inner is
    // not preserved at the type level, so payload widens to AsyncIterable<unknown>.
    // This is a deliberate trade-off — see payload.ts header.
    const sd = streamOf(Kinds.ContentSource);
    assertType<Equal<KindPayload<typeof sd>, AsyncIterable<unknown>>>();
    expect(sd.name).toBe("Stream");
  });
});

describe("KindPayload<{ name: ext:..., ... }>", () => {
  it("an unaugmented ext: name maps to unknown", () => {
    const ext = { name: "ext:my-thing" as const, version: "1.0" };
    assertType<Equal<KindPayload<typeof ext>, unknown>>();
    expect(ext.name).toBe("ext:my-thing");
  });
});
