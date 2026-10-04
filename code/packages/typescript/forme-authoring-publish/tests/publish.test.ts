import { createHash } from "node:crypto";
import { describe, expect, it, vi } from "vitest";

import type {
  AuthoringProject,
  AuthoringPublicationCommand,
  AuthoringSession,
} from "@coding-adventures/forme-authoring-core";
import type { ContentStore, DeployManifest } from "@coding-adventures/forme-deploy-runner-core";
import {
  createAuthoringPublisher,
  type AuthoringPublicationBuilder,
  type AuthoringPublishAttempt,
  type AuthoringPublishTarget,
  type PreparedAuthoringPublication,
} from "../src/index.js";

const PROJECT_ID = "01952c0d-7e63-7000-8000-000000000001";
const DOCUMENT_ID = "01952c0d-7e63-7000-8000-000000000002";
const BYTES = new TextEncoder().encode("hello");
const DIGEST = createHash("sha256").update(BYTES).digest("base64");

function project(): AuthoringProject {
  return {
    schemaVersion: 1,
    projectId: PROJECT_ID,
    title: "My site",
    site: { baseUrl: "https://example.com", themeId: "forme-classless" },
    workflow: { lastPublication: null },
    documents: [{
      id: DOCUMENT_ID,
      slug: "hello",
      title: "Hello",
      status: "draft",
      body: { type: "document", children: [] },
    }],
    activeDocumentId: DOCUMENT_ID,
  };
}

function manifest(overrides: Partial<DeployManifest> = {}): DeployManifest {
  return {
    version: 1,
    baseUrl: "https://example.com",
    fileCount: 1,
    totalSizeBytes: BYTES.byteLength,
    files: {
      "index.html": {
        outputPath: "index.html",
        contentType: "text/html; charset=utf-8",
        sizeBytes: BYTES.byteLength,
        sha256: DIGEST,
        source: "page-bundle",
        route: "/",
      },
    },
    ...overrides,
  };
}

function store(bytes = BYTES): ContentStore {
  return Object.freeze({
    async get(digest: string) {
      if (digest !== DIGEST) throw new Error("missing");
      return bytes.slice();
    },
    async has(digest: string) { return digest === DIGEST; },
    async *hashes() { yield DIGEST; },
  });
}

async function collect(values: AsyncIterable<string>): Promise<string[]> {
  const result: string[] = [];
  for await (const value of values) result.push(value);
  return result;
}

class Session implements AuthoringSession {
  project: AuthoringProject = project();
  storageRevision = "revision-1";
  readonly canUndo = false;
  readonly canRedo = false;
  recordError: Error | null = null;
  records: Array<{ readonly expected: string; readonly command: AuthoringPublicationCommand }> = [];

  async dispatch(): Promise<void> { throw new Error("ordinary dispatch must not be used"); }
  async undo(): Promise<void> { throw new Error("unused"); }
  async redo(): Promise<void> { throw new Error("unused"); }
  async dispatchAtRevision(expected: string, command: AuthoringPublicationCommand): Promise<void> {
    if (this.recordError !== null) throw this.recordError;
    if (expected !== this.storageRevision) throw new Error("stale revision");
    this.records.push({ expected, command });
    this.storageRevision = "revision-2";
    this.project = Object.freeze({
      ...this.project,
      workflow: Object.freeze({ lastPublication: Object.freeze(command.publication) }),
      documents: Object.freeze(this.project.documents.map((document) => Object.freeze({ ...document, status: "published" as const }))),
    });
  }
}

function prepared(overrides: Partial<PreparedAuthoringPublication> = {}): PreparedAuthoringPublication {
  return {
    manifest: manifest(),
    contentStore: store(),
    async release() {},
    ...overrides,
  };
}

function target(
  publish: AuthoringPublishTarget["publish"] = async (input) => ({
    outcome: "success",
    manifestSha256: input.manifestSha256,
  }),
): AuthoringPublishTarget {
  return {
    review: {
      targetId: "github-pages",
      label: "GitHub Pages",
      destination: "example/site on gh-pages",
    },
    publish,
  };
}

function coordinator(
  build: AuthoringPublicationBuilder["build"] = async () => prepared(),
  publishTarget = target(),
) {
  return createAuthoringPublisher({
    builder: { build },
    target: publishTarget,
  });
}

describe("reviewed authoring publication", () => {
  it("builds, validates, deploys, retires, and records one exact revision", async () => {
    const session = new Session();
    const release = vi.fn(async () => {});
    const build = vi.fn(async (input) => {
      expect(Object.isFrozen(input)).toBe(true);
      expect(Object.isFrozen(input.project)).toBe(true);
      expect(input.revision).toBe("revision-1");
      return prepared({ release });
    });
    const publish = vi.fn<AuthoringPublishTarget["publish"]>(async (input) => {
      expect(Object.keys(input).sort()).toEqual(["contentStore", "manifest", "manifestSha256"]);
      expect(input.manifest.files["index.html"]!.sha256).toBe(DIGEST);
      expect(input.manifestSha256).toMatch(/^[A-Za-z0-9+/]{43}=$/);
      return { outcome: "success", manifestSha256: input.manifestSha256 };
    });
    const publisher = coordinator(build, target(publish));

    const result = await publisher.publish(session);

    expect(result).toMatchObject({
      outcome: "published",
      revision: "revision-1",
      targetId: "github-pages",
      diagnostics: [],
    });
    expect(build).toHaveBeenCalledOnce();
    expect(publish).toHaveBeenCalledOnce();
    expect(release).toHaveBeenCalledOnce();
    expect(session.records).toHaveLength(1);
    expect(session.records[0]!.command.publication).toEqual({
      authoringRevision: "revision-1",
      manifestSha256: result.manifestSha256,
      targetId: "github-pages",
    });
    expect(session.project.documents[0]!.status).toBe("published");
    expect(publisher.state).toMatchObject({ phase: "published", revision: "revision-1" });
  });

  it("never exposes credentials or authority through the review or build input", async () => {
    const session = new Session();
    let review: unknown;
    let buildInput: unknown;
    const publisher = createAuthoringPublisher({
      builder: { async build(input) { buildInput = input; return prepared(); } },
      target: target(async (input) => {
        review = publisher.target;
        return { outcome: "success", manifestSha256: input.manifestSha256 };
      }),
    });
    await publisher.publish(session);
    expect(review).toEqual({ targetId: "github-pages", label: "GitHub Pages", destination: "example/site on gh-pages" });
    expect(JSON.stringify({ review, buildInput })).not.toMatch(/token|credential|environment|filesystem|network/i);
  });

  it("installs active state before an untrusted builder can re-enter", async () => {
    const session = new Session();
    let publisher!: ReturnType<typeof coordinator>;
    let nested!: AuthoringPublishAttempt;
    publisher = coordinator(async () => {
      nested = await publisher.publish(session);
      return prepared();
    });

    await expect(publisher.publish(session)).resolves.toMatchObject({ outcome: "published" });
    expect(nested).toMatchObject({ outcome: "failed", diagnostics: [{ code: "E_PUBLISH_BUSY" }] });
    expect(session.records).toHaveLength(1);
  });

  it("honors disposal re-entered synchronously by the builder", async () => {
    const session = new Session();
    let publisher!: ReturnType<typeof coordinator>;
    let disposal!: Promise<void>;
    publisher = coordinator(async () => {
      disposal = publisher.dispose();
      return prepared();
    });

    await expect(publisher.publish(session)).resolves.toMatchObject({ outcome: "cancelled" });
    await disposal;
    expect(publisher.state.phase).toBe("disposed");
    expect(session.records).toEqual([]);
  });

  it("retires without deploying when the exact session revision changes during build", async () => {
    const session = new Session();
    const release = vi.fn(async () => {});
    const publish = vi.fn<AuthoringPublishTarget["publish"]>();
    const publisher = coordinator(async () => {
      session.storageRevision = "revision-newer";
      return prepared({ release });
    }, target(publish));
    await expect(publisher.publish(session)).resolves.toMatchObject({
      outcome: "failed",
      diagnostics: [{ code: "E_PUBLISH_STALE" }],
    });
    expect(release).toHaveBeenCalledOnce();
    expect(publish).not.toHaveBeenCalled();
    expect(session.records).toEqual([]);
  });

  it("fails closed and redacts build, manifest, and content errors", async () => {
    const releases = [vi.fn(async () => {}), vi.fn(async () => {})];
    const cases: Array<() => Promise<PreparedAuthoringPublication>> = [
      async () => { throw new Error("builder secret"); },
      async () => prepared({ manifest: { version: 999 }, release: releases[0] }),
      async () => prepared({ contentStore: store(new TextEncoder().encode("wrong")), release: releases[1] }),
    ];
    for (const build of cases) {
      const session = new Session();
      const result = await coordinator(build).publish(session);
      expect(result.outcome).toBe("failed");
      expect(result.diagnostics[0]!.code).toMatch(/^E_PUBLISH_/);
      expect(JSON.stringify(result)).not.toContain("secret");
      expect(session.records).toEqual([]);
    }
    expect(releases[0]).toHaveBeenCalledOnce();
    expect(releases[1]).toHaveBeenCalledOnce();
  });

  it("re-verifies every target content read after preflight and retires the view", async () => {
    let reads = 0;
    const swappingStore: ContentStore = Object.freeze({
      async get() {
        reads += 1;
        return reads === 1 ? BYTES.slice() : new TextEncoder().encode("wrong");
      },
      async has(digest: string) { return digest === DIGEST; },
      async *hashes() { yield DIGEST; },
    });
    const session = new Session();
    const swapped = await coordinator(
      async () => prepared({ contentStore: swappingStore }),
      target(async (input) => {
        await input.contentStore.get(DIGEST);
        return { outcome: "success", manifestSha256: input.manifestSha256 };
      }),
    ).publish(session);
    expect(swapped).toMatchObject({ outcome: "failed", diagnostics: [{ code: "E_PUBLISH_TARGET" }] });
    expect(session.records).toEqual([]);

    let retained!: ContentStore;
    const published = await coordinator(undefined, target(async (input) => {
      retained = input.contentStore;
      expect(await retained.has(DIGEST)).toBe(true);
      expect(await collect(retained.hashes())).toEqual([DIGEST]);
      return { outcome: "success", manifestSha256: input.manifestSha256 };
    })).publish(new Session());
    expect(published.outcome).toBe("published");
    await expect(retained.get(DIGEST)).rejects.toThrow("retired");
  });

  it("poisons when invalid prepared output cannot be retired", async () => {
    const result = await coordinator(async () => prepared({
      manifest: { version: 999 },
      async release() { throw new Error("cleanup secret"); },
    })).publish(new Session());
    expect(result).toMatchObject({ outcome: "indeterminate", diagnostics: [{ code: "E_PUBLISH_CLEANUP" }] });
    expect(JSON.stringify(result)).not.toContain("secret");
  });

  it("allows retry after a known pre-commit target failure", async () => {
    const session = new Session();
    let attempts = 0;
    const publisher = coordinator(undefined, target(async (input) => {
      attempts += 1;
      return attempts === 1
        ? { outcome: "failed", manifestSha256: input.manifestSha256 }
        : { outcome: "success", manifestSha256: input.manifestSha256 };
    }));
    await expect(publisher.publish(session)).resolves.toMatchObject({ outcome: "failed" });
    await expect(publisher.publish(session)).resolves.toMatchObject({ outcome: "published" });
  });

  it("poisons retry after an indeterminate or malformed target acknowledgement", async () => {
    for (const makeResult of [
      (identity: string) => ({ outcome: "indeterminate", manifestSha256: identity }),
      () => ({ outcome: "success", manifestSha256: "wrong" }),
      (identity: string) => ({ outcome: "mystery", manifestSha256: identity }),
    ]) {
      const session = new Session();
      const publisher = coordinator(undefined, target(async (input) => makeResult(input.manifestSha256) as never));
      await expect(publisher.publish(session)).resolves.toMatchObject({ outcome: "indeterminate" });
      await expect(publisher.publish(session)).resolves.toMatchObject({
        outcome: "indeterminate",
        diagnostics: [{ code: "E_PUBLISH_RECONCILE" }],
      });
      expect(session.records).toEqual([]);
    }
  });

  it("treats retirement or metadata failure after deployment as indeterminate", async () => {
    const releaseSession = new Session();
    const releasePublisher = coordinator(async () => prepared({
      async release() { throw new Error("cleanup secret"); },
    }));
    const released = await releasePublisher.publish(releaseSession);
    expect(released).toMatchObject({ outcome: "indeterminate", diagnostics: [{ code: "E_PUBLISH_CLEANUP" }] });
    expect(JSON.stringify(released)).not.toContain("secret");
    expect(releaseSession.records).toEqual([]);

    const recordSession = new Session();
    recordSession.recordError = new Error("storage secret");
    const recorded = await coordinator().publish(recordSession);
    expect(recorded).toMatchObject({ outcome: "indeterminate", diagnostics: [{ code: "E_PUBLISH_RECORD" }] });
    expect(JSON.stringify(recorded)).not.toContain("secret");
    expect(recordSession.project.documents[0]!.status).toBe("draft");
  });

  it("handles target rejection, cancellation, stale cleanup failure, and malformed results", async () => {
    const rejected = await coordinator(undefined, target(async () => { throw new Error("target secret"); })).publish(new Session());
    expect(rejected).toMatchObject({ outcome: "failed", diagnostics: [{ code: "E_PUBLISH_TARGET" }] });
    expect(JSON.stringify(rejected)).not.toContain("secret");

    const rejectedCleanup = await coordinator(
      async () => prepared({ async release() { throw new Error("cleanup"); } }),
      target(async () => { throw new Error("target"); }),
    ).publish(new Session());
    expect(rejectedCleanup).toMatchObject({ outcome: "indeterminate", diagnostics: [{ code: "E_PUBLISH_CLEANUP" }] });

    const controller = new AbortController();
    controller.abort();
    await expect(coordinator().publish(new Session(), controller.signal)).resolves.toMatchObject({
      outcome: "cancelled",
      diagnostics: [{ code: "E_PUBLISH_CANCELLED" }],
    });

    const stale = new Session();
    const stalePublisher = coordinator(async () => {
      stale.storageRevision = "revision-newer";
      return prepared({ async release() { throw new Error("cleanup"); } });
    });
    await expect(stalePublisher.publish(stale)).resolves.toMatchObject({
      outcome: "indeterminate",
      diagnostics: [{ code: "E_PUBLISH_CLEANUP" }],
    });

    const malformedPublisher = coordinator(undefined, target(async () => null as never));
    await expect(malformedPublisher.publish(new Session())).resolves.toMatchObject({ outcome: "indeterminate" });

    const malformedCleanup = coordinator(
      async () => prepared({ async release() { throw new Error("cleanup"); } }),
      target(async () => null as never),
    );
    await expect(malformedCleanup.publish(new Session())).resolves.toMatchObject({
      outcome: "indeterminate",
      diagnostics: [{ code: "E_PUBLISH_CLEANUP" }],
    });

    const invalidRevision = new Session();
    const invalidRevisionPublisher = coordinator(async () => {
      invalidRevision.storageRevision = " bad";
      return prepared({ async release() { throw new Error("cleanup"); } });
    });
    await expect(invalidRevisionPublisher.publish(invalidRevision)).resolves.toMatchObject({
      outcome: "indeterminate",
      diagnostics: [{ code: "E_PUBLISH_CLEANUP" }],
    });
  });

  it("rejects overlapping explicit actions and disposes without a hidden retry", async () => {
    const session = new Session();
    let finish!: (value: PreparedAuthoringPublication) => void;
    const waiting = new Promise<PreparedAuthoringPublication>((resolve) => { finish = resolve; });
    const publisher = coordinator(async () => await waiting);
    const first = publisher.publish(session);
    await expect(publisher.publish(session)).resolves.toMatchObject({
      outcome: "failed",
      diagnostics: [{ code: "E_PUBLISH_BUSY" }],
    });
    finish(prepared());
    await expect(first).resolves.toMatchObject({ outcome: "published" });
    await publisher.dispose();
    await publisher.dispose();
    await expect(publisher.publish(session)).rejects.toThrow("disposed");
  });

  it("aborts active work during disposal and reports cancellation", async () => {
    const session = new Session();
    const publisher = coordinator(async (_input, signal) => {
      await new Promise<void>((resolve, reject) => {
        signal.addEventListener("abort", () => reject(new Error("aborted")), { once: true });
      });
      return prepared();
    });
    const action = publisher.publish(session);
    await publisher.dispose();
    await expect(action).resolves.toMatchObject({ outcome: "cancelled" });
    expect(publisher.state.phase).toBe("disposed");
  });

  it("forwards a later caller cancellation to active build work", async () => {
    const controller = new AbortController();
    const publisher = coordinator(async (_input, signal) => {
      await new Promise<void>((_resolve, reject) => {
        signal.addEventListener("abort", () => reject(new Error("aborted")), { once: true });
      });
      return prepared();
    });
    const action = publisher.publish(new Session(), controller.signal);
    controller.abort();
    await expect(action).resolves.toMatchObject({ outcome: "cancelled" });
  });

  it("rejects unsafe construction and hostile session or prepared shapes generically", async () => {
    const invalidOptions: unknown[] = [
      null,
      {},
      { builder: null, target: target() },
      { builder: {}, target: target() },
      { builder: { build: 1 }, target: target() },
      { builder: { build: async () => prepared() }, target: { ...target(), extra: true } },
      { builder: { build: async () => prepared() }, target: { ...target(), review: { ...target().review, targetId: "UPPER" } } },
      { builder: { build: async () => prepared() }, target: { ...target(), review: { ...target().review, label: " bad" } } },
      { builder: { build: async () => prepared() }, target: { ...target(), review: { ...target().review, destination: "bad\u202e" } } },
    ];
    for (const options of invalidOptions) {
      expect(() => createAuthoringPublisher(options as never)).toThrow();
    }

    const accessor = {} as Record<string, unknown>;
    Object.defineProperty(accessor, "builder", { enumerable: true, get() { throw new Error("secret"); } });
    Object.defineProperty(accessor, "target", { enumerable: true, value: target() });
    expect(() => createAuthoringPublisher(accessor as never)).toThrow();

    const symbolOptions = { builder: { build: async () => prepared() }, target: target(), [Symbol("secret")]: true };
    expect(() => createAuthoringPublisher(symbolOptions as never)).toThrow();

    const publisher = coordinator();
    await expect(publisher.publish(null as never)).rejects.toThrow("inspected safely");
    await expect(publisher.publish({ project: project(), storageRevision: "bad\u202e" } as never)).rejects.toThrow("inspected safely");
    await expect(coordinator(async () => ({ manifest: manifest(), contentStore: store() } as never)).publish(new Session()))
      .resolves.toMatchObject({ outcome: "failed", diagnostics: [{ code: "E_PUBLISH_BUILD" }] });
    await expect(coordinator(async () => ({ ...prepared(), release: 1 } as never)).publish(new Session()))
      .resolves.toMatchObject({ outcome: "failed" });
    const extraRelease = vi.fn(async () => {});
    await expect(coordinator(async () => ({ ...prepared(), release: extraRelease, extra: true } as never)).publish(new Session()))
      .resolves.toMatchObject({ outcome: "failed" });
    expect(extraRelease).toHaveBeenCalledOnce();
    await expect(coordinator(async () => prepared({ contentStore: {} as never })).publish(new Session()))
      .resolves.toMatchObject({ outcome: "failed" });

    const hiddenReview = target();
    Object.defineProperty(hiddenReview.review, "token", { value: "secret" });
    expect(() => createAuthoringPublisher({ builder: { async build() { return prepared(); } }, target: hiddenReview })).toThrow(
      "could not be inspected safely",
    );

    const hostile = new Proxy({}, { getPrototypeOf() { throw new Error("secret"); } });
    expect(() => createAuthoringPublisher(hostile as never)).toThrow("could not be inspected safely");
  });

  it("returns immutable closed attempts and state", async () => {
    const session = new Session();
    const publisher = coordinator();
    const attempt: AuthoringPublishAttempt = await publisher.publish(session);
    expect(Object.isFrozen(attempt)).toBe(true);
    expect(Object.isFrozen(attempt.diagnostics)).toBe(true);
    expect(Object.isFrozen(publisher.state)).toBe(true);
    expect(Object.isFrozen(publisher.target)).toBe(true);
  });
});
