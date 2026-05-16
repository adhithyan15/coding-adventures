import { describe, it, expect } from "vitest";
import {
  parseManifest,
  computeManifestHash,
  isManifestHashShape,
  MANIFEST_HASH_DIGEST_BYTES,
  MANIFEST_HASH_ALGORITHM,
} from "../src/index.js";

const M = `
manifestVersion = 1
[plugin]
name = "@me/x"
version = "1.0.0"
apiVersion = 1
[runtime]
kind = "node"
entry = "./e.js"
[[contributes.stages]]
id = "s"
consumes = "ContentSource"
produces = "ContentNode"
`;

describe("computeManifestHash", () => {
  it("produces the documented format", () => {
    const hash = computeManifestHash(parseManifest(M), new Uint8Array([1, 2, 3]));
    expect(hash).toMatch(/^blake2b:[0-9a-f]{64}$/);
  });

  it("is deterministic for the same inputs", () => {
    const m = parseManifest(M);
    const entry = new Uint8Array([1, 2, 3]);
    expect(computeManifestHash(m, entry)).toBe(computeManifestHash(m, entry));
  });

  it("changes when the entry file changes", () => {
    const m = parseManifest(M);
    const a = computeManifestHash(m, new Uint8Array([1, 2, 3]));
    const b = computeManifestHash(m, new Uint8Array([1, 2, 4]));
    expect(a).not.toBe(b);
  });

  it("changes when the manifest changes", () => {
    const entry = new Uint8Array([1]);
    const a = computeManifestHash(parseManifest(M), entry);
    const b = computeManifestHash(
      parseManifest(M.replace('version = "1.0.0"', 'version = "1.0.1"')),
      entry,
    );
    expect(a).not.toBe(b);
  });

  it("rejects non-Uint8Array entry argument", () => {
    expect(() => computeManifestHash(parseManifest(M), "hi" as unknown as Uint8Array))
      .toThrow(TypeError);
  });

  it("constants are sensible", () => {
    expect(MANIFEST_HASH_ALGORITHM).toBe("blake2b");
    expect(MANIFEST_HASH_DIGEST_BYTES).toBe(32);
  });

  it("isManifestHashShape accepts valid hashes", () => {
    const hash = computeManifestHash(parseManifest(M), new Uint8Array(1));
    expect(isManifestHashShape(hash)).toBe(true);
  });

  it("isManifestHashShape rejects bad shapes", () => {
    expect(isManifestHashShape("blake2b:" + "G".repeat(64))).toBe(false);  // non-hex
    expect(isManifestHashShape("blake2b:abc")).toBe(false);                // wrong length
    expect(isManifestHashShape("nohash")).toBe(false);                     // no colon
    expect(isManifestHashShape(":nohex")).toBe(false);                     // empty algo
    expect(isManifestHashShape("blake2b:")).toBe(false);                   // empty hex
    expect(isManifestHashShape("BLAKE2b:" + "a".repeat(64))).toBe(false);  // uppercase algo
    expect(isManifestHashShape(123 as unknown as string)).toBe(false);     // wrong type
    expect(isManifestHashShape("future:abcdef")).toBe(true);               // forward-compat
  });

  it("manifests differing only in [signature] block produce the same hash (signature is excluded)", () => {
    const noSig = parseManifest(M);
    const withSig = parseManifest(M + `
[signature]
algorithm = "ed25519"
publicKey = "AAA="
signature = "BBB="
signedAt = "2026-05-16T00:00:00Z"
`);
    const entry = new Uint8Array([1, 2, 3]);
    expect(computeManifestHash(noSig, entry)).toBe(computeManifestHash(withSig, entry));
  });
});
