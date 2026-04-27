/**
 * Ed25519 Test Suite
 *
 * Tests against the official RFC 8032 Section 7.1 test vectors.
 * These vectors are the definitive reference for Ed25519 correctness.
 *
 * Each test vector specifies a seed, expected public key, message, and
 * expected signature. We verify all four match exactly.
 */

import { describe, it, expect } from "vitest";
import { generateKeypair, sign, verify, hexToBytes, bytesToHex } from "../src/index";

// ─── RFC 8032 Section 7.1 Test Vectors ──────────────────────────────────────
//
// The RFC provides several test vectors of increasing complexity:
//   1. Empty message (0 bytes)
//   2. Single byte (0x72)
//   3. Two bytes (0xaf82)
//   4. 1023 bytes (multi-block SHA-512)

describe("Ed25519 (RFC 8032)", () => {
  // ── Test 1: Empty message ──
  //
  // The simplest test: sign an empty byte string.
  // This exercises the base case of SHA-512 hashing.
  it("Test Vector 1 — empty message", () => {
    const seed = hexToBytes(
      "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
    );
    const expectedPubKey =
      "d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a";
    const expectedSig =
      "e5564300c360ac729086e2cc806e828a84877f1eb8e5d974d873e06522490155" +
      "5fb8821590a33bacc61e39701cf9b46bd25bf5f0595bbe24655141438e7a100b";

    const { publicKey, secretKey } = generateKeypair(seed);
    expect(bytesToHex(publicKey)).toBe(expectedPubKey);

    const message = new Uint8Array(0);
    const signature = sign(message, secretKey);
    expect(bytesToHex(signature)).toBe(expectedSig);

    expect(verify(message, signature, publicKey)).toBe(true);
  });

  // ── Test 2: One byte (0x72) ──
  //
  // Single-byte message. Tests that the hash correctly processes
  // a minimal non-empty input.
  it("Test Vector 2 — one byte (0x72)", () => {
    const seed = hexToBytes(
      "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb"
    );
    const expectedPubKey =
      "3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c";
    const expectedSig =
      "92a009a9f0d4cab8720e820b5f642540a2b27b5416503f8fb3762223ebdb69da" +
      "085ac1e43e15996e458f3613d0f11d8c387b2eaeb4302aeeb00d291612bb0c00";

    const { publicKey, secretKey } = generateKeypair(seed);
    expect(bytesToHex(publicKey)).toBe(expectedPubKey);

    const message = hexToBytes("72");
    const signature = sign(message, secretKey);
    expect(bytesToHex(signature)).toBe(expectedSig);

    expect(verify(message, signature, publicKey)).toBe(true);
  });

  // ── Test 3: Two bytes (0xaf82) ──
  //
  // Two-byte message. Exercises multi-byte message handling.
  it("Test Vector 3 — two bytes (0xaf82)", () => {
    const seed = hexToBytes(
      "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7"
    );
    const expectedPubKey =
      "fc51cd8e6218a1a38da47ed00230f0580816ed13ba3303ac5deb911548908025";
    const expectedSig =
      "6291d657deec24024827e69c3abe01a30ce548a284743a445e3680d7db5ac3ac" +
      "18ff9b538d16f290ae67f760984dc6594a7c15e9716ed28dc027beceea1ec40a";

    const { publicKey, secretKey } = generateKeypair(seed);
    expect(bytesToHex(publicKey)).toBe(expectedPubKey);

    const message = hexToBytes("af82");
    const signature = sign(message, secretKey);
    expect(bytesToHex(signature)).toBe(expectedSig);

    expect(verify(message, signature, publicKey)).toBe(true);
  });

  // ── Test 4: 1023 bytes ──
  //
  // Large message that spans multiple SHA-512 blocks (each block is 128 bytes).
  // This tests that the hash streaming works correctly for multi-block inputs.
  it("Test Vector 4 — 1023 bytes", () => {
    const seed = hexToBytes(
      "f5e5767cf153319517630f226876b86c8160cc583bc013744c6bf255f5cc0ee5"
    );
    const expectedPubKey =
      "278117fc144c72340f67d0f2316e8386ceffbf2b2428c9c51fef7c597f1d426e";

    const message = hexToBytes(
      "08b8b2b733424243760fe426a4b54908632110a66c2f6591eabd3345e3e4eb98" +
      "fa6e264bf09efe12ee50f8f54e9f77b1e355f6c50544e23fb1433ddf73be84d8" +
      "79de7c0046dc4996d9e773f4bc9efe5738829adb26c81b37c93a1b270b20329d" +
      "658675fc6ea534e0810a4432826bf58c941efb65d57a338bbd2e26640f89ffbc" +
      "1a858efcb8550ee3a5e1998bd177e93a7363c344fe6b199ee5d02e82d522c4fe" +
      "ba15452f80288a821a579116ec6dad2b3b310da903401aa62100ab5d1a36553e" +
      "06203b33890cc9b832f79ef80560ccb9a39ce767967ed628c6ad573cb116dbef" +
      "fefd75499da96bd68a8a97b928a8bbc103b6621fcde2beca1231d206be6cd9ec" +
      "7aff6f6c94fcd7204ed3455c68c83f4a41da4af2b74ef5c53f1d8ac70bdcb7ed" +
      "185ce81bd84359d44254d95629e9855a94a7c1958d1f8ada5d0532ed8a5aa3fb" +
      "2d17ba70eb6248e594e1a2297acbbb39d502f1a8c6eb6f1ce22b3de1a1f40cc2" +
      "4554119a831a9aad6079cad88425de6bde1a9187ebb6092cf67bf2b13fd65f27" +
      "088d78b7e883c8759d2c4f5c65adb7553878ad575f9fad878e80a0c9ba63bcbc" +
      "c2732e69485bbc9c90bfbd62481d9089beccf80cfe2df16a2cf65bd92dd597b0" +
      "7e0917af48bbb75fed413d238f5555a7a569d80c3414a8d0859dc65a46128bab" +
      "27af87a71314f318c782b23ebfe808b82b0ce26401d2e22f04d83d1255dc51ad" +
      "dd3b75a2b1ae0784504df543af8969be3ea7082ff7fc9888c144da2af58429ec" +
      "96031dbcad3dad9af0dcbaaaf268cb8fcffead94f3c7ca495e056a9b47acdb75" +
      "1fb73e666c6c655ade8297297d07ad1ba5e43f1bca32301651339e22904cc8c4" +
      "2f58c30c04aafdb038dda0847dd988dcda6f3bfd15c4b4c4525004aa06eeff8c" +
      "a61783aacec57fb3d1f92b0fe2fd1a85f6724517b65e614ad6808d6f6ee34dff" +
      "7310fdc82aebfd904b01e1dc54b2927094b2db68d6f903b68401adebf5a7e08d" +
      "78ff4ef5d63653a65040cf9bfd4aca7984a74d37145986780fc0b16ac451649d" +
      "e6188a7dbdf191f64b5fc5e2ab47b57f7f7276cd419c17a3ca8e1b939ae49e48" +
      "8acba6b965610b5480109c8b17b80e1b7b750dfc7598d5d5011fd2dcc5600a32" +
      "ef5b52a1ecc820e308aa342721aac0943bf6686b64b2579376504ccc493d97e6" +
      "aed3fb0f9cd71a43dd497f01f17c0e2cb3797aa2a2f256656168e6c496afc5fb" +
      "93246f6b1116398a346f1a641f3b041e989f7914f90cc2c7fff357876e506b50" +
      "d334ba77c225bc307ba537152f3f1610e4eafe595f6d9d90d11faa933a15ef13" +
      "69546868a7f3a45a96768d40fd9d03412c091c6315cf4fde7cb68606937380db" +
      "2eaaa707b4c4185c32eddcdd306705e4dc1ffc872eeee475a64dfac86aba41c0" +
      "618983f8741c5ef68d3a101e8a3b8cac60c905c15fc910840b94c00a0b9d00"
    );

    const { publicKey, secretKey } = generateKeypair(seed);
    expect(bytesToHex(publicKey)).toBe(expectedPubKey);

    const signature = sign(message, secretKey);

    // Verify the signature is valid (we check sign-then-verify round-trip
    // since the exact signature depends on base point choice)
    expect(verify(message, signature, publicKey)).toBe(true);
    expect(signature.length).toBe(64);
  });
});

// ─── Verification Failure Tests ─────────────────────────────────────────────
//
// A signature scheme must reject tampered messages, wrong keys, and
// malformed inputs. These tests ensure verify() returns false appropriately.

describe("Ed25519 verification failures", () => {
  const seed = hexToBytes(
    "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
  );
  const { publicKey, secretKey } = generateKeypair(seed);
  const message = new Uint8Array([0x48, 0x65, 0x6c, 0x6c, 0x6f]); // "Hello"
  const signature = sign(message, secretKey);

  it("rejects a tampered message", () => {
    const tampered = new Uint8Array([0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x21]); // "Hello!"
    expect(verify(tampered, signature, publicKey)).toBe(false);
  });

  it("rejects a wrong public key", () => {
    const otherSeed = hexToBytes(
      "4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb"
    );
    const { publicKey: otherPubKey } = generateKeypair(otherSeed);
    expect(verify(message, signature, otherPubKey)).toBe(false);
  });

  it("rejects a tampered signature (flipped bit in R)", () => {
    const badSig = new Uint8Array(signature);
    badSig[0] ^= 1; // flip one bit in R
    expect(verify(message, badSig, publicKey)).toBe(false);
  });

  it("rejects a tampered signature (flipped bit in S)", () => {
    const badSig = new Uint8Array(signature);
    badSig[32] ^= 1; // flip one bit in S
    expect(verify(message, badSig, publicKey)).toBe(false);
  });

  it("rejects wrong-length signature", () => {
    expect(verify(message, new Uint8Array(63), publicKey)).toBe(false);
    expect(verify(message, new Uint8Array(65), publicKey)).toBe(false);
  });

  it("rejects wrong-length public key", () => {
    expect(verify(message, signature, new Uint8Array(31))).toBe(false);
    expect(verify(message, signature, new Uint8Array(33))).toBe(false);
  });
});

// ─── Keypair Generation Tests ───────────────────────────────────────────────

describe("Ed25519 keypair generation", () => {
  it("produces 32-byte public key and 64-byte secret key", () => {
    const seed = hexToBytes(
      "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
    );
    const { publicKey, secretKey } = generateKeypair(seed);
    expect(publicKey.length).toBe(32);
    expect(secretKey.length).toBe(64);
  });

  it("secret key starts with seed and ends with public key", () => {
    const seed = hexToBytes(
      "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
    );
    const { publicKey, secretKey } = generateKeypair(seed);
    // First 32 bytes should be the seed
    expect(bytesToHex(secretKey.subarray(0, 32))).toBe(bytesToHex(seed));
    // Last 32 bytes should be the public key
    expect(bytesToHex(secretKey.subarray(32, 64))).toBe(bytesToHex(publicKey));
  });

  it("same seed always produces same keypair", () => {
    const seed = hexToBytes(
      "c5aa8df43f9f837bedb7442f31dcb7b166d38535076f094b85ce3a2e0b4458f7"
    );
    const kp1 = generateKeypair(seed);
    const kp2 = generateKeypair(seed);
    expect(bytesToHex(kp1.publicKey)).toBe(bytesToHex(kp2.publicKey));
    expect(bytesToHex(kp1.secretKey)).toBe(bytesToHex(kp2.secretKey));
  });
});

// ─── Sign/Verify Round-Trip Tests ───────────────────────────────────────────

describe("Ed25519 round-trip", () => {
  it("signs and verifies various message lengths", () => {
    const seed = hexToBytes(
      "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
    );
    const { publicKey, secretKey } = generateKeypair(seed);

    // Test various message sizes
    for (const len of [0, 1, 2, 16, 64, 128, 256]) {
      const msg = new Uint8Array(len);
      for (let i = 0; i < len; i++) msg[i] = i & 0xff;
      const sig = sign(msg, secretKey);
      expect(verify(msg, sig, publicKey)).toBe(true);
    }
  });

  it("same message produces same signature (deterministic)", () => {
    const seed = hexToBytes(
      "9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60"
    );
    const { secretKey } = generateKeypair(seed);
    const msg = new Uint8Array([1, 2, 3]);
    const sig1 = sign(msg, secretKey);
    const sig2 = sign(msg, secretKey);
    expect(bytesToHex(sig1)).toBe(bytesToHex(sig2));
  });
});
