import { describe, it, expect } from "vitest";
import {
  hmac,
  hmacMD5, hmacMD5Hex,
  hmacSHA1, hmacSHA1Hex,
  hmacSHA256, hmacSHA256Hex,
  hmacSHA512, hmacSHA512Hex,
  toHex,
} from "../src/index.js";
import { sha256 } from "@coding-adventures/sha256";

const enc = new TextEncoder();

// ─── RFC 4231 — HMAC-SHA256 ───────────────────────────────────────────────────

describe("HMAC-SHA256 (RFC 4231)", () => {
  it("TC1: 20-byte key, 'Hi There'", () => {
    const key = new Uint8Array(20).fill(0x0b);
    expect(hmacSHA256Hex(key, enc.encode("Hi There"))).toBe(
      "b0344c61d8db38535ca8afceaf0bf12b881dc200c9833da726e9376c2e32cff7",
    );
  });

  it("TC2: 'Jefe', 'what do ya want for nothing?'", () => {
    expect(
      hmacSHA256Hex(enc.encode("Jefe"), enc.encode("what do ya want for nothing?")),
    ).toBe("5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843");
  });

  it("TC3: 20-byte key (0xaa), 50-byte data (0xdd)", () => {
    const key = new Uint8Array(20).fill(0xaa);
    const data = new Uint8Array(50).fill(0xdd);
    expect(hmacSHA256Hex(key, data)).toBe(
      "773ea91e36800e46854db8ebd09181a72959098b3ef8c122d9635514ced565fe",
    );
  });

  it("TC6: 131-byte key (longer than block size)", () => {
    const key = new Uint8Array(131).fill(0xaa);
    expect(
      hmacSHA256Hex(key, enc.encode("Test Using Larger Than Block-Size Key - Hash Key First")),
    ).toBe("60e431591ee0b67f0d8a26aacbf5b77f8e0bc6213728c5140546040f0ee37f54");
  });

  it("TC7: 131-byte key, large data", () => {
    const key = new Uint8Array(131).fill(0xaa);
    const data = enc.encode(
      "This is a test using a larger than block-size key and a larger than block-size data. " +
        "The key needs to be hashed before being used by the HMAC algorithm.",
    );
    expect(hmacSHA256Hex(key, data)).toBe(
      "9b09ffa71b942fcb27635fbcd5b0e944bfdc63644f0713938a7f51535c3a35e2",
    );
  });
});

// ─── RFC 4231 — HMAC-SHA512 ───────────────────────────────────────────────────

describe("HMAC-SHA512 (RFC 4231)", () => {
  it("TC1: 20-byte key, 'Hi There'", () => {
    const key = new Uint8Array(20).fill(0x0b);
    expect(hmacSHA512Hex(key, enc.encode("Hi There"))).toBe(
      "87aa7cdea5ef619d4ff0b4241a1d6cb02379f4e2ce4ec2787ad0b30545e17cdedaa833b7d6b8a702038b274eaea3f4e4be9d914eeb61f1702e696c203a126854",
    );
  });

  it("TC2: 'Jefe', 'what do ya want for nothing?'", () => {
    expect(
      hmacSHA512Hex(enc.encode("Jefe"), enc.encode("what do ya want for nothing?")),
    ).toBe(
      "164b7a7bfcf819e2e395fbe73b56e0a387bd64222e831fd610270cd7ea2505549758bf75c05a994a6d034f65f8f0e6fdcaeab1a34d4a6b4b636e070a38bce737",
    );
  });

  it("TC6: 131-byte key (longer than block size)", () => {
    const key = new Uint8Array(131).fill(0xaa);
    expect(
      hmacSHA512Hex(key, enc.encode("Test Using Larger Than Block-Size Key - Hash Key First")),
    ).toBe(
      "80b24263c7c1a3ebb71493c1dd7be8b49b46d1f41b4aeec1121b013783f8f3526b56d037e05f2598bd0fd2215d6a1e5295e64f73f63f0aec8b915a985d786598",
    );
  });
});

// ─── RFC 2202 — HMAC-MD5 ─────────────────────────────────────────────────────

describe("HMAC-MD5 (RFC 2202)", () => {
  it("TC1: 16-byte key (0x0b), 'Hi There'", () => {
    const key = new Uint8Array(16).fill(0x0b);
    expect(hmacMD5Hex(key, enc.encode("Hi There"))).toBe(
      "9294727a3638bb1c13f48ef8158bfc9d",
    );
  });

  it("TC2: 'Jefe', 'what do ya want for nothing?'", () => {
    expect(
      hmacMD5Hex(enc.encode("Jefe"), enc.encode("what do ya want for nothing?")),
    ).toBe("750c783e6ab0b503eaa86e310a5db738");
  });

  it("TC6: 80-byte key (longer than block size)", () => {
    const key = new Uint8Array(80).fill(0xaa);
    expect(
      hmacMD5Hex(key, enc.encode("Test Using Larger Than Block-Size Key - Hash Key First")),
    ).toBe("6b1ab7fe4bd7bf8f0b62e6ce61b9d0cd");
  });
});

// ─── RFC 2202 — HMAC-SHA1 ────────────────────────────────────────────────────

describe("HMAC-SHA1 (RFC 2202)", () => {
  it("TC1: 20-byte key (0x0b), 'Hi There'", () => {
    const key = new Uint8Array(20).fill(0x0b);
    expect(hmacSHA1Hex(key, enc.encode("Hi There"))).toBe(
      "b617318655057264e28bc0b6fb378c8ef146be00",
    );
  });

  it("TC2: 'Jefe', 'what do ya want for nothing?'", () => {
    expect(
      hmacSHA1Hex(enc.encode("Jefe"), enc.encode("what do ya want for nothing?")),
    ).toBe("effcdf6ae5eb2fa2d27416d5f184df9c259a7c79");
  });

  it("TC6: 80-byte key (longer than block size)", () => {
    const key = new Uint8Array(80).fill(0xaa);
    expect(
      hmacSHA1Hex(key, enc.encode("Test Using Larger Than Block-Size Key - Hash Key First")),
    ).toBe("aa4ae5e15272d00e95705637ce8a3b55ed402112");
  });
});

// ─── Return lengths ───────────────────────────────────────────────────────────

describe("return lengths", () => {
  const k = enc.encode("k");
  const m = enc.encode("m");

  it("HMAC-MD5 → 16 bytes", () => expect(hmacMD5(k, m).length).toBe(16));
  it("HMAC-SHA1 → 20 bytes", () => expect(hmacSHA1(k, m).length).toBe(20));
  it("HMAC-SHA256 → 32 bytes", () => expect(hmacSHA256(k, m).length).toBe(32));
  it("HMAC-SHA512 → 64 bytes", () => expect(hmacSHA512(k, m).length).toBe(64));
});

// ─── Key handling ─────────────────────────────────────────────────────────────

describe("key handling", () => {
  it("empty key throws for SHA-256", () => {
    expect(() => hmacSHA256(new Uint8Array(0), new Uint8Array(0))).toThrow(
      "HMAC key must not be empty"
    );
  });

  it("empty key throws for SHA-512", () => {
    expect(() => hmacSHA512(new Uint8Array(0), new Uint8Array(0))).toThrow(
      "HMAC key must not be empty"
    );
  });

  it("empty message with non-empty key is allowed for SHA-256", () => {
    expect(hmacSHA256(new Uint8Array([1]), new Uint8Array(0)).length).toBe(32);
  });

  it("keys of different lengths beyond block size produce different tags", () => {
    const k65 = new Uint8Array(65).fill(0x01);
    const k66 = new Uint8Array(66).fill(0x01);
    const msg = enc.encode("msg");
    expect(toHex(hmacSHA256(k65, msg))).not.toBe(toHex(hmacSHA256(k66, msg)));
  });
});

// ─── Authentication properties ────────────────────────────────────────────────

describe("authentication properties", () => {
  const k = enc.encode("secret");
  const m = enc.encode("message");

  it("deterministic — same inputs always same output", () => {
    expect(toHex(hmacSHA256(k, m))).toBe(toHex(hmacSHA256(k, m)));
  });

  it("key sensitivity — different keys → different tags", () => {
    expect(toHex(hmacSHA256(enc.encode("k1"), m))).not.toBe(
      toHex(hmacSHA256(enc.encode("k2"), m)),
    );
  });

  it("message sensitivity — different messages → different tags", () => {
    expect(toHex(hmacSHA256(k, enc.encode("m1")))).not.toBe(
      toHex(hmacSHA256(k, enc.encode("m2"))),
    );
  });

  it("hex matches bytes", () => {
    const tag = hmacSHA256(k, m);
    expect(hmacSHA256Hex(k, m)).toBe(toHex(tag));
  });
});

// ─── Generic hmac() function ──────────────────────────────────────────────────

describe("generic hmac()", () => {
  it("produces same result as hmacSHA256 when called with sha256", () => {
    const k = enc.encode("key");
    const m = enc.encode("msg");
    const fromGeneric = toHex(hmac(sha256, 64, k, m));
    const fromNamed = hmacSHA256Hex(k, m);
    expect(fromGeneric).toBe(fromNamed);
  });
});
