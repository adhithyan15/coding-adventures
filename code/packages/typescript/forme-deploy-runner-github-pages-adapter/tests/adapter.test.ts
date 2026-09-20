import { createHash } from "node:crypto";
import { describe, expect, it } from "vitest";
import type { ContentStore } from "@coding-adventures/forme-deploy-runner-core";
import {
  createGitHubRestBoundary,
  GitHubPagesBoundaryError,
  publishGitHubPagesSite,
  type GitHubPagesBoundary,
  type GitHubPagesBoundaryCall,
  type GitHubPagesPublishOptions,
} from "../src/index.js";

const encoder = new TextEncoder();

describe("publishGitHubPagesSite", () => {
  it("publishes one complete owned prefix with one non-forced ref update", async () => {
    const fixture = site({ "index.html": "new home", "assets/app.css": "body{}" });
    const prior = ownership("landing", "", {
      "index.html": [digest("old home"), "blob-old"],
      "old.css": [digest("stale"), "blob-stale"],
    });
    const boundary = new MockBoundary({ landing: prior, blog: ownership("blog", "blog", {
      "index.html": [digest("blog"), "blob-blog"],
    }) });

    const result = await publishGitHubPagesSite(options(fixture, boundary));

    expect(result).toEqual({
      status: "published",
      commitSha: gitSha("commit-1"),
      attempts: 1,
      fileCount: 2,
      totalSizeBytes: 14,
    });
    expect(boundary.treeCalls).toEqual([{
      owner: "octo",
      repository: "site",
      baseTreeSha: gitSha("tree-base-1"),
      entries: [
        { path: ".forme/deployments/landing.json", mode: "100644", type: "blob", sha: gitSha("blob-3") },
        { path: "assets/app.css", mode: "100644", type: "blob", sha: gitSha("blob-1") },
        { path: "index.html", mode: "100644", type: "blob", sha: gitSha("blob-2") },
        { path: "old.css", mode: "100644", type: "blob", sha: null },
      ],
    }]);
    expect(boundary.updateCalls).toEqual([{
      owner: "octo",
      repository: "site",
      ref: "heads/gh-pages",
      sha: gitSha("commit-1"),
      force: false,
    }]);
  });

  it("preserves another owner's sibling paths and rejects overlapping ownership", async () => {
    const fixture = site({ "index.html": "blog" });
    const boundary = new MockBoundary({ landing: ownership("landing", "", {
      "blog/index.html": [digest("other"), "blob-other"],
    }) });

    await expect(publishGitHubPagesSite(options(fixture, boundary, {
      deploymentOwner: "blog",
      destination: "blog",
    }))).rejects.toMatchObject({ code: "OWNERSHIP_CONFLICT" });
    expect(boundary.updateCalls).toHaveLength(0);
  });

  it("fails closed when stored owners overlap before deleting either path", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary({
      one: ownership("one", "apps", { "demo/index.html": [digest("one"), "one"] }),
      two: ownership("two", "apps/demo", { "index.html": [digest("two"), "two"] }),
    });
    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_INVALID" });
    expect(boundary.updateCalls).toHaveLength(0);
  });

  it("detects non-adjacent lexical prefix collisions", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary({
      one: ownership("one", "", {
        "a": [digest("a"), "a"],
        "a-b": [digest("a-b"), "a-b"],
        "a/c": [digest("a/c"), "a-c"],
      }),
    });
    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_INVALID" });
  });

  it("returns unchanged after trusted blob comparison without creating a commit", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary({
      landing: ownership("landing", "", { "index.html": [digest("home"), gitSha("blob-1")] }),
    });
    const result = await publishGitHubPagesSite(options(fixture, boundary));
    expect(result).toMatchObject({ status: "unchanged", attempts: 1, commitSha: gitSha("base-1") });
    expect(boundary.calls.map(({ method }) => method)).toEqual([
      "getRef", "getCommit", "listOwnershipManifests", "getTargetTree",
      "createBlob", "createBlob",
    ]);
    expect(fixture.reads()).toBe(2);
  });

  it("retries a concurrent ref advance from a fresh base without forcing", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    boundary.updateFailures.push(new GitHubPagesBoundaryError("REF_CONFLICT", "ref advanced", 422));

    const result = await publishGitHubPagesSite(options(fixture, boundary, { retryLimit: 2 }));

    expect(result.attempts).toBe(2);
    expect(boundary.refReads).toBe(2);
    expect(boundary.commitParents).toEqual([[gitSha("base-1")], [gitSha("base-2")]]);
    expect(boundary.updateCalls.map((call) => call.force)).toEqual([false, false]);
  });

  it("fails explicitly when every optimistic ref update conflicts", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    boundary.updateFailures.push(
      new GitHubPagesBoundaryError("REF_CONFLICT", "advanced", 409),
      new GitHubPagesBoundaryError("REF_CONFLICT", "advanced", 422),
    );
    await expect(publishGitHubPagesSite(options(fixture, boundary, { retryLimit: 1 })))
      .rejects.toMatchObject({ code: "RETRY_EXHAUSTED" });
    expect(boundary.updateCalls).toHaveLength(2);
  });

  it("reports an unconfirmed transport outcome as indeterminate", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    boundary.updateFailures.push(new GitHubPagesBoundaryError("NETWORK_ERROR", "lost response", 503));
    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "INDETERMINATE" });
    expect(boundary.refReads).toBe(2);
  });

  it("confirms a publication whose successful ref response was lost", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    boundary.applyBeforeUpdateFailure = true;
    boundary.updateFailures.push(new GitHubPagesBoundaryError("NETWORK_ERROR", "lost response", 503));
    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .resolves.toMatchObject({ status: "published", attempts: 1 });
    expect(boundary.refReads).toBe(2);
  });

  it("confirms a publication whose ref update returned an applied 500", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    boundary.applyBeforeUpdateFailure = true;
    const rest = createGitHubRestBoundary({
      token: "secret",
      fetch: (async () => fragmentedErrorResponse(500)) as typeof globalThis.fetch,
    });
    const updateError = await rest.updateRef({
      owner: "octo", repository: "site", ref: "heads/gh-pages", sha: gitSha("commit"), force: false,
    }).catch((error: unknown) => error);
    expect(updateError).toMatchObject({ status: 500 });
    boundary.updateFailures.push(updateError as Error);
    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .resolves.toMatchObject({ status: "published", attempts: 1 });
    expect(boundary.refReads).toBe(2);
  });

  it("returns published when cancellation races a completed ref update", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    const controller = new AbortController();
    boundary.afterUpdate = () => controller.abort();

    await expect(publishGitHubPagesSite(options(fixture, boundary, { signal: controller.signal })))
      .resolves.toMatchObject({ status: "published", attempts: 1 });
  });

  it("reports cancellation during an unconfirmed ref update as indeterminate", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    const controller = new AbortController();
    boundary.afterUpdate = () => controller.abort();
    boundary.updateFailures.push(new DOMException("aborted", "AbortError"));

    await expect(publishGitHubPagesSite(options(fixture, boundary, { signal: controller.signal })))
      .rejects.toMatchObject({ code: "INDETERMINATE" });
  });

  it("reports cancellation of a never-settling ref update as indeterminate", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    const controller = new AbortController();
    boundary.afterUpdate = () => controller.abort();
    boundary.hangUpdate = true;

    await expect(Promise.race([
      publishGitHubPagesSite(options(fixture, boundary, { signal: controller.signal })),
      new Promise((_, reject) => setTimeout(() => reject(new Error("ref update did not observe cancellation")), 100)),
    ])).rejects.toMatchObject({ code: "INDETERMINATE" });
  });

  it("retries bounded transient failures but never permanent authorization failures", async () => {
    const fixture = site({ "index.html": "home" });
    const transient = new MockBoundary();
    transient.blobFailures.push(new GitHubPagesBoundaryError("SERVICE_UNAVAILABLE", "try later", 503, 1));
    const sleeps: number[] = [];
    await publishGitHubPagesSite(options(fixture, transient, {
      retryLimit: 1,
      sleep: async (milliseconds) => { sleeps.push(milliseconds); },
    }));
    expect(sleeps).toEqual([1]);

    const denied = new MockBoundary();
    denied.blobFailures.push(new GitHubPagesBoundaryError("FORBIDDEN", "denied", 403));
    await expect(publishGitHubPagesSite(options(fixture, denied, { retryLimit: 3 })))
      .rejects.toMatchObject({ code: "FORBIDDEN" });
    expect(denied.calls.filter((call) => call.method === "createBlob")).toHaveLength(1);
  });

  it("retries a definite rate-limited ref rejection without treating it as ambiguous", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    boundary.updateFailures.push(new GitHubPagesBoundaryError("RATE_LIMITED", "slow down", 429, 2));
    const sleeps: number[] = [];

    await expect(publishGitHubPagesSite(options(fixture, boundary, {
      retryLimit: 1,
      sleep: async (milliseconds) => { sleeps.push(milliseconds); },
    }))).resolves.toMatchObject({ status: "published" });
    expect(boundary.updateCalls).toHaveLength(2);
    expect(sleeps).toEqual([2]);
  });

  it("reports cancellation during definite ref-update backoff as aborted", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    const controller = new AbortController();
    boundary.updateFailures.push(new GitHubPagesBoundaryError("RATE_LIMITED", "slow down", 429, 2));

    await expect(publishGitHubPagesSite(options(fixture, boundary, {
      retryLimit: 1,
      signal: controller.signal,
      sleep: async () => { controller.abort(); },
    }))).rejects.toMatchObject({ code: "ABORTED" });
    expect(boundary.updateCalls).toHaveLength(1);
  });

  it("fails closed on malformed ownership state before updating the ref", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    boundary.rawOwnership = [{ path: ".forme/deployments/landing.json", bytes: encoder.encode("{bad") }];

    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_INVALID" });
    expect(boundary.updateCalls).toHaveLength(0);
  });

  it("rejects an overlong ownership key before collision indexing", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    const hostilePath = `${"a/".repeat(4096)}index.html`;
    boundary.rawOwnership = [{
      path: ".forme/deployments/landing.json",
      bytes: encoder.encode(JSON.stringify({
        version: 1,
        owner: "landing",
        destination: "",
        files: { [hostilePath]: { sha256: digest("x"), gitBlobSha: gitSha("x") } },
      })),
    }];

    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_INVALID" });
    expect(boundary.calls.map(({ method }) => method)).not.toContain("getTargetTree");
  });

  it("rejects a desired ownership manifest that would exceed the read bound", async () => {
    const prefix = Array.from({ length: 7 }, () => "p".repeat(250)).join("/");
    const files = Object.fromEntries(Array.from({ length: 10_000 }, (_, index) => [
      `${prefix}/${String(index).padStart(5, "0")}.html`,
      "x",
    ]));
    const fixture = site(files);
    const boundary = new MockBoundary();

    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_INVALID" });
    expect(boundary.calls).toHaveLength(0);
    expect(fixture.reads()).toBe(0);
  });

  it("rejects adding a new owner when the ownership count is already at its limit", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    boundary.rawOwnership = Array.from({ length: 1024 }, (_, index) => {
      const owner = `owner-${String(index).padStart(4, "0")}`;
      return { path: `.forme/deployments/${owner}.json`, bytes: ownership(owner, "", {}) };
    });

    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_INVALID" });
    expect(boundary.calls.map(({ method }) => method)).not.toContain("createBlob");
  });

  it("rejects a projected ownership aggregate that exceeds its read bound", async () => {
    const files = Object.fromEntries(Array.from({ length: 100 }, (_, index) => [`page-${index}.html`, "x"]));
    const fixture = site(files);
    const boundary = new MockBoundary();
    const perOwnerBytes = 16 * 1024 * 1024 - 1000;
    boundary.rawOwnership = Array.from({ length: 4 }, (_, index) => {
      const owner = `owner-${index}`;
      const base = ownership(owner, "", {});
      const padded = new Uint8Array(perOwnerBytes);
      padded.fill(0x20);
      padded.set(base);
      return { path: `.forme/deployments/${owner}.json`, bytes: padded };
    });

    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_INVALID" });
    expect(boundary.calls.map(({ method }) => method)).not.toContain("createBlob");
  });

  it("rejects an owner rebound to another destination", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary({
      landing: ownership("landing", "old-root", { "index.html": [digest("old"), "old"] }),
    });
    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_INVALID" });
  });

  it("rejects target bytes that diverge from recorded ownership", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary({
      landing: ownership("landing", "", { "index.html": [digest("home"), "home"] }),
    });
    boundary.pathBlobOverrides = new Map([["index.html", gitSha("tampered")]]);
    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_INVALID" });
    expect(boundary.updateCalls).toHaveLength(0);
  });

  it("does not trust a target-recorded content digest when its blob differs", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary({
      landing: ownership("landing", "", { "index.html": [digest("home"), "attacker-bytes"] }),
    });
    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .resolves.toMatchObject({ status: "published" });
    expect(boundary.updateCalls).toHaveLength(1);
  });

  it("rejects replacing an unowned target directory with a desired file", async () => {
    const fixture = site({ assets: "new file" });
    const boundary = new MockBoundary();
    boundary.targetEntries = new Map([
      ["assets", { type: "tree", mode: "040000", sha: gitSha("assets-tree") }],
      ["assets/vendor.js", { type: "blob", mode: "100644", sha: gitSha("vendor") }],
    ]);

    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_CONFLICT" });
    expect(boundary.updateCalls).toHaveLength(0);
  });

  it("rejects writing a desired descendant below an unowned target blob", async () => {
    const fixture = site({ "assets/app.js": "new file" });
    const boundary = new MockBoundary();
    boundary.targetEntries = new Map([
      ["assets", { type: "blob", mode: "100644", sha: gitSha("assets-file") }],
    ]);

    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_CONFLICT" });
    expect(boundary.updateCalls).toHaveLength(0);
  });

  it("rejects overwriting an exact unowned blob for a new owner", async () => {
    const fixture = site({ "index.html": "new home" });
    const boundary = new MockBoundary();
    boundary.targetEntries = new Map([
      ["index.html", { type: "blob", mode: "100644", sha: gitSha("unowned-home") }],
    ]);

    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_CONFLICT" });
    expect(boundary.updateCalls).toHaveLength(0);
  });

  it("rejects an existing owner adding a file over an exact unowned blob", async () => {
    const fixture = site({ "index.html": "home", "new.css": "new" });
    const boundary = new MockBoundary({
      landing: ownership("landing", "", { "index.html": [digest("old"), "old-home"] }),
    });
    boundary.targetEntries = new Map([
      ["new.css", { type: "blob", mode: "100644", sha: gitSha("unowned-css") }],
    ]);

    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "OWNERSHIP_CONFLICT" });
    expect(boundary.updateCalls).toHaveLength(0);
  });

  it("rejects a candidate commit whose full target tree is truncated", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    boundary.rejectCandidateTree = true;

    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "MALFORMED_RESPONSE" });
    expect(boundary.updateCalls).toHaveLength(0);
  });

  it("rejects a candidate commit that retained a stale owned path", async () => {
    const fixture = site({ "index.html": "new home" });
    const boundary = new MockBoundary({
      landing: ownership("landing", "", {
        "index.html": [digest("old home"), "old-home"],
        "stale.css": [digest("stale"), "stale"],
      }),
    });
    boundary.retainCandidateDeletes = true;

    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "TARGET_FAILED" });
    expect(boundary.updateCalls).toHaveLength(0);
  });

  it("wraps unclassified boundary failures without leaking into a ref update", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    boundary.blobFailures.push(new Error("socket broke"));
    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toMatchObject({ code: "TARGET_FAILED" });
    expect(boundary.updateCalls).toHaveLength(0);
  });

  it("observes cancellation before publication and leaves the ref unchanged", async () => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    const controller = new AbortController();
    boundary.afterBlob = () => controller.abort();

    await expect(publishGitHubPagesSite(options(fixture, boundary, { signal: controller.signal })))
      .rejects.toMatchObject({ code: "ABORTED" });
    expect(boundary.updateCalls).toHaveLength(0);
  });

  it.each([
    ["bad owner", { deploymentOwner: "../landing" }],
    ["reserved destination", { destination: ".forme/site" }],
    ["combined target path", { destination: Array.from({ length: 8 }, () => "d".repeat(255)).join("/") }],
    ["unsafe ref", { ref: "heads/../main" }],
    ["unsafe repository", { repository: "site/other" }],
    ["unsafe commit message", { commitMessage: "deploy\nnow" }],
    ["excess retry budget", { retryLimit: 11 }],
  ])("rejects %s before content or target access", async (_label, overrides) => {
    const fixture = site({ "index.html": "home" });
    const boundary = new MockBoundary();
    await expect(publishGitHubPagesSite(options(fixture, boundary, overrides)))
      .rejects.toBeInstanceOf(TypeError);
    expect(boundary.calls).toHaveLength(0);
    expect(fixture.reads()).toBe(0);
  });

  it("rejects content mutations before any GitHub write", async () => {
    const fixture = site({ "index.html": "home" });
    fixture.values.set(digest("home"), encoder.encode("evil"));
    const boundary = new MockBoundary();

    await expect(publishGitHubPagesSite(options(fixture, boundary)))
      .rejects.toThrow(/CONTENT_(SIZE|HASH)_MISMATCH/);
    expect(boundary.calls).toHaveLength(0);
  });
});

interface SiteFixture {
  readonly manifest: unknown;
  readonly store: ContentStore;
  readonly values: Map<string, Uint8Array>;
  readonly reads: () => number;
}

function site(files: Readonly<Record<string, string>>): SiteFixture {
  const values = new Map<string, Uint8Array>();
  const entries: Record<string, unknown> = {};
  let totalSizeBytes = 0;
  let readCount = 0;
  for (const [path, text] of Object.entries(files)) {
    const bytes = encoder.encode(text);
    const sha256 = digest(text);
    values.set(sha256, bytes);
    totalSizeBytes += bytes.byteLength;
    entries[path] = {
      outputPath: path,
      contentType: path.endsWith(".html") ? "text/html" : "text/css",
      sizeBytes: bytes.byteLength,
      sha256,
      source: "extra",
    };
  }
  return {
    manifest: { version: 1, fileCount: Object.keys(files).length, totalSizeBytes, files: entries },
    values,
    reads: () => readCount,
    store: {
      has: async (sha256) => values.has(sha256),
      get: async (sha256) => {
        readCount += 1;
        const value = values.get(sha256);
        if (value === undefined) throw new Error("missing");
        return value.slice();
      },
      hashes: async function* () { yield* values.keys(); },
    },
  };
}

function options(
  fixture: SiteFixture,
  boundary: GitHubPagesBoundary,
  overrides: Partial<GitHubPagesPublishOptions> = {},
): GitHubPagesPublishOptions {
  return {
    manifest: fixture.manifest,
    contentStore: fixture.store,
    boundary,
    owner: "octo",
    repository: "site",
    ref: "heads/gh-pages",
    deploymentOwner: "landing",
    destination: "",
    retryLimit: 0,
    ...overrides,
  };
}

function digest(text: string): string {
  return createHash("sha256").update(text).digest("base64");
}

function ownership(
  owner: string,
  destination: string,
  files: Readonly<Record<string, readonly [string, string]>>,
): Uint8Array {
  return encoder.encode(`${JSON.stringify({
    version: 1,
    owner,
    destination,
    files: Object.fromEntries(Object.entries(files).map(([path, [sha256, gitBlobSha]]) => [
      path,
      { sha256, gitBlobSha: /^[0-9a-f]{40}$/.test(gitBlobSha) ? gitBlobSha : gitSha(gitBlobSha) },
    ])),
  }, null, 2)}\n`);
}

function gitSha(label: string): string {
  return createHash("sha1").update(label).digest("hex");
}

function fragmentedErrorResponse(status: number): Response {
  let reads = 0;
  const reader = {
    read: async () => {
      reads += 1;
      return reads <= 100_001
        ? { done: false as const, value: new Uint8Array(0) }
        : { done: true as const, value: undefined };
    },
    cancel: async () => undefined,
    releaseLock: () => undefined,
  };
  return {
    ok: false,
    status,
    headers: new Headers(),
    body: { getReader: () => reader },
  } as unknown as Response;
}

class MockBoundary implements GitHubPagesBoundary {
  readonly calls: GitHubPagesBoundaryCall[] = [];
  readonly treeCalls: Extract<GitHubPagesBoundaryCall, { method: "createTree" }>["input"][] = [];
  readonly updateCalls: Extract<GitHubPagesBoundaryCall, { method: "updateRef" }>["input"][] = [];
  readonly commitParents: string[][] = [];
  readonly blobFailures: Error[] = [];
  readonly updateFailures: Error[] = [];
  readonly createdBlobBytes = new Map<string, Uint8Array>();
  rawOwnership?: readonly { readonly path: string; readonly bytes: Uint8Array }[];
  pathBlobOverrides?: ReadonlyMap<string, string>;
  targetEntries?: ReadonlyMap<string, { readonly type: "blob" | "tree"; readonly mode: string; readonly sha: string }>;
  applyBeforeUpdateFailure = false;
  hangUpdate = false;
  rejectCandidateTree = false;
  retainCandidateDeletes = false;
  afterBlob?: () => void;
  afterUpdate?: () => void;
  refReads = 0;
  private blobCounter = 0;
  private commitCounter = 0;

  constructor(private readonly owners: Readonly<Record<string, Uint8Array>> = {}) {}

  async getRef(input: Extract<GitHubPagesBoundaryCall, { method: "getRef" }>["input"]) {
    this.calls.push({ method: "getRef", input });
    this.refReads += 1;
    return { sha: gitSha(`base-${this.refReads}`) };
  }

  async getCommit(input: Extract<GitHubPagesBoundaryCall, { method: "getCommit" }>["input"]) {
    this.calls.push({ method: "getCommit", input });
    return { treeSha: gitSha(`tree-base-${this.refReads}`) };
  }

  async listOwnershipManifests(input: Extract<GitHubPagesBoundaryCall, { method: "listOwnershipManifests" }>["input"]) {
    this.calls.push({ method: "listOwnershipManifests", input });
    const existing = this.rawOwnership ?? Object.entries(this.owners).map(([owner, bytes]) => ({
      path: `.forme/deployments/${owner}.json`,
      bytes,
    }));
    if (!this.isCandidate(input.commitSha)) return existing;
    const ownershipEntry = this.treeCalls.at(-1)?.entries.find(({ path }) => path.startsWith(".forme/deployments/"));
    const bytes = ownershipEntry?.sha === null || ownershipEntry?.sha === undefined
      ? undefined
      : this.createdBlobBytes.get(ownershipEntry.sha);
    if (ownershipEntry === undefined || bytes === undefined) return existing;
    return [...existing.filter(({ path }) => path !== ownershipEntry.path), { path: ownershipEntry.path, bytes }];
  }

  async getTargetTree(input: Extract<GitHubPagesBoundaryCall, { method: "getTargetTree" }>["input"]) {
    this.calls.push({ method: "getTargetTree", input });
    if (this.rejectCandidateTree && input.commitSha === gitSha("commit-1")) {
      throw new GitHubPagesBoundaryError("MALFORMED_RESPONSE", "candidate tree was truncated");
    }
    const result = new Map<string, { readonly type: "blob" | "tree"; readonly mode: string; readonly sha: string }>();
    const manifests = this.rawOwnership?.map(({ bytes }) => bytes) ?? Object.values(this.owners);
    for (const bytes of manifests) {
      const value = JSON.parse(new TextDecoder().decode(bytes)) as {
        destination: string;
        files: Record<string, { gitBlobSha: string }>;
      };
      for (const [path, file] of Object.entries(value.files)) {
        result.set(value.destination === "" ? path : `${value.destination}/${path}`, {
          type: "blob", mode: "100644", sha: file.gitBlobSha,
        });
      }
    }
    for (const [path, sha] of this.pathBlobOverrides ?? []) result.set(path, { type: "blob", mode: "100644", sha });
    for (const [path, entry] of this.targetEntries ?? []) result.set(path, entry);
    if (this.isCandidate(input.commitSha)) {
      for (const entry of this.treeCalls.at(-1)?.entries ?? []) {
        if (entry.sha === null) {
          if (!this.retainCandidateDeletes) result.delete(entry.path);
        }
        else result.set(entry.path, { type: "blob", mode: entry.mode, sha: entry.sha });
      }
    }
    return result;
  }

  async createBlob(input: Extract<GitHubPagesBoundaryCall, { method: "createBlob" }>["input"]) {
    this.calls.push({ method: "createBlob", input });
    const failure = this.blobFailures.shift();
    if (failure !== undefined) throw failure;
    this.blobCounter += 1;
    const sha = gitSha(`blob-${this.blobCounter}`);
    this.createdBlobBytes.set(sha, Uint8Array.from(Buffer.from(input.contentBase64, "base64")));
    this.afterBlob?.();
    return { sha };
  }

  async createTree(input: Extract<GitHubPagesBoundaryCall, { method: "createTree" }>["input"]) {
    this.calls.push({ method: "createTree", input });
    this.treeCalls.push(input);
    return { sha: gitSha(`tree-new-${this.treeCalls.length}`) };
  }

  async createCommit(input: Extract<GitHubPagesBoundaryCall, { method: "createCommit" }>["input"]) {
    this.calls.push({ method: "createCommit", input });
    this.commitParents.push([...input.parents]);
    this.commitCounter += 1;
    return { sha: gitSha(`commit-${this.commitCounter}`) };
  }

  async updateRef(input: Extract<GitHubPagesBoundaryCall, { method: "updateRef" }>["input"]) {
    this.calls.push({ method: "updateRef", input });
    this.updateCalls.push(input);
    const failure = this.updateFailures.shift();
    this.afterUpdate?.();
    if (this.hangUpdate) await new Promise<void>(() => {});
    if (failure !== undefined) {
      if (this.applyBeforeUpdateFailure) {
        const ownershipSha = [...this.createdBlobBytes.keys()].at(-1);
        const bytes = ownershipSha === undefined ? undefined : this.createdBlobBytes.get(ownershipSha);
        if (bytes !== undefined) {
          this.rawOwnership = [{ path: ".forme/deployments/landing.json", bytes }];
          const value = JSON.parse(new TextDecoder().decode(bytes)) as {
            destination: string;
            files: Record<string, { gitBlobSha: string }>;
          };
          this.pathBlobOverrides = new Map(Object.entries(value.files).map(([path, file]) => [
            value.destination === "" ? path : `${value.destination}/${path}`,
            file.gitBlobSha,
          ]));
        }
      }
      throw failure;
    }
  }

  private isCandidate(commitSha: string): boolean {
    return this.commitCounter > 0 && commitSha === gitSha(`commit-${this.commitCounter}`);
  }
}
