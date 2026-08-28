import { afterEach, describe, expect, it } from "vitest";
import { mkdir, mkdtemp, rm, symlink, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import {
  Kinds,
  streamOf,
  type Asset,
  type AssetRef,
  type ContentNode,
  type LogicalId,
} from "@coding-adventures/forme-types";
import { computeBinaryRevisionId } from "@coding-adventures/forme-identity";
import {
  createCancellationTokenSource,
  silentLogger,
} from "@coding-adventures/forme-stage";
import loadAssetsFs, { detectMimeType } from "../src/index.js";

const roots: string[] = [];
const ID_A = "01952c0d-7e63-7000-8000-000000000041" as LogicalId;
const ID_B = "01952c0d-7e63-7000-8000-000000000042" as LogicalId;

afterEach(async () => {
  await Promise.all(roots.splice(0).map(root => rm(root, { recursive: true, force: true })));
});

async function tempRoot(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "forme-load-assets-"));
  roots.push(root);
  return root;
}

function ref(
  id: LogicalId,
  sourcePath: string | undefined,
  role: AssetRef["role"] = "image",
): AssetRef {
  return {
    id,
    nodePath: [0],
    role,
    ...(sourcePath === undefined ? {} : { sourcePath }),
  };
}

function node(...assetRefs: AssetRef[]): ContentNode {
  return {
    identity: "01952c0d-7e63-7000-8000-000000000001" as LogicalId,
    revision: "blake2b:" as never,
    document: { type: "document", children: [] },
    frontmatter: {},
    route: "/post.html",
    assetRefs,
    sourcePath: "post.md",
  };
}

async function* nodes(...values: ContentNode[]): AsyncIterable<ContentNode> {
  yield* values;
}

function context(cancelled = false): Parameters<typeof loadAssetsFs.run>[2] {
  const source = createCancellationTokenSource();
  if (cancelled) source.cancel("test");
  return { logger: silentLogger(), cancellation: source.token } as never;
}

async function collect(
  values: AsyncIterable<ContentNode>,
  config: unknown,
  cancelled = false,
): Promise<Asset[]> {
  const output = (loadAssetsFs.run as Function)(values, config, context(cancelled));
  const assets: Asset[] = [];
  for await (const asset of output) assets.push(asset);
  return assets;
}

describe("MIME detection", () => {
  it("prefers byte signatures over misleading extensions", () => {
    const png = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a]);
    expect(detectMimeType("photo.txt", png)).toBe("image/png");
    expect(detectMimeType("photo.png", new Uint8Array([0xff, 0xd8, 0xff]))).toBe("image/jpeg");
    expect(detectMimeType("old.bin", new TextEncoder().encode("GIF89a"))).toBe("image/gif");
    expect(detectMimeType("doc.bin", new TextEncoder().encode("%PDF-1.7"))).toBe("application/pdf");
  });

  it("detects WebP, fonts, ISO media brands, and SVG", () => {
    expect(detectMimeType("x.bin", new TextEncoder().encode("RIFF0000WEBP"))).toBe("image/webp");
    expect(detectMimeType("x.bin", new TextEncoder().encode("wOFFdata"))).toBe("font/woff");
    expect(detectMimeType("x.bin", new TextEncoder().encode("wOF2data"))).toBe("font/woff2");
    expect(detectMimeType("x.bin", new Uint8Array([0, 0, 1, 0]))).toBe("image/x-icon");
    expect(detectMimeType("x.bin", new TextEncoder().encode("0000ftypavif"))).toBe("image/avif");
    expect(detectMimeType("x.bin", new TextEncoder().encode("0000ftypheic"))).toBe("image/heic");
    expect(detectMimeType("x.bin", new TextEncoder().encode("0000ftypisom"))).toBe("video/mp4");
    expect(detectMimeType("x.bin", new TextEncoder().encode(" <?xml version='1.0'?><svg/>")))
      .toBe("image/svg+xml");
  });

  it("falls back to a case-insensitive extension and then octet-stream", () => {
    expect(detectMimeType("images/photo.JPEG", new Uint8Array())).toBe("image/jpeg");
    expect(detectMimeType("assets/data.unknown", new Uint8Array())).toBe("application/octet-stream");
  });
});

describe("loadAssetsFs stage", () => {
  it("declares the collector stream contract and storage capability", () => {
    expect(loadAssetsFs.consumes).toEqual(streamOf(Kinds.ContentNode));
    expect(loadAssetsFs.produces).toEqual(streamOf(Kinds.Asset));
    expect(loadAssetsFs.capabilities).toEqual(["storage:read"]);
  });

  it("loads, hashes, annotates, sorts, and deduplicates unique sources", async () => {
    const root = await tempRoot();
    await mkdir(join(root, "images"));
    const alpha = new Uint8Array([0xff, 0xd8, 0xff, 1]);
    const zeta = new Uint8Array([0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 2]);
    await writeFile(join(root, "images", "alpha.jpg"), alpha);
    await writeFile(join(root, "images", "zeta.png"), zeta);

    const assets = await collect(nodes(
      node(ref(ID_B, "images/zeta.png"), ref(ID_A, "images/alpha.jpg")),
      node(ref(ID_B, "images/zeta.png")),
    ), { root });

    expect(assets.map(asset => asset.identity)).toEqual([ID_A, ID_B]);
    expect(assets[0]).toMatchObject({
      identity: ID_A,
      role: "image",
      mimeType: "image/jpeg",
      byteLength: alpha.byteLength,
      dimensions: null,
      durationMs: null,
      derivedFrom: null,
      meta: { sourcePath: "images/alpha.jpg" },
    });
    expect(assets[0]!.revision).toBe(computeBinaryRevisionId(alpha));
    expect(assets[0]!.bytes).toEqual(alpha);
    expect(assets[1]!.revision).toBe(computeBinaryRevisionId(zeta));
  });

  it("returns an empty stream for content without asset references", async () => {
    const root = await tempRoot();
    await expect(collect(nodes(node()), { root })).resolves.toEqual([]);
  });

  it("rejects missing source locators and malformed portable paths", async () => {
    const root = await tempRoot();
    await expect(collect(nodes(node(ref(ID_A, undefined))), { root }))
      .rejects.toThrow(/forme-resolve-asset-refs-fs upstream/);
    for (const sourcePath of ["", "../secret.png", "a/../b.png", "/root.png", "C:/root.png", "a\\b.png"]) {
      await expect(collect(nodes(node(ref(ID_A, sourcePath))), { root }))
        .rejects.toThrow(/invalid resolved sourcePath/);
    }
  });

  it("rejects missing files, directories, invalid roots, and empty config roots", async () => {
    const root = await tempRoot();
    await mkdir(join(root, "directory.png"));
    await writeFile(join(root, "root-file"), "not a root");
    await expect(collect(nodes(node(ref(ID_A, "missing.png"))), { root }))
      .rejects.toThrow(/does not exist/);
    await expect(collect(nodes(node(ref(ID_A, "directory.png"))), { root }))
      .rejects.toThrow(/not a regular file/);
    await expect(collect(nodes(), { root: join(root, "missing-root") }))
      .rejects.toThrow(/storage root is unavailable/);
    await expect(collect(nodes(), { root: join(root, "root-file") }))
      .rejects.toThrow(/storage root is not a directory/);
    await expect(collect(nodes(), { root: "" })).rejects.toThrow(/config.root/);
  });

  it("rejects conflicting identity, source, and role claims before emitting", async () => {
    const root = await tempRoot();
    await expect(collect(nodes(node(
      ref(ID_A, "same.png"),
      ref(ID_B, "same.png"),
    )), { root })).rejects.toThrow(/source .* conflicting identity or role/);
    await expect(collect(nodes(node(
      ref(ID_A, "a.png"),
      ref(ID_A, "b.png"),
    )), { root })).rejects.toThrow(/logical id .* conflicting source or role/);
    await expect(collect(nodes(node(
      ref(ID_A, "same.png", "image"),
      ref(ID_A, "same.png", "binary"),
    )), { root })).rejects.toThrow(/conflicting identity or role/);
  });

  it("allows an in-root symlink but rejects a symlink escape", async () => {
    const root = await tempRoot();
    const outside = await tempRoot();
    await mkdir(join(root, "images"));
    await writeFile(join(root, "images", "real.png"), new Uint8Array([1, 2, 3]));
    await writeFile(join(outside, "secret.png"), new Uint8Array([9, 9, 9]));
    try {
      await symlink(join(root, "images", "real.png"), join(root, "images", "inside.png"));
      await symlink(join(outside, "secret.png"), join(root, "images", "outside.png"));
    } catch {
      return;
    }
    const [inside] = await collect(nodes(node(ref(ID_A, "images/inside.png"))), { root });
    expect(inside!.bytes).toEqual(new Uint8Array([1, 2, 3]));
    await expect(collect(nodes(node(ref(ID_B, "images/outside.png"))), { root }))
      .rejects.toThrow(/resolves outside storage root via symlink/);
  });

  it("honours cancellation before filesystem collection", async () => {
    const root = await tempRoot();
    await expect(collect(nodes(node(ref(ID_A, "asset.png"))), { root }, true))
      .rejects.toThrow("test");
  });
});
