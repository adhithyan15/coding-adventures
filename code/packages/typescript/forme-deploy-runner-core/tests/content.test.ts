import { createHash } from "node:crypto";
import { describe, expect, it, vi } from "vitest";
import {
  ContentPreflightError,
  createVerifiedContentReader,
  parseDeployManifest,
  preflightDeployContent,
  type ContentStore,
} from "../src/index.js";

const bytes = (value: string): Uint8Array => new TextEncoder().encode(value);
const sha = (value: Uint8Array): string => createHash("sha256").update(value).digest("base64");

function single(body: Uint8Array) {
  return parseDeployManifest({
    version: 1,
    fileCount: 1,
    totalSizeBytes: body.byteLength,
    files: {
      "index.html": {
        outputPath: "index.html",
        contentType: "text/html",
        sizeBytes: body.byteLength,
        sha256: sha(body),
        source: "extra",
      },
    },
  });
}

function store(values: ReadonlyMap<string, Uint8Array>): ContentStore {
  return {
    has: async digest => values.has(digest),
    get: async digest => {
      const value = values.get(digest);
      if (value === undefined) throw new Error("missing");
      return value;
    },
    hashes: async function* () { for (const key of values.keys()) yield key; },
  };
}

describe("preflightDeployContent", () => {
  it("resolves and verifies every current file before publication without retaining bytes", async () => {
    const body = bytes("hello");
    const result = await preflightDeployContent(single(body), store(new Map([[sha(body), body]])));
    expect(result).toEqual({ fileCount: 1, uniqueContentCount: 1, totalSizeBytes: 5 });
  });

  it("reports missing content with a stable code", async () => {
    await expect(preflightDeployContent(single(bytes("hello")), store(new Map())))
      .rejects.toMatchObject<Partial<ContentPreflightError>>({ code: "CONTENT_MISSING", outputPath: "index.html" });
  });

  it("rejects size mismatches even when the store key is trusted", async () => {
    const expected = bytes("hello");
    const wrong = bytes("longer");
    await expect(preflightDeployContent(single(expected), store(new Map([[sha(expected), wrong]]))))
      .rejects.toMatchObject<Partial<ContentPreflightError>>({ code: "CONTENT_SIZE_MISMATCH" });
  });

  it("rejects digest mismatches with equal-length bytes", async () => {
    const expected = bytes("hello");
    const wrong = bytes("jello");
    await expect(preflightDeployContent(single(expected), store(new Map([[sha(expected), wrong]]))))
      .rejects.toMatchObject<Partial<ContentPreflightError>>({ code: "CONTENT_HASH_MISMATCH" });
  });

  it("normalizes content-store availability errors", async () => {
    const body = bytes("hello");
    const broken: ContentStore = {
      has: async () => { throw new Error("offline"); },
      get: async () => body,
      hashes: async function* () { yield sha(body); },
    };
    await expect(preflightDeployContent(single(body), broken))
      .rejects.toMatchObject<Partial<ContentPreflightError>>({ code: "CONTENT_READ_ERROR" });
  });

  it("normalizes content-store read errors", async () => {
    const body = bytes("hello");
    const broken: ContentStore = {
      has: async () => true,
      get: async () => { throw new Error("offline"); },
      hashes: async function* () { yield sha(body); },
    };
    await expect(preflightDeployContent(single(body), broken))
      .rejects.toMatchObject<Partial<ContentPreflightError>>({ code: "CONTENT_READ_ERROR" });
  });

  it("rejects a non-byte store result", async () => {
    const body = bytes("hello");
    const broken = {
      has: async () => true,
      get: async () => "hello",
      hashes: async function* () { yield sha(body); },
    } as unknown as ContentStore;
    await expect(preflightDeployContent(single(body), broken))
      .rejects.toMatchObject<Partial<ContentPreflightError>>({ code: "CONTENT_READ_ERROR" });
  });

  it("rejects proxied typed arrays without consuming an attacker iterator", async () => {
    const good = bytes("good");
    let iterated = false;
    const proxied = new Proxy(good, {
      get: (target, property) => {
        if (property === Symbol.iterator) {
          return function* (): IterableIterator<number> {
            iterated = true;
            yield 101;
            yield 118;
            yield 105;
            yield 108;
          };
        }
        return Reflect.get(target, property, target) as unknown;
      },
    });
    await expect(preflightDeployContent(single(good), store(new Map([[sha(good), proxied]]))))
      .rejects.toMatchObject<Partial<ContentPreflightError>>({ code: "CONTENT_READ_ERROR" });
    expect(iterated).toBe(false);
  });

  it("checks a genuine view's intrinsic byte length before copying", async () => {
    const expected = bytes("x");
    let iterated = false;
    class OversizedBytes extends Uint8Array {
      override *[Symbol.iterator](): ArrayIterator<number> {
        iterated = true;
        yield* super[Symbol.iterator]();
      }
    }
    const oversized = new OversizedBytes(bytes("too large"));
    await expect(preflightDeployContent(single(expected), store(new Map([[sha(expected), oversized]]))))
      .rejects.toMatchObject<Partial<ContentPreflightError>>({ code: "CONTENT_SIZE_MISMATCH" });
    expect(iterated).toBe(false);
  });

  it("normalizes detached typed-array snapshot failures", async () => {
    const detached = new Uint8Array(0);
    const digest = sha(detached);
    structuredClone(detached.buffer, { transfer: [detached.buffer] });
    const reader = createVerifiedContentReader(single(bytes("")), store(new Map([[digest, detached]])));
    await expect(reader.read("index.html"))
      .rejects.toMatchObject<Partial<ContentPreflightError>>({ code: "CONTENT_READ_ERROR" });
  });

  it("snapshots hostile typed-array subclasses before verification and return", async () => {
    const expected = bytes("good");
    class HostileBytes extends Uint8Array {
      override slice(): Uint8Array { return bytes("evil"); }
    }
    const hostile = new HostileBytes(expected);
    const result = await preflightDeployContent(single(expected), store(new Map([[sha(expected), hostile]])));
    expect(result).toEqual({ fileCount: 1, uniqueContentCount: 1, totalSizeBytes: 4 });
  });

  it("reads duplicate content digests only once", async () => {
    const body = bytes("same");
    let reads = 0;
    const digest = sha(body);
    const contentStore: ContentStore = {
      has: async () => true,
      get: async () => { reads += 1; return body; },
      hashes: async function* () { yield digest; },
    };
    const parsed = parseDeployManifest({
      version: 1,
      fileCount: 2,
      totalSizeBytes: 8,
      files: {
        "a.txt": { outputPath: "a.txt", contentType: "text/plain", sizeBytes: 4, sha256: digest, source: "extra" },
        "b.txt": { outputPath: "b.txt", contentType: "text/plain", sizeBytes: 4, sha256: digest, source: "extra" },
      },
    });
    await expect(preflightDeployContent(parsed, contentStore)).resolves.toMatchObject({ uniqueContentCount: 1 });
    expect(reads).toBe(1);
  });

  it("can abort a content store that never settles", async () => {
    const body = bytes("hello");
    const controller = new AbortController();
    const hanging: ContentStore = {
      has: async () => await new Promise<boolean>(() => {}),
      get: async () => body,
      hashes: async function* () { yield sha(body); },
    };
    const pending = preflightDeployContent(single(body), hanging, { signal: controller.signal });
    const remove = vi.spyOn(controller.signal, "removeEventListener");
    controller.abort();
    await expect(pending).rejects.toMatchObject<Partial<ContentPreflightError>>({ code: "CONTENT_ABORTED" });
    expect(remove).toHaveBeenCalledWith("abort", expect.any(Function));
  });

  it("verifies the exact snapshot handed to a publisher after preflight", async () => {
    const good = bytes("good");
    const evil = bytes("evil");
    let reads = 0;
    const changing: ContentStore = {
      has: async () => true,
      get: async () => { reads += 1; return reads === 1 ? good : evil; },
      hashes: async function* () { yield sha(good); },
    };
    const parsed = single(good);
    await expect(preflightDeployContent(parsed, changing)).resolves.toMatchObject({ fileCount: 1 });
    const reader = createVerifiedContentReader(parsed, changing);
    await expect(reader.read("index.html"))
      .rejects.toMatchObject<Partial<ContentPreflightError>>({ code: "CONTENT_HASH_MISMATCH" });
  });

  it("returns a trusted plain Uint8Array snapshot for immediate publication", async () => {
    const good = bytes("good");
    class HostileBytes extends Uint8Array {
      override slice(): Uint8Array { return bytes("evil"); }
    }
    const hostile = new HostileBytes(good);
    const reader = createVerifiedContentReader(single(good), store(new Map([[sha(good), hostile]])));
    const snapshot = await reader.read("index.html");
    expect(snapshot.constructor).toBe(Uint8Array);
    expect([...snapshot]).toEqual([...good]);
  });

  it("parses a publication manifest once for any number of reads", async () => {
    const body = bytes("same");
    const digest = sha(body);
    const files = {
      "a.txt": { outputPath: "a.txt", contentType: "text/plain", sizeBytes: 4, sha256: digest, source: "extra" },
      "b.txt": { outputPath: "b.txt", contentType: "text/plain", sizeBytes: 4, sha256: digest, source: "extra" },
    };
    let fileReads = 0;
    const input = { version: 1, fileCount: 2, totalSizeBytes: 8 } as Record<string, unknown>;
    Object.defineProperty(input, "files", {
      enumerable: true,
      get: () => { fileReads += 1; return files; },
    });
    const reader = createVerifiedContentReader(input, store(new Map([[digest, body]])));
    expect(fileReads).toBe(1);
    await reader.read("a.txt");
    await reader.read("b.txt");
    expect(fileReads).toBe(1);
  });
});
