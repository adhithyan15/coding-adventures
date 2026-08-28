import { afterEach, describe, expect, it } from "vitest";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import type { DocumentNode } from "@coding-adventures/document-ast";
import { Kinds, streamOf, type ContentNode, type LogicalId } from "@coding-adventures/forme-types";
import { isLogicalIdShape } from "@coding-adventures/forme-identity";
import {
  createCancellationTokenSource,
  silentLogger,
} from "@coding-adventures/forme-stage";
import resolveAssetRefsFs, {
  isLocalAssetDestination,
  resolveAssetSource,
} from "../src/index.js";

const roots: string[] = [];

afterEach(async () => {
  await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});

async function tempRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "forme-asset-refs-"));
  roots.push(root);
  return root;
}

function documentWithImages(destinations: readonly string[]): DocumentNode {
  return {
    type: "document",
    children: [{
      type: "blockquote",
      children: [{
        type: "paragraph",
        children: destinations.map((destination, index) => ({
          type: "image",
          destination,
          title: null,
          alt: `image ${index}`,
        })),
      }],
    }],
  };
}

function node(sourcePath: string, document: DocumentNode): ContentNode {
  return {
    identity: "01952c0d-7e63-7000-8000-000000000001" as never,
    revision: "blake2b:00" as never,
    document,
    frontmatter: {},
    route: null,
    assetRefs: [],
    sourcePath,
  };
}

async function* nodes(...values: ContentNode[]): AsyncIterable<ContentNode> {
  yield* values;
}

function context(cancelled = false): Parameters<typeof resolveAssetRefsFs.run>[2] {
  const source = createCancellationTokenSource();
  if (cancelled) source.cancel("test");
  return { logger: silentLogger(), cancellation: source.token } as never;
}

describe("asset destination resolution", () => {
  it("classifies local and external destinations", () => {
    expect(isLocalAssetDestination("../images/cat.png")).toBe(true);
    expect(isLocalAssetDestination("/images/cat.png")).toBe(true);
    expect(isLocalAssetDestination("https://example.com/cat.png")).toBe(false);
    expect(isLocalAssetDestination("data:image/png;base64,AA==")).toBe(false);
    expect(isLocalAssetDestination("//cdn.example.com/cat.png")).toBe(false);
    expect(isLocalAssetDestination("#poster")).toBe(false);
    expect(isLocalAssetDestination(" ")).toBe(false);
  });

  it("normalizes relative, root-relative, encoded, query, and Windows-style paths", async () => {
    const root = await tempRoot();
    expect(resolveAssetSource(root, "posts/hello.md", "../images/a%20b.png?v=1#x"))
      .toMatchObject({ sourcePath: "images/a b.png", urlSuffix: "?v=1#x" });
    expect(resolveAssetSource(root, "posts/hello.md", "/images/logo.svg")?.sourcePath)
      .toBe("images/logo.svg");
    expect(resolveAssetSource(root, "posts\\hello.md", "..\\images\\cat.png")?.sourcePath)
      .toBe("images/cat.png");
  });

  it("rejects traversal, malformed encoding, and storage-root references", async () => {
    const root = await tempRoot();
    expect(() => resolveAssetSource(root, "post.md", "../../secret.png")).toThrow(/escapes storage root/);
    expect(() => resolveAssetSource(root, "post.md", "%ZZ.png")).toThrow(/malformed percent/);
    expect(() => resolveAssetSource(root, "post.md", "bad%00.png")).toThrow(/null bytes/);
    expect(() => resolveAssetSource(root, "post.md", "/")).toThrow(/storage root/);
  });
});

describe("resolveAssetRefsFs stage", () => {
  it("declares the stream contract and storage capability", () => {
    expect(resolveAssetRefsFs.consumes).toEqual(streamOf(Kinds.ContentNode));
    expect(resolveAssetRefsFs.produces).toEqual(streamOf(Kinds.ContentNode));
    expect(resolveAssetRefsFs.capabilities).toEqual(["storage:read"]);
  });

  it("rejects an empty configured root", async () => {
    const output = (resolveAssetRefsFs.run as Function)(nodes(), { root: "" }, context());
    await expect(collectDirect(output)).rejects.toThrow(/config.root/);
  });

  it("discovers nested local images and preserves external URLs", async () => {
    const root = await tempRoot();
    const input = node("posts/hello.md", documentWithImages([
      "../images/cat.png",
      "https://example.com/remote.png",
      "data:image/png;base64,AA==",
    ]));
    const output = (resolveAssetRefsFs.run as Function)(nodes(input), { root }, context());
    const [resolved] = await collectDirect(output);

    expect(resolved!.assetRefs).toHaveLength(1);
    expect(resolved!.assetRefs[0]).toMatchObject({
      nodePath: [0, 0, 0],
      role: "image",
      sourcePath: "images/cat.png",
    });
    expect(resolved!.revision).not.toBe(input.revision);
    expect(resolved!.document).toBe(input.document);
  });

  it("reuses one identity for duplicate source paths across the stream", async () => {
    const root = await tempRoot();
    const first = node("posts/a.md", documentWithImages(["../images/shared.png"]));
    const second = node("posts/b.md", documentWithImages(["../images/shared.png"]));
    const output = (resolveAssetRefsFs.run as Function)(
      nodes(first, second),
      { root, persistIdentities: false },
      context(),
    );
    const resolved = await collectDirect(output);
    expect(resolved[0]!.assetRefs[0]!.id).toBe(resolved[1]!.assetRefs[0]!.id);
  });

  it("keeps asset resolution revision-independent from routing decoration", async () => {
    const root = await tempRoot();
    const unrouted = node("posts/a.md", documentWithImages(["../images/shared.png"]));
    const routed = { ...unrouted, route: "/blog/a.html" };
    const output = (resolveAssetRefsFs.run as Function)(
      nodes(unrouted, routed),
      { root, persistIdentities: false },
      context(),
    );
    const resolved = await collectDirect(output);
    expect(resolved[0]!.revision).toBe(resolved[1]!.revision);
  });

  it("loads a valid adjacent identity sidecar", async () => {
    const root = await tempRoot();
    await mkdir(join(root, "images"), { recursive: true });
    const id = "01952c0d-7e63-7000-8000-000000000099";
    await writeFile(join(root, "images", ".cat.png.id.json"), JSON.stringify({ logicalId: id }));
    const output = (resolveAssetRefsFs.run as Function)(
      nodes(node("posts/a.md", documentWithImages(["../images/cat.png"]))),
      { root },
      context(),
    );
    const [resolved] = await collectDirect(output);
    expect(resolved!.assetRefs[0]!.id).toBe(id);
  });

  it("falls back safely for unreadable, invalid, and malformed sidecars", async () => {
    const root = await tempRoot();
    await mkdir(join(root, "images"), { recursive: true });
    await mkdir(join(root, "images", ".unreadable.png.id.json"));
    await writeFile(join(root, "images", ".invalid.png.id.json"), "not-json");
    await writeFile(join(root, "images", ".malformed.png.id.json"), JSON.stringify({ logicalId: "nope" }));
    const output = (resolveAssetRefsFs.run as Function)(nodes(node(
      "post.md",
      documentWithImages([
        "images/unreadable.png",
        "images/invalid.png",
        "images/malformed.png",
      ]),
    )), { root }, context());
    const [resolved] = await collectDirect(output);
    expect(resolved!.assetRefs).toHaveLength(3);
    for (const ref of resolved!.assetRefs) expect(isLogicalIdShape(ref.id)).toBe(true);
  });

  it("rejects one persisted identity claimed by two source paths", async () => {
    const root = await tempRoot();
    await mkdir(join(root, "images"), { recursive: true });
    const id = "01952c0d-7e63-7000-8000-000000000099";
    await writeFile(join(root, "images", ".a.png.id.json"), JSON.stringify({ logicalId: id }));
    await writeFile(join(root, "images", ".b.png.id.json"), JSON.stringify({ logicalId: id }));
    const output = (resolveAssetRefsFs.run as Function)(nodes(
      node("one.md", documentWithImages(["images/a.png"])),
      node("two.md", documentWithImages(["images/b.png"])),
    ), { root }, context());
    await expect(collectDirect(output)).rejects.toThrow(/claimed by both/);
  });

  it("rejects one source path claimed by two existing identities", async () => {
    const root = await tempRoot();
    const firstId = "01952c0d-7e63-7000-8000-000000000031" as LogicalId;
    const secondId = "01952c0d-7e63-7000-8000-000000000032" as LogicalId;
    const first: ContentNode = {
      ...node("posts/a.md", documentWithImages(["../images/shared.png"])),
      assetRefs: [{ id: firstId, nodePath: [0, 0, 0], role: "image", sourcePath: "images/shared.png" }],
    };
    const second: ContentNode = {
      ...node("posts/b.md", documentWithImages(["../images/shared.png"])),
      assetRefs: [{ id: secondId, nodePath: [0, 0, 0], role: "image", sourcePath: "images/shared.png" }],
    };
    const output = (resolveAssetRefsFs.run as Function)(nodes(first, second), { root }, context());
    await expect(collectDirect(output)).rejects.toThrow(/source .* is claimed by both/);
  });

  it("fails closed on traversal and honours cancellation", async () => {
    const root = await tempRoot();
    const traversal = (resolveAssetRefsFs.run as Function)(
      nodes(node("post.md", documentWithImages(["../../secret.png"]))),
      { root },
      context(),
    );
    await expect(collectDirect(traversal)).rejects.toThrow(/escapes storage root/);

    const cancelled = (resolveAssetRefsFs.run as Function)(
      nodes(node("post.md", documentWithImages(["image.png"]))),
      { root },
      context(true),
    );
    await expect(collectDirect(cancelled)).rejects.toThrow();
  });

  it("preserves non-image refs and is idempotent for resolved images", async () => {
    const root = await tempRoot();
    const otherId = "01952c0d-7e63-7000-8000-000000000010" as LogicalId;
    const original: ContentNode = {
      ...node("post.md", documentWithImages(["image.png"])),
      assetRefs: [{ id: otherId, nodePath: [], role: "font", sourcePath: "fonts/site.woff2" }],
    };
    const first = await collectDirect((resolveAssetRefsFs.run as Function)(
      nodes(original), { root, persistIdentities: false }, context(),
    ));
    const second = await collectDirect((resolveAssetRefsFs.run as Function)(
      nodes(first[0]!), { root, persistIdentities: false }, context(),
    ));
    expect(second[0]!.assetRefs).toEqual(first[0]!.assetRefs);
    expect(second[0]!.revision).toBe(first[0]!.revision);
  });
});

async function collectDirect(iterable: AsyncIterable<unknown>): Promise<ContentNode[]> {
  const values: ContentNode[] = [];
  for await (const value of iterable) values.push(value as ContentNode);
  return values;
}
