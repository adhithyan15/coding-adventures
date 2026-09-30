import { chmod, link, lstat, mkdir, mkdtemp, readFile, symlink, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import {
  formatGrantsFile,
  formatTrustStore,
  parseGrantsFile,
  parseTrustStore,
  readGrantsFile,
  readTrustStore,
  writeGrantsFile,
  writeTrustStore,
} from "../src/index.js";

const KEY_A = Buffer.alloc(32, 0x11).toString("base64");
const KEY_B = Buffer.alloc(32, 0x22).toString("base64");
const HASH_A = `blake2b:${"a".repeat(64)}`;
const HASH_B = `blake2b:${"b".repeat(64)}`;
const WHEN = "2026-09-30T12:00:00Z";

describe("persistent plugin authority codecs", () => {
  it("round-trips trust keys in canonical order and escapes notes", () => {
    const text = formatTrustStore({
      trustedKeys: [
        { algorithm: "ed25519", publicKey: KEY_B, addedAt: WHEN, note: "second\nline" },
        { algorithm: "ed25519", publicKey: KEY_A, addedAt: WHEN },
      ],
    });
    expect(text).toContain(`publicKey = "${KEY_A}"`);
    expect(text.indexOf(KEY_A)).toBeLessThan(text.indexOf(KEY_B));
    expect(text).toContain('note = "second\\nline"');
    expect(parseTrustStore(text)).toEqual({
      trustedKeys: [
        { algorithm: "ed25519", publicKey: KEY_A, addedAt: WHEN },
        { algorithm: "ed25519", publicKey: KEY_B, addedAt: WHEN, note: "second\nline" },
      ],
    });
  });

  it("round-trips grants in code-point order", () => {
    const text = formatGrantsFile({
      manifestHash: HASH_A,
      granted: [
        { capability: "storage:write", grantedAt: WHEN, note: "needed" },
        { capability: "network:https:example.com", grantedAt: WHEN },
      ],
    });
    expect(text.indexOf("network:https:example.com")).toBeLessThan(text.indexOf("storage:write"));
    expect(parseGrantsFile(text)).toEqual({
      manifestHash: HASH_A,
      granted: [
        { capability: "network:https:example.com", grantedAt: WHEN },
        { capability: "storage:write", grantedAt: WHEN, note: "needed" },
      ],
    });
  });

  it("accepts comments, quoted comment markers, and empty grant lists", () => {
    expect(parseTrustStore(`# heading\n[[trustedKeys]] # row\nalgorithm = "ed25519"\npublicKey = "${KEY_A}"\naddedAt = "${WHEN}"\nnote = "escaped \\\"#\\\" marker" # tail\n`))
      .toEqual({ trustedKeys: [{ algorithm: "ed25519", publicKey: KEY_A, addedAt: WHEN, note: 'escaped "#" marker' }] });
    expect(formatGrantsFile({ manifestHash: HASH_A, granted: [] }))
      .toBe(`manifestHash = "${HASH_A}"\n`);
  });

  it("implements TOML basic-string Unicode escapes and scalar validation", () => {
    const text = `[[trustedKeys]]\nalgorithm = "ed25519"\npublicKey = "${KEY_A}"\naddedAt = "${WHEN}"\nnote = "smile: \\U0001F600"\n`;
    expect(parseTrustStore(text).trustedKeys[0]?.note).toBe("smile: 😀");
    expect(() => parseTrustStore(text.replace("\\U0001F600", "\\/"))).toThrow(/TOML escape/);
    expect(() => parseTrustStore(text.replace("\\U0001F600", "\\uD800"))).toThrow(/non-scalar/);
    expect(() => parseTrustStore(text.replace("\\U0001F600", "\\u12"))).toThrow(/Unicode escape/);
    expect(() => parseTrustStore(text.replace("\\U0001F600", "\\uZZZZ"))).toThrow(/Unicode escape/);
    expect(() => parseTrustStore(text.replace("smile: \\U0001F600", 'bad " quote'))).toThrow(/quote/);
    const dangling = text.replace('note = "smile: \\U0001F600"', 'note = "dangling' + "\\" + '"');
    expect(() => parseTrustStore(dangling)).toThrow(/dangling escape/);
    expect(() => parseTrustStore(text.replace("\\U0001F600", "\uD800"))).toThrow(/invalid character/);
    expect(() => parseTrustStore(text.replace("\\U0001F600", "\u0001"))).toThrow(/invalid character/);
    expect(() => parseTrustStore(text.replace("\\U0001F600", "\u007f"))).toThrow(/invalid character/);
    expect(() => formatTrustStore({
      trustedKeys: [{ algorithm: "ed25519", publicKey: KEY_A, addedAt: WHEN, note: "\uD800" }],
    })).toThrow(/Unicode scalar/);
  });

  it("sorts authority rows by Unicode code point rather than UTF-16 code unit", () => {
    const bmp = "custom:\uE000";
    const supplementary = "custom:\u{10000}";
    const text = formatGrantsFile({
      manifestHash: HASH_A,
      granted: [
        { capability: supplementary, grantedAt: WHEN },
        { capability: bmp, grantedAt: WHEN },
      ],
    });
    expect(text.indexOf(bmp)).toBeLessThan(text.indexOf(supplementary));
    expect(parseGrantsFile(text).granted.map(entry => entry.capability)).toEqual([bmp, supplementary]);
    const sharedSupplementary = formatGrantsFile({
      manifestHash: HASH_A,
      granted: [
        { capability: "custom:\u{10000}b", grantedAt: WHEN },
        { capability: "custom:\u{10000}a", grantedAt: WHEN },
        { capability: "custom:a", grantedAt: WHEN },
        { capability: "custom:aa", grantedAt: WHEN },
      ],
    });
    expect(parseGrantsFile(sharedSupplementary).granted.map(entry => entry.capability)).toEqual([
      "custom:a", "custom:aa", "custom:\u{10000}a", "custom:\u{10000}b",
    ]);
  });

  it.each([
    ["non-string input", () => parseTrustStore(null as never)],
    ["byte-order mark", () => parseTrustStore(`\uFEFF[[trustedKeys]]\n`)],
    ["unknown table", () => parseTrustStore("[[other]]\n")],
    ["malformed assignment", () => parseTrustStore("not an assignment\n")],
    ["unquoted string", () => parseTrustStore("[[trustedKeys]]\nalgorithm = ed25519\n")],
    ["invalid escaped string", () => parseTrustStore('[[trustedKeys]]\nalgorithm = "\\q"\n')],
  ])("rejects %s", (_label, action) => {
    expect(action).toThrow();
  });

  it.each([
    ["unknown top-level field", `other = "x"\n`],
    ["field outside a trust row", `algorithm = "ed25519"\n`],
    ["duplicate field", `[[trustedKeys]]\nalgorithm = "ed25519"\nalgorithm = "ed25519"\npublicKey = "${KEY_A}"\naddedAt = "${WHEN}"\n`],
    ["unknown row field", `[[trustedKeys]]\nalgorithm = "ed25519"\npublicKey = "${KEY_A}"\naddedAt = "${WHEN}"\nextra = "x"\n`],
    ["wrong algorithm", `[[trustedKeys]]\nalgorithm = "rsa"\npublicKey = "${KEY_A}"\naddedAt = "${WHEN}"\n`],
    ["noncanonical key", `[[trustedKeys]]\nalgorithm = "ed25519"\npublicKey = "${KEY_A.replace(/=$/, "")}"\naddedAt = "${WHEN}"\n`],
    ["bad timestamp", `[[trustedKeys]]\nalgorithm = "ed25519"\npublicKey = "${KEY_A}"\naddedAt = "today"\n`],
    ["duplicate key", `[[trustedKeys]]\nalgorithm = "ed25519"\npublicKey = "${KEY_A}"\naddedAt = "${WHEN}"\n[[trustedKeys]]\nalgorithm = "ed25519"\npublicKey = "${KEY_A}"\naddedAt = "${WHEN}"\n`],
  ])("rejects trust-store %s", (_label, text) => {
    expect(() => parseTrustStore(text)).toThrow();
  });

  it.each([
    ["missing hash", `[[granted]]\ncapability = "storage:read"\ngrantedAt = "${WHEN}"\n`],
    ["duplicate hash", `manifestHash = "${HASH_A}"\nmanifestHash = "${HASH_A}"\n`],
    ["invalid hash", `manifestHash = "sha256:${"a".repeat(64)}"\n`],
    ["invalid capability", `manifestHash = "${HASH_A}"\n[[granted]]\ncapability = "storage read"\ngrantedAt = "${WHEN}"\n`],
    ["duplicate capability", `manifestHash = "${HASH_A}"\n[[granted]]\ncapability = "storage:read"\ngrantedAt = "${WHEN}"\n[[granted]]\ncapability = "storage:read"\ngrantedAt = "${WHEN}"\n`],
    ["unknown field", `manifestHash = "${HASH_A}"\n[[granted]]\ncapability = "storage:read"\ngrantedAt = "${WHEN}"\nextra = "x"\n`],
  ])("rejects grants-file %s", (_label, text) => {
    expect(() => parseGrantsFile(text)).toThrow();
  });

  it("bounds bytes and row counts", () => {
    expect(() => parseTrustStore("#".repeat(1024 * 1024 + 1))).toThrow(/byte limit/);
    const row = `[[granted]]\ncapability = "storage:read"\ngrantedAt = "${WHEN}"\n`;
    expect(() => parseGrantsFile(`manifestHash = "${HASH_A}"\n${row.repeat(4_097)}`))
      .toThrow(/row limit/);
  });

  it("bounds formatter output before joining repeated large rows", () => {
    const largeNote = "x".repeat(1024 * 1024);
    expect(() => formatTrustStore({
      trustedKeys: [{ algorithm: "ed25519", publicKey: KEY_A, addedAt: WHEN, note: largeNote }],
    })).toThrow(/byte limit/);
    expect(() => formatTrustStore({
      trustedKeys: [{ algorithm: "ed25519", publicKey: KEY_A, addedAt: WHEN, note: '"'.repeat(600_000) }],
    })).toThrow(/byte limit/);
    expect(() => formatGrantsFile({
      manifestHash: HASH_A,
      granted: Array.from({ length: 4_096 }, (_, index) => ({
        capability: `network:https:host-${index}.example`,
        grantedAt: WHEN,
        note: largeNote,
      })),
    })).toThrow(/byte limit/);
  });

  it("validates trust stores passed to the formatter", () => {
    const valid = { algorithm: "ed25519" as const, publicKey: KEY_A, addedAt: WHEN };
    const tooMany = Array.from({ length: 4_097 }, () => valid);
    const cases: unknown[] = [
      null,
      {},
      { trustedKeys: "no" },
      { trustedKeys: tooMany },
      { trustedKeys: [valid, valid] },
      { trustedKeys: [null] },
      { trustedKeys: [{ ...valid, extra: "x" }] },
      { trustedKeys: [{ ...valid, algorithm: "rsa" }] },
      { trustedKeys: [{ ...valid, publicKey: 7 }] },
      { trustedKeys: [{ ...valid, publicKey: "!!!!" }] },
      { trustedKeys: [{ ...valid, publicKey: Buffer.alloc(31).toString("base64") }] },
      { trustedKeys: [{ ...valid, addedAt: 7 }] },
      { trustedKeys: [{ ...valid, addedAt: "2026-99-30T12:00:00Z" }] },
      { trustedKeys: [{ ...valid, addedAt: "2026-02-30T12:00:00Z" }] },
      { trustedKeys: [{ ...valid, addedAt: "2026-01-01T24:00:00Z" }] },
      { trustedKeys: [{ ...valid, addedAt: "2026-01-01T23:60:00Z" }] },
      { trustedKeys: [{ ...valid, addedAt: "2026-01-01T23:59:60Z" }] },
      { trustedKeys: [{ ...valid, note: 7 }] },
      { trustedKeys: [], extra: "x" },
      Object.create({ trustedKeys: [] }),
    ];
    for (const value of cases) expect(() => formatTrustStore(value as never)).toThrow();
  });

  it("validates grants passed to the formatter", () => {
    const valid = { capability: "storage:read" as const, grantedAt: WHEN };
    const tooMany = Array.from({ length: 4_097 }, () => valid);
    const cases: unknown[] = [
      null,
      {},
      { manifestHash: HASH_A, granted: "no" },
      { manifestHash: "bad", granted: [] },
      { manifestHash: HASH_A, granted: tooMany },
      { manifestHash: HASH_A, granted: [valid, valid] },
      { manifestHash: HASH_A, granted: [null] },
      { manifestHash: HASH_A, granted: [{ ...valid, extra: "x" }] },
      { manifestHash: HASH_A, granted: [{ ...valid, capability: 7 }] },
      { manifestHash: HASH_A, granted: [{ ...valid, grantedAt: 7 }] },
      { manifestHash: HASH_A, granted: [{ ...valid, grantedAt: "2026-99-30T12:00:00Z" }] },
      { manifestHash: HASH_A, granted: [{ ...valid, note: 7 }] },
      { manifestHash: HASH_A, granted: [], extra: "x" },
      Object.create({ manifestHash: HASH_A, granted: [] }),
    ];
    for (const value of cases) expect(() => formatGrantsFile(value as never)).toThrow();
    expect(() => formatTrustStore({
      trustedKeys: [{ ...({ algorithm: "ed25519" as const, publicKey: KEY_A }), addedAt: "2024-02-29T23:59:59.123Z" }],
    })).not.toThrow();
  });
});

describe("persistent plugin authority files", () => {
  it("treats absent files as empty or missing and fails closed on stale grants", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-authority-missing-"));
    await expect(readTrustStore(join(root, "trust.toml"))).resolves.toEqual({ trustedKeys: [] });
    await expect(readGrantsFile(join(root, "grants.toml"), HASH_A)).resolves.toEqual({
      status: "missing", capabilities: [], file: null,
    });
    const grantsPath = join(root, "grants.toml");
    await writeGrantsFile(grantsPath, {
      manifestHash: HASH_A,
      granted: [{ capability: "storage:read", grantedAt: WHEN }],
    });
    await expect(readGrantsFile(grantsPath, HASH_A)).resolves.toMatchObject({
      status: "current", capabilities: ["storage:read"],
    });
    await expect(readGrantsFile(grantsPath, HASH_B)).resolves.toMatchObject({
      status: "stale", capabilities: [], file: { manifestHash: HASH_A },
    });
  });

  it("publishes deterministic restrictive files without leftover staging files", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-authority-write-"));
    const nested = join(root, "owned", "trust.toml");
    await mkdir(join(root, "owned"));
    const store = { trustedKeys: [{ algorithm: "ed25519" as const, publicKey: KEY_A, addedAt: WHEN }] };
    await writeTrustStore(nested, store);
    const first = await readFile(nested, "utf8");
    await writeTrustStore(nested, store);
    expect(await readFile(nested, "utf8")).toBe(first);
    expect(await readTrustStore(nested)).toEqual(store);
    if (process.platform !== "win32") expect((await lstat(nested)).mode & 0o777).toBe(0o600);
    expect((await import("node:fs/promises")).readdir(join(root, "owned")))
      .resolves.toEqual(["trust.toml"]);
  });

  it("rejects symlinked and multiply-linked targets on read and write", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-authority-links-"));
    const real = join(root, "real.toml");
    const linked = join(root, "linked.toml");
    const hard = join(root, "hard.toml");
    await writeFile(real, formatTrustStore({ trustedKeys: [] }), { mode: 0o600 });
    await symlink(real, linked);
    await expect(readTrustStore(linked)).rejects.toThrow(/linked|regular file/);
    await expect(writeTrustStore(linked, { trustedKeys: [] })).rejects.toThrow(/linked|regular file/);
    await link(real, hard);
    await expect(readTrustStore(real)).rejects.toThrow(/linked|regular file/);
    await expect(writeTrustStore(real, { trustedKeys: [] })).rejects.toThrow(/linked|regular file/);
  });

  it("rejects oversized files before parsing", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-authority-large-"));
    const path = join(root, "trust.toml");
    await writeFile(path, "#".repeat(1024 * 1024 + 1));
    await expect(readTrustStore(path)).rejects.toThrow(/bounded regular file|byte limit/);
  });

  it("rejects authority files that are not valid UTF-8", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-authority-utf8-"));
    const path = join(root, "trust.toml");
    await writeFile(path, Buffer.from([0xff]));
    await expect(readTrustStore(path)).rejects.toThrow(/UTF-8/);
  });

  it("rejects unsafe parents, directories, and invalid expected hashes", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-authority-unsafe-"));
    await expect(readGrantsFile(join(root, "missing.toml"), "bad"))
      .rejects.toThrow(/manifestHash/);
    await expect(readTrustStore(root)).rejects.toThrow(/regular file/);
    await expect(writeTrustStore(join(root, "missing", "trust.toml"), { trustedKeys: [] }))
      .rejects.toThrow(/parent/);

    const realParent = join(root, "real-parent");
    const linkedParent = join(root, "linked-parent");
    await mkdir(realParent);
    await symlink(realParent, linkedParent);
    await expect(writeTrustStore(join(linkedParent, "trust.toml"), { trustedKeys: [] }))
      .rejects.toThrow(/parent/);

    const directoryTarget = join(realParent, "trust.toml");
    await mkdir(directoryTarget);
    await expect(writeTrustStore(directoryTarget, { trustedKeys: [] }))
      .rejects.toThrow(/regular file/);
  });

  it("bounds serialized files and cleans up when staging cannot start", async () => {
    const root = await mkdtemp(join(tmpdir(), "forme-authority-stage-"));
    await expect(writeTrustStore(join(root, "large.toml"), {
      trustedKeys: [{ algorithm: "ed25519", publicKey: KEY_A, addedAt: WHEN, note: "x".repeat(1024 * 1024) }],
    })).rejects.toThrow(/byte limit/);

    if (process.platform !== "win32") {
      await chmod(root, 0o500);
      try {
        await expect(writeTrustStore(join(root, "blocked.toml"), { trustedKeys: [] })).rejects.toThrow();
      } finally {
        await chmod(root, 0o700);
      }
      expect(await (await import("node:fs/promises")).readdir(root)).toEqual([]);
    }
  });
});
