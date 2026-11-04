// ============================================================================
// x25519.test.ts — Test suite for X25519 (RFC 7748)
// ============================================================================
//
// These tests verify the X25519 implementation against the official test
// vectors from RFC 7748 Section 6.1, plus the iterated test and the
// full Diffie-Hellman key exchange test.
// ============================================================================

import assert from "node:assert/strict";
import { describe, test } from "node:test";
import { x25519, x25519Base, generateKeypair } from "../src/index.js";

// ---------------------------------------------------------------------------
// Helper: convert hex string to Uint8Array
// ---------------------------------------------------------------------------

function hexToBytes(hex: string): Uint8Array {
  const bytes = new Uint8Array(hex.length / 2);
  for (let i = 0; i < hex.length; i += 2) {
    bytes[i / 2] = parseInt(hex.substring(i, i + 2), 16);
  }
  return bytes;
}

// ---------------------------------------------------------------------------
// Helper: convert Uint8Array to hex string
// ---------------------------------------------------------------------------

function bytesToHex(bytes: Uint8Array): string {
  return Array.from(bytes)
    .map((b) => b.toString(16).padStart(2, "0"))
    .join("");
}

// ============================================================================
// RFC 7748 Section 6.1 — Test Vectors
// ============================================================================

describe("X25519", () => {
  // -------------------------------------------------------------------------
  // Test Vector 1
  // -------------------------------------------------------------------------
  // This is the first test vector from RFC 7748. It tests a generic scalar
  // multiplication with arbitrary inputs (not the base point).

  test("should compute RFC 7748 test vector 1", () => {
    const scalar = hexToBytes(
      "a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4",
    );
    const u = hexToBytes(
      "e6db6867583030db3594c1a424b15f7c726624ec26b3353b10a903a6d0ab1c4c",
    );
    const expected =
      "c3da55379de9c6908e94ea4df28d084f32eccf03491c71f754b4075577a28552";

    const result = x25519(scalar, u);
    assert.equal(bytesToHex(result), expected);
  });

  // -------------------------------------------------------------------------
  // Test Vector 2
  // -------------------------------------------------------------------------
  // The second test vector from RFC 7748. Another generic scalar multiplication.

  test("should compute RFC 7748 test vector 2", () => {
    const scalar = hexToBytes(
      "4b66e9d4d1b4673c5ad22691957d6af5c11b6421e0ea01d42ca4169e7918ba0d",
    );
    const u = hexToBytes(
      "e5210f12786811d3f4b7959d0538ae2c31dbe7106fc03c3efc4cd549c715a493",
    );
    const expected =
      "95cbde9476e8907d7aade45cb4b873f88b595a68799fa152e6f8f7647aac7957";

    const result = x25519(scalar, u);
    assert.equal(bytesToHex(result), expected);
  });

  // -------------------------------------------------------------------------
  // Base point multiplication — Alice's public key
  // -------------------------------------------------------------------------
  // Alice generates her public key by multiplying her private key by the
  // base point (u = 9). This tests x25519Base.

  test("should generate Alice's public key from base point", () => {
    const alicePrivate = hexToBytes(
      "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a",
    );
    const expected =
      "8520f0098930a754748b7ddcb43ef75a0dbf3a0d26381af4eba4a98eaa9b4e6a";

    const result = x25519Base(alicePrivate);
    assert.equal(bytesToHex(result), expected);
  });

  // -------------------------------------------------------------------------
  // Base point multiplication — Bob's public key
  // -------------------------------------------------------------------------

  test("should generate Bob's public key from base point", () => {
    const bobPrivate = hexToBytes(
      "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb",
    );
    const expected =
      "de9edb7d7b7dc1b4d35b61c2ece435373f8343c85b78674dadfc7e146f882b4f";

    const result = x25519Base(bobPrivate);
    assert.equal(bytesToHex(result), expected);
  });

  // -------------------------------------------------------------------------
  // Diffie-Hellman key exchange: shared secret
  // -------------------------------------------------------------------------
  // The whole point of X25519: Alice and Bob compute the same shared secret
  // by combining their private key with the other party's public key.
  //
  //   Alice: secret = x25519(alice_private, bob_public)
  //   Bob:   secret = x25519(bob_private, alice_public)
  //
  // Both must produce the same value. This is the fundamental property of
  // Diffie-Hellman: [a]([b]G) = [b]([a]G) = [ab]G.

  test("should compute matching shared secrets (Diffie-Hellman)", () => {
    const alicePrivate = hexToBytes(
      "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a",
    );
    const bobPrivate = hexToBytes(
      "5dab087e624a8a4b79e17f8b83800ee66f3bb1292618b6fd1c2f8b27ff88e0eb",
    );
    const alicePublic = x25519Base(alicePrivate);
    const bobPublic = x25519Base(bobPrivate);

    const aliceShared = x25519(alicePrivate, bobPublic);
    const bobShared = x25519(bobPrivate, alicePublic);

    const expectedShared =
      "4a5d9d5ba4ce2de1728e3bf480350f25e07e21c947d19e3376f09b3c1e161742";

    assert.equal(bytesToHex(aliceShared), expectedShared);
    assert.equal(bytesToHex(bobShared), expectedShared);
    assert.equal(bytesToHex(aliceShared), bytesToHex(bobShared));
  });

  // -------------------------------------------------------------------------
  // generateKeypair is an alias for x25519Base
  // -------------------------------------------------------------------------

  test("should work with generateKeypair (alias for x25519Base)", () => {
    const privateKey = hexToBytes(
      "77076d0a7318a57d3c16c17251b26645df4c2f87ebc0992ab177fba51db92c2a",
    );
    const fromBase = x25519Base(privateKey);
    const fromKeypair = generateKeypair(privateKey);
    assert.equal(bytesToHex(fromKeypair), bytesToHex(fromBase));
  });

  // -------------------------------------------------------------------------
  // Iterated test: 1 iteration
  // -------------------------------------------------------------------------
  // RFC 7748 defines an iterated test where we repeatedly apply x25519:
  //   Start with k = u = 9 (as 32-byte LE)
  //   Repeat: (k, u) = (x25519(k, u), k)
  //
  // After 1 iteration, the result should match the given vector.

  test("should pass iterated test after 1 iteration", () => {
    let k: Uint8Array = new Uint8Array(32);
    k[0] = 9;
    let u = new Uint8Array(32);
    u[0] = 9;

    const oldK = new Uint8Array(k);
    k = x25519(k, u);
    u = oldK;

    assert.equal(
      bytesToHex(k),
      "422c8e7a6227d7bca1350b3e2bb7279f7897b87bb6854b783c60e80311ae3079",
    );
  });

  // -------------------------------------------------------------------------
  // Iterated test: 1000 iterations
  // -------------------------------------------------------------------------
  // After 1000 iterations of the same process, we must reach a specific value.
  // This is a thorough stress test of the field arithmetic.

  test("should pass iterated test after 1000 iterations", { timeout: 15_000 }, () => {
    let k: Uint8Array = new Uint8Array(32);
    k[0] = 9;
    let u = new Uint8Array(32);
    u[0] = 9;

    for (let i = 0; i < 1000; i++) {
      const oldK = new Uint8Array(k);
      k = x25519(k, u);
      u = oldK;
    }

    assert.equal(
      bytesToHex(k),
      "684cf59ba83309552800ef566f2f4d3c1c3887c49360e3875f2eb94d99532c51",
    );
  });

  // -------------------------------------------------------------------------
  // Input validation
  // -------------------------------------------------------------------------

  test("should reject scalar of wrong length", () => {
    assert.throws(
      () => x25519(new Uint8Array(16), new Uint8Array(32)),
      /Scalar must be exactly 32 bytes/,
    );
  });

  test("should reject u-coordinate of wrong length", () => {
    assert.throws(
      () => x25519(new Uint8Array(32), new Uint8Array(16)),
      /U-coordinate must be exactly 32 bytes/,
    );
  });

  // -------------------------------------------------------------------------
  // Edge case: u = 0 should produce all-zero output (and throw)
  // -------------------------------------------------------------------------
  // The point with u = 0 is the identity element. Multiplying any scalar
  // by it yields the identity, which encodes as all zeros.

  test("should throw on all-zero output (low-order point)", () => {
    const scalar = hexToBytes(
      "a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4",
    );
    const u = new Uint8Array(32); // u = 0

    assert.throws(() => x25519(scalar, u), /all-zero output/);
  });

  // -------------------------------------------------------------------------
  // Edge case: u = 1 is a low-order point
  // -------------------------------------------------------------------------
  // u = 1 is (0,0) on the curve, which is a point of order 2 in the
  // small subgroup. Scalar multiplication by a clamped scalar (multiple
  // of 8) maps it to the identity, producing all zeros.

  test("should throw on u = 1 (low-order point)", () => {
    const scalar = hexToBytes(
      "a546e36bf0527c9d3b16154b82465edd62144c0ac1fc5a18506a2244ba449ac4",
    );
    const u = new Uint8Array(32);
    u[0] = 1;

    assert.throws(() => x25519(scalar, u), /all-zero output/);
  });
});
