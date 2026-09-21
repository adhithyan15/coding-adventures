import { describe, expect, it } from "vitest";
import { createGitHubRestBoundary, GitHubPagesBoundaryError } from "../src/index.js";

describe("createGitHubRestBoundary", () => {
  it("authenticates and encodes the selected repository and ref", async () => {
    const mock = fetchQueue(json({ object: { sha: hex("a") } }));
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: mock.fetch });

    await expect(boundary.getRef({ owner: "octo", repository: "site", ref: "heads/gh-pages" }))
      .resolves.toEqual({ sha: hex("a") });
    expect(mock.calls[0]?.url).toBe("https://api.github.com/repos/octo/site/git/ref/heads%2Fgh-pages");
    expect(mock.calls[0]?.init.headers).toMatchObject({
      Authorization: "Bearer secret",
      "X-GitHub-Api-Version": "2026-03-10",
    });
    expect(mock.calls[0]?.init.redirect).toBe("error");
  });

  it("walks only the reserved ownership tree and decodes manifests", async () => {
    const bytes = new TextEncoder().encode('{"version":1}');
    const mock = fetchQueue(
      json({ tree: { sha: hex("root") } }),
      tree([{ path: ".forme", type: "tree", sha: hex("forme") }]),
      tree([{ path: "deployments", type: "tree", sha: hex("deployments") }]),
      tree([{ path: "landing.json", type: "blob", sha: hex("manifest") }]),
      json({ encoding: "base64", content: Buffer.from(bytes).toString("base64") }),
    );
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: mock.fetch });

    await expect(boundary.listOwnershipManifests({
      owner: "octo",
      repository: "site",
      commitSha: hex("commit"),
    })).resolves.toEqual([{ path: ".forme/deployments/landing.json", bytes }]);
    expect(mock.calls.map(({ url }) => url)).toEqual([
      `https://api.github.com/repos/octo/site/git/commits/${hex("commit")}`,
      `https://api.github.com/repos/octo/site/git/trees/${hex("root")}`,
      `https://api.github.com/repos/octo/site/git/trees/${hex("forme")}`,
      `https://api.github.com/repos/octo/site/git/trees/${hex("deployments")}`,
      `https://api.github.com/repos/octo/site/git/blobs/${hex("manifest")}`,
    ]);
  });

  it("returns no ownership manifests when the reserved tree is absent", async () => {
    const mock = fetchQueue(
      json({ tree: { sha: hex("root") } }),
      tree([{ path: "index.html", type: "blob", sha: hex("index") }]),
    );
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: mock.fetch });
    await expect(boundary.listOwnershipManifests({
      owner: "octo", repository: "site", commitSha: hex("commit"),
    })).resolves.toEqual([]);
  });

  it("verifies owned paths against regular blobs in a bounded recursive tree", async () => {
    const mock = fetchQueue(
      json({ tree: { sha: hex("root") } }),
      tree([
        { path: "index.html", type: "blob", sha: hex("index") },
        { path: "blog", type: "tree", sha: hex("blog-tree") },
        { path: "blog/index.html", type: "blob", sha: hex("blog") },
      ]),
    );
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: mock.fetch });
    await expect(boundary.getTargetTree({
      owner: "octo",
      repository: "site",
      commitSha: hex("commit"),
    })).resolves.toEqual(new Map([
      ["index.html", { type: "blob", mode: "100644", sha: hex("index") }],
      ["blog", { type: "tree", mode: "040000", sha: hex("blog-tree") }],
      ["blog/index.html", { type: "blob", mode: "100644", sha: hex("blog") }],
    ]));
    expect(mock.calls[1]?.url.endsWith(`git/trees/${hex("root")}?recursive=1`)).toBe(true);
  });

  it("creates Git objects and always performs a non-forced ref update", async () => {
    const mock = fetchQueue(
      json({ sha: hex("blob") }, 201),
      json({ sha: hex("tree") }, 201),
      json({ sha: hex("commit") }, 201),
      json({ object: { sha: hex("commit") } }),
    );
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: mock.fetch });
    const repository = { owner: "octo", repository: "site" };

    await boundary.createBlob({ ...repository, contentBase64: "aGk=" });
    await boundary.createTree({
      ...repository,
      baseTreeSha: hex("base"),
      entries: [{ path: "index.html", mode: "100644", type: "blob", sha: hex("blob") }],
    });
    await boundary.createCommit({
      ...repository,
      message: "forme deploy: landing",
      treeSha: hex("tree"),
      parents: [hex("parent")],
    });
    await boundary.updateRef({ ...repository, ref: "heads/gh-pages", sha: hex("commit"), force: false });

    expect(mock.calls.map(({ init }) => init.method)).toEqual(["POST", "POST", "POST", "PATCH"]);
    expect(JSON.parse(String(mock.calls[3]?.init.body))).toEqual({ sha: hex("commit"), force: false });
  });

  it("accepts a successful ref update without parsing or waiting for its body", async () => {
    let cancelled = false;
    const stalled = new Response(new ReadableStream<Uint8Array>({
      pull: async () => await new Promise<void>(() => {}),
      cancel: () => { cancelled = true; },
    }), { status: 200, headers: { "content-type": "application/json" } });
    const malformed = new Response("{not json", {
      status: 200,
      headers: { "content-type": "application/json" },
    });
    const mock = fetchQueue(malformed, stalled);
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: mock.fetch });
    const input = { owner: "octo", repository: "site", ref: "heads/gh-pages", sha: hex("commit"), force: false as const };

    await expect(boundary.updateRef(input)).resolves.toBeUndefined();
    await expect(Promise.race([
      boundary.updateRef(input),
      new Promise((_, reject) => setTimeout(() => reject(new Error("update body was awaited")), 50)),
    ])).resolves.toBeUndefined();
    expect(cancelled).toBe(true);
  });

  it("classifies only ref-update 422 responses as optimistic conflicts", async () => {
    const update = fetchQueue(json({ message: "not a fast forward" }, 422));
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: update.fetch });
    await expect(boundary.updateRef({
      owner: "octo", repository: "site", ref: "heads/gh-pages", sha: hex("commit"), force: false,
    })).rejects.toMatchObject({ code: "REF_CONFLICT", status: 422 });

    const treeFailure = fetchQueue(json({ message: "bad tree" }, 422));
    const second = createGitHubRestBoundary({ token: "secret", fetch: treeFailure.fetch });
    await expect(second.createTree({
      owner: "octo", repository: "site", baseTreeSha: hex("base"), entries: [],
    })).rejects.toMatchObject({ code: "UNPROCESSABLE", status: 422 });

    const invalidUpdate = fetchQueue(json({ message: "Validation Failed" }, 422));
    const third = createGitHubRestBoundary({ token: "secret", fetch: invalidUpdate.fetch });
    await expect(third.updateRef({
      owner: "octo", repository: "site", ref: "heads/gh-pages", sha: hex("commit"), force: false,
    })).rejects.toMatchObject({ code: "UNPROCESSABLE", status: 422 });
  });

  it("maps retry-after rate limits to the adapter's transient status", async () => {
    const mock = fetchQueue(new Response(JSON.stringify({ message: "slow down" }), {
      status: 403,
      headers: { "content-type": "application/json", "retry-after": "2" },
    }));
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: mock.fetch });
    await expect(boundary.getRef({ owner: "octo", repository: "site", ref: "heads/gh-pages" }))
      .rejects.toMatchObject({ code: "RATE_LIMITED", status: 429, retryAfterMs: 2000 });
  });

  it("keeps authorization failures permanent and diagnostics credential-free", async () => {
    const mock = fetchQueue(json({ message: "Bad credentials" }, 401));
    const boundary = createGitHubRestBoundary({ token: "top-secret", fetch: mock.fetch });
    const error = await boundary.getRef({ owner: "octo", repository: "site", ref: "heads/gh-pages" })
      .catch((caught: unknown) => caught);
    expect(error).toMatchObject({ code: "UNAUTHORIZED", status: 401 });
    expect(String(error)).not.toContain("top-secret");
  });

  it("redacts credentials and terminal controls from response diagnostics", async () => {
    const mock = fetchQueue(json({ message: "denied top-secret\n\u001b[31m forged" }, 403));
    const boundary = createGitHubRestBoundary({ token: "top-secret", fetch: mock.fetch });
    const error = await boundary.getRef({ owner: "octo", repository: "site", ref: "heads/gh-pages" })
      .catch((caught: unknown) => caught);
    expect(String(error)).not.toContain("top-secret");
    expect(String(error)).not.toMatch(/[\u0000-\u001f\u007f-\u009f]/);
    expect(String(error)).toContain("[REDACTED]");
  });

  it("normalizes transport failures as retryable without exposing fetch details", async () => {
    const fetch = (async () => { throw new TypeError("request to https://secret.invalid failed"); }) as typeof globalThis.fetch;
    const boundary = createGitHubRestBoundary({ token: "top-secret", fetch });
    const error = await boundary.getRef({ owner: "octo", repository: "site", ref: "heads/gh-pages" })
      .catch((caught: unknown) => caught);
    expect(error).toMatchObject({ code: "NETWORK_ERROR", status: 503 });
    expect(String(error)).not.toContain("secret.invalid");
    expect(String(error)).not.toContain("top-secret");
  });

  it("normalizes response-stream failures without exposing transport details", async () => {
    const stream = new ReadableStream<Uint8Array>({
      start(controller) {
        controller.enqueue(new TextEncoder().encode('{"object":'));
        controller.error(new Error("socket from https://secret.invalid failed"));
      },
    });
    const mock = fetchQueue(new Response(stream, { status: 200 }));
    const boundary = createGitHubRestBoundary({ token: "top-secret", fetch: mock.fetch });
    const error = await boundary.getRef({ owner: "octo", repository: "site", ref: "heads/gh-pages" })
      .catch((caught: unknown) => caught);
    expect(error).toMatchObject({ code: "NETWORK_ERROR", status: 503 });
    expect(String(error)).not.toContain("secret.invalid");
    expect(String(error)).not.toContain("top-secret");
  });

  it("starts no follow-on composite request after cancellation", async () => {
    const controller = new AbortController();
    const calls: string[] = [];
    const fetch = (async (input: string | URL | Request) => {
      calls.push(String(input));
      controller.abort();
      return json({ tree: { sha: hex("root") } });
    }) as typeof globalThis.fetch;
    const boundary = createGitHubRestBoundary({ token: "secret", fetch });

    await expect(boundary.listOwnershipManifests({
      owner: "octo", repository: "site", commitSha: hex("commit"), signal: controller.signal,
    })).rejects.toMatchObject({ name: "AbortError" });
    expect(calls).toHaveLength(1);
  });

  it.each([
    ["token", { token: "bad\ncredential" }],
    ["API version", { token: "secret", apiVersion: "latest" }],
  ])("rejects an unsafe %s before calling fetch", (_label, options) => {
    const mock = fetchQueue();
    expect(() => createGitHubRestBoundary({ ...options, fetch: mock.fetch })).toThrow(TypeError);
    expect(mock.calls).toHaveLength(0);
  });

  it("rejects truncated and malformed GitHub responses", async () => {
    const truncated = fetchQueue(
      json({ tree: { sha: hex("root") } }),
      json({ truncated: true, tree: [] }),
    );
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: truncated.fetch });
    await expect(boundary.listOwnershipManifests({
      owner: "octo", repository: "site", commitSha: hex("commit"),
    })).rejects.toBeInstanceOf(GitHubPagesBoundaryError);
  });

  it.each([undefined, null, "false"])("rejects a tree with malformed truncated=%s", async (truncatedValue) => {
    const payload: Record<string, unknown> = { tree: [] };
    if (truncatedValue !== undefined) payload.truncated = truncatedValue;
    const mock = fetchQueue(
      json({ tree: { sha: hex("root") } }),
      json(payload),
    );
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: mock.fetch });
    await expect(boundary.getTargetTree({
      owner: "octo", repository: "site", commitSha: hex("commit"),
    })).rejects.toMatchObject({ code: "MALFORMED_RESPONSE" });
  });

  it("rejects an oversized response before reading its body", async () => {
    const mock = fetchQueue(new Response("{}", {
      status: 200,
      headers: { "content-length": String(24 * 1024 * 1024 + 1) },
    }));
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: mock.fetch });
    await expect(boundary.getRef({ owner: "octo", repository: "site", ref: "heads/gh-pages" }))
      .rejects.toMatchObject({ code: "MALFORMED_RESPONSE" });
  });

  it("does not await a nonsettling stream cancel after reaching a response bound", async () => {
    let cancelled = false;
    const stream = new ReadableStream<Uint8Array>({
      pull(controller) { controller.enqueue(new Uint8Array(5000)); },
      cancel() { cancelled = true; return new Promise<void>(() => {}); },
    });
    const mock = fetchQueue(new Response(stream, { status: 400 }));
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: mock.fetch });
    await expect(Promise.race([
      boundary.getRef({ owner: "octo", repository: "site", ref: "heads/gh-pages" }),
      new Promise((_, reject) => setTimeout(() => reject(new Error("stream cancel was awaited")), 100)),
    ])).rejects.toMatchObject({ code: "HTTP_ERROR", status: 400 });
    expect(cancelled).toBe(true);
  });

  it("rejects non-base64 ownership blobs", async () => {
    const mock = fetchQueue(
      json({ tree: { sha: hex("root") } }),
      tree([{ path: ".forme", type: "tree", sha: hex("forme") }]),
      tree([{ path: "deployments", type: "tree", sha: hex("deployments") }]),
      tree([{ path: "landing.json", type: "blob", sha: hex("manifest") }]),
      json({ encoding: "utf-8", content: "{}" }),
    );
    const boundary = createGitHubRestBoundary({
      token: "secret",
      fetch: mock.fetch,
    });
    await expect(boundary.listOwnershipManifests({
      owner: "octo", repository: "site", commitSha: hex("commit"),
    })).rejects.toMatchObject({ code: "MALFORMED_RESPONSE" });
    expect(mock.calls[0]?.url.startsWith("https://api.github.com/repos/")).toBe(true);
  });

  it("bounds ownership manifests before issuing blob requests", async () => {
    const ownershipEntries = Array.from({ length: 1025 }, (_, index) => ({
      path: `owner-${String(index).padStart(4, "0")}.json`,
      type: "blob" as const,
      sha: hex(`manifest-${index}`),
    }));
    const mock = fetchQueue(
      json({ tree: { sha: hex("root") } }),
      tree([{ path: ".forme", type: "tree", sha: hex("forme") }]),
      tree([{ path: "deployments", type: "tree", sha: hex("deployments") }]),
      tree(ownershipEntries),
    );
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: mock.fetch });
    await expect(boundary.listOwnershipManifests({
      owner: "octo", repository: "site", commitSha: hex("commit"),
    })).rejects.toMatchObject({ code: "MALFORMED_RESPONSE" });
    expect(mock.calls).toHaveLength(4);
  });

  it.each([
    ["symlink", { path: "landing.json", type: "blob", mode: "120000", sha: hex("manifest") }],
    ["tree", { path: "landing.json", type: "tree", mode: "040000", sha: hex("manifest") }],
  ])("rejects a %s-mode ownership manifest before reading its contents", async (_label, manifestEntry) => {
    const mock = fetchQueue(
      json({ tree: { sha: hex("root") } }),
      tree([{ path: ".forme", type: "tree", sha: hex("forme") }]),
      tree([{ path: "deployments", type: "tree", sha: hex("deployments") }]),
      json({ truncated: false, tree: [manifestEntry] }),
      json({
        encoding: "base64",
        content: Buffer.from(JSON.stringify({
          version: 1, owner: "landing", destination: "", files: {},
        })).toString("base64"),
      }),
    );
    const boundary = createGitHubRestBoundary({ token: "secret", fetch: mock.fetch });
    await expect(boundary.listOwnershipManifests({
      owner: "octo", repository: "site", commitSha: hex("commit"),
    })).rejects.toMatchObject({ code: "MALFORMED_RESPONSE" });
    expect(mock.calls).toHaveLength(4);
  });
});

interface FetchCall {
  readonly url: string;
  readonly init: RequestInit;
}

function fetchQueue(...responses: Response[]) {
  const calls: FetchCall[] = [];
  const fetch = async (input: string | URL | Request, init: RequestInit = {}) => {
    calls.push({ url: String(input), init });
    const response = responses.shift();
    if (response === undefined) throw new Error("unexpected fetch");
    return response;
  };
  return { calls, fetch: fetch as typeof globalThis.fetch };
}

function json(value: unknown, status = 200): Response {
  return new Response(JSON.stringify(value), {
    status,
    headers: { "content-type": "application/json" },
  });
}

function tree(entries: readonly { readonly path: string; readonly type: "tree" | "blob"; readonly sha: string }[]): Response {
  return json({
    truncated: false,
    tree: entries.map((entry) => ({ ...entry, mode: entry.type === "tree" ? "040000" : "100644" })),
  });
}

function hex(label: string): string {
  return Buffer.from(label).toString("hex").padEnd(40, "0").slice(0, 40);
}
