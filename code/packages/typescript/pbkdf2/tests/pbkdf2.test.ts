import { describe, it, expect } from "vitest";
import {
  pbkdf2HmacSHA1,
  pbkdf2HmacSHA1Hex,
  pbkdf2HmacSHA256,
  pbkdf2HmacSHA256Hex,
  pbkdf2HmacSHA512,
  pbkdf2HmacSHA512Hex,
} from "../src/index.js";

// Helper: encode a string to Uint8Array using UTF-8.
const enc = (s: string) => new TextEncoder().encode(s);

// Helper: encode a hex string to Uint8Array.
const fromHex = (s: string) =>
  new Uint8Array(s.match(/.{2}/g)!.map((b) => parseInt(b, 16)));

// ─────────────────────────────────────────────────────────────────────────────
// RFC 6070 — PBKDF2-HMAC-SHA1
// ─────────────────────────────────────────────────────────────────────────────

describe("RFC 6070 PBKDF2-HMAC-SHA1", () => {
  it("vector 1 — c=1, dkLen=20", () => {
    const dk = pbkdf2HmacSHA1(enc("password"), enc("salt"), 1, 20);
    expect(Array.from(dk)).toEqual(
      Array.from(fromHex("0c60c80f961f0e71f3a9b524af6012062fe037a6")),
    );
  });

  it("vector 2 — c=4096, dkLen=20", () => {
    const dk = pbkdf2HmacSHA1(enc("password"), enc("salt"), 4096, 20);
    expect(Array.from(dk)).toEqual(
      Array.from(fromHex("4b007901b765489abead49d926f721d065a429c1")),
    );
  });

  it("vector 3 — long password and salt", () => {
    const dk = pbkdf2HmacSHA1(
      enc("passwordPASSWORDpassword"),
      enc("saltSALTsaltSALTsaltSALTsaltSALTsalt"),
      4096,
      25,
    );
    expect(Array.from(dk)).toEqual(
      Array.from(
        fromHex("3d2eec4fe41c849b80c8d83662c0e44a8b291a964cf2f07038"),
      ),
    );
  });

  it("vector 4 — null bytes in password and salt", () => {
    const password = new Uint8Array([112, 97, 115, 115, 0, 119, 111, 114, 100]); // "pass\0word"
    const salt = new Uint8Array([115, 97, 0, 108, 116]); // "sa\0lt"
    const dk = pbkdf2HmacSHA1(password, salt, 4096, 16);
    expect(Array.from(dk)).toEqual(
      Array.from(fromHex("56fa6aa75548099dcc37d7f03425e0c3")),
    );
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// RFC 7914 — PBKDF2-HMAC-SHA256
// ─────────────────────────────────────────────────────────────────────────────

describe("RFC 7914 PBKDF2-HMAC-SHA256", () => {
  it("vector 1 — c=1, dkLen=64", () => {
    const dk = pbkdf2HmacSHA256(enc("passwd"), enc("salt"), 1, 64);
    const expected = fromHex(
      "55ac046e56e3089fec1691c22544b605" +
        "f94185216dde0465e68b9d57c20dacbc" +
        "49ca9cccf179b645991664b39d77ef31" +
        "7c71b845b1e30bd509112041d3a19783",
    );
    expect(Array.from(dk)).toEqual(Array.from(expected));
  });

  it("output length matches requested keyLength", () => {
    const dk = pbkdf2HmacSHA256(enc("key"), enc("salt"), 1, 32);
    expect(dk.length).toBe(32);
  });

  it("truncation is consistent with prefix of longer key", () => {
    const short = pbkdf2HmacSHA256(enc("key"), enc("salt"), 1, 16);
    const full = pbkdf2HmacSHA256(enc("key"), enc("salt"), 1, 32);
    expect(Array.from(short)).toEqual(Array.from(full.slice(0, 16)));
  });

  it("multi-block: first 32 bytes match single-block result", () => {
    const dk64 = pbkdf2HmacSHA256(enc("password"), enc("salt"), 1, 64);
    const dk32 = pbkdf2HmacSHA256(enc("password"), enc("salt"), 1, 32);
    expect(dk64.length).toBe(64);
    expect(Array.from(dk64.slice(0, 32))).toEqual(Array.from(dk32));
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// SHA-512 sanity checks
// ─────────────────────────────────────────────────────────────────────────────

describe("PBKDF2-HMAC-SHA512", () => {
  it("output length", () => {
    expect(pbkdf2HmacSHA512(enc("secret"), enc("nacl"), 1, 64).length).toBe(64);
  });

  it("truncation consistent", () => {
    const short = pbkdf2HmacSHA512(enc("secret"), enc("nacl"), 1, 32);
    const full = pbkdf2HmacSHA512(enc("secret"), enc("nacl"), 1, 64);
    expect(Array.from(short)).toEqual(Array.from(full.slice(0, 32)));
  });

  it("multi-block 128 bytes", () => {
    const dk = pbkdf2HmacSHA512(enc("key"), enc("salt"), 1, 128);
    expect(dk.length).toBe(128);
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Hex variants
// ─────────────────────────────────────────────────────────────────────────────

describe("hex variants", () => {
  it("SHA1 hex matches RFC 6070 vector 1", () => {
    expect(pbkdf2HmacSHA1Hex(enc("password"), enc("salt"), 1, 20)).toBe(
      "0c60c80f961f0e71f3a9b524af6012062fe037a6",
    );
  });

  it("SHA256 hex matches bytes", () => {
    const dk = pbkdf2HmacSHA256(enc("passwd"), enc("salt"), 1, 32);
    const h = pbkdf2HmacSHA256Hex(enc("passwd"), enc("salt"), 1, 32);
    expect(h).toBe(Array.from(dk).map((b) => b.toString(16).padStart(2, "0")).join(""));
  });

  it("SHA512 hex matches bytes", () => {
    const dk = pbkdf2HmacSHA512(enc("secret"), enc("nacl"), 1, 64);
    const h = pbkdf2HmacSHA512Hex(enc("secret"), enc("nacl"), 1, 64);
    expect(h).toBe(Array.from(dk).map((b) => b.toString(16).padStart(2, "0")).join(""));
  });
});

// ─────────────────────────────────────────────────────────────────────────────
// Validation
// ─────────────────────────────────────────────────────────────────────────────

describe("validation", () => {
  it("empty password throws", () => {
    expect(() =>
      pbkdf2HmacSHA256(new Uint8Array(0), enc("salt"), 1, 32),
    ).toThrow("password must not be empty");
  });

  it("zero iterations throws", () => {
    expect(() => pbkdf2HmacSHA256(enc("pw"), enc("salt"), 0, 32)).toThrow(
      "iterations must be a positive integer",
    );
  });

  it("negative iterations throws", () => {
    expect(() => pbkdf2HmacSHA256(enc("pw"), enc("salt"), -1, 32)).toThrow(
      "iterations must be a positive integer",
    );
  });

  it("zero keyLength throws", () => {
    expect(() => pbkdf2HmacSHA256(enc("pw"), enc("salt"), 1, 0)).toThrow(
      "keyLength must be a positive integer",
    );
  });

  it("empty salt is allowed", () => {
    const dk = pbkdf2HmacSHA256(enc("password"), new Uint8Array(0), 1, 32);
    expect(dk.length).toBe(32);
  });

  it("is deterministic", () => {
    const a = pbkdf2HmacSHA256(enc("secret"), enc("nacl"), 100, 32);
    const b = pbkdf2HmacSHA256(enc("secret"), enc("nacl"), 100, 32);
    expect(Array.from(a)).toEqual(Array.from(b));
  });

  it("different salts produce different keys", () => {
    const a = pbkdf2HmacSHA256(enc("password"), enc("salt1"), 1, 32);
    const b = pbkdf2HmacSHA256(enc("password"), enc("salt2"), 1, 32);
    expect(Array.from(a)).not.toEqual(Array.from(b));
  });

  it("different passwords produce different keys", () => {
    const a = pbkdf2HmacSHA256(enc("password1"), enc("salt"), 1, 32);
    const b = pbkdf2HmacSHA256(enc("password2"), enc("salt"), 1, 32);
    expect(Array.from(a)).not.toEqual(Array.from(b));
  });

  it("different iterations produce different keys", () => {
    const a = pbkdf2HmacSHA256(enc("password"), enc("salt"), 1, 32);
    const b = pbkdf2HmacSHA256(enc("password"), enc("salt"), 2, 32);
    expect(Array.from(a)).not.toEqual(Array.from(b));
  });
});
