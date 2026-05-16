import { describe, it, expect } from "vitest";
import { generateKeypair } from "@coding-adventures/ed25519";
import {
  parseManifest,
  signManifest,
  verifyManifest,
  assertManifestSigned,
  ManifestError,
  type Manifest,
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

const ENTRY = new Uint8Array([1, 2, 3, 4, 5]);

// 32-byte deterministic seed for tests — same seed across the whole file
// gives stable keys and lets the suite stay reproducible.
const SEED = new Uint8Array(32).fill(0x42);

function withSignedManifest(): Manifest {
  const keys = generateKeypair(SEED);
  const m = parseManifest(M);
  const sig = signManifest(m, ENTRY, {
    secretSeed: SEED,
    publicKey: keys.publicKey,
  }, "2026-05-16T12:00:00Z");
  return { ...m, signature: sig };
}

describe("signManifest", () => {
  it("produces a SignatureBlock with all four fields", () => {
    const keys = generateKeypair(SEED);
    const sig = signManifest(parseManifest(M), ENTRY, {
      secretSeed: SEED,
      publicKey: keys.publicKey,
    });
    expect(sig.algorithm).toBe("ed25519");
    expect(typeof sig.publicKey).toBe("string");
    expect(typeof sig.signature).toBe("string");
    expect(typeof sig.signedAt).toBe("string");
  });

  it("uses the provided signedAt for determinism", () => {
    const keys = generateKeypair(SEED);
    const sig = signManifest(parseManifest(M), ENTRY, {
      secretSeed: SEED,
      publicKey: keys.publicKey,
    }, "2026-05-16T12:00:00Z");
    expect(sig.signedAt).toBe("2026-05-16T12:00:00Z");
  });

  it("defaults signedAt to Date.now()-derived ISO when omitted", () => {
    const keys = generateKeypair(SEED);
    const sig = signManifest(parseManifest(M), ENTRY, {
      secretSeed: SEED,
      publicKey: keys.publicKey,
    });
    expect(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}/.test(sig.signedAt)).toBe(true);
  });

  it("rejects wrong-length seed", () => {
    const keys = generateKeypair(SEED);
    expect(() => signManifest(parseManifest(M), ENTRY, {
      secretSeed: new Uint8Array(16),
      publicKey: keys.publicKey,
    })).toThrowError(/secretSeed/);
  });

  it("rejects wrong-length publicKey", () => {
    expect(() => signManifest(parseManifest(M), ENTRY, {
      secretSeed: SEED,
      publicKey: new Uint8Array(16),
    })).toThrowError(/publicKey/);
  });
});

describe("verifyManifest", () => {
  it("returns true for a freshly-signed manifest", () => {
    const m = withSignedManifest();
    expect(verifyManifest(m, ENTRY)).toBe(true);
  });

  it("returns false when manifest has no signature", () => {
    expect(verifyManifest(parseManifest(M), ENTRY)).toBe(false);
  });

  it("returns false on wrong algorithm", () => {
    const m = withSignedManifest();
    const tampered: Manifest = { ...m, signature: { ...m.signature!, algorithm: "rsa" } };
    expect(verifyManifest(tampered, ENTRY)).toBe(false);
  });

  it("returns false when manifest body is tampered", () => {
    const m = withSignedManifest();
    const tampered: Manifest = {
      ...m,
      plugin: { ...m.plugin, version: "9.9.9" },
    };
    expect(verifyManifest(tampered, ENTRY)).toBe(false);
  });

  it("returns false when entry bytes are tampered", () => {
    const m = withSignedManifest();
    expect(verifyManifest(m, new Uint8Array([9, 9, 9]))).toBe(false);
  });

  it("returns false on malformed base64 in signature", () => {
    const m = withSignedManifest();
    const tampered: Manifest = {
      ...m,
      signature: { ...m.signature!, signature: "!not base64!" },
    };
    expect(verifyManifest(tampered, ENTRY)).toBe(false);
  });

  it("returns false on wrong-length decoded signature", () => {
    const m = withSignedManifest();
    // "QQ==" is base64 for [0x41] — 1 byte, not 64
    const tampered: Manifest = {
      ...m,
      signature: { ...m.signature!, signature: "QQ==" },
    };
    expect(verifyManifest(tampered, ENTRY)).toBe(false);
  });

  it("throws on non-Uint8Array entry", () => {
    const m = withSignedManifest();
    expect(() => verifyManifest(m, "x" as unknown as Uint8Array)).toThrow(TypeError);
  });
});

describe("assertManifestSigned", () => {
  it("returns void on a valid signed manifest", () => {
    const m = withSignedManifest();
    expect(() => assertManifestSigned(m, ENTRY)).not.toThrow();
  });

  it("throws ManifestError when no signature is present", () => {
    expect(() => assertManifestSigned(parseManifest(M), ENTRY)).toThrow(ManifestError);
  });

  it("throws on wrong algorithm", () => {
    const m = withSignedManifest();
    const tampered: Manifest = { ...m, signature: { ...m.signature!, algorithm: "rsa" } };
    expect(() => assertManifestSigned(tampered, ENTRY)).toThrowError(/algorithm/);
  });

  it("throws on verification failure", () => {
    const m = withSignedManifest();
    expect(() => assertManifestSigned(m, new Uint8Array([0]))).toThrow(/does not verify/);
  });
});
