/**
 * hash.test.ts — SHA-256 base64 + UTF-8 byte length.
 */

import { describe, it, expect } from "vitest";
import { sha256Base64, utf8ByteLength } from "../src/index.js";

describe("sha256Base64", () => {
  it("empty string has known digest", () => {
    // SHA-256("") = 47DEQpj8HBSa-_TImW-5JCeuQeRkm5NMpJWZG3hSuFU= (URL-safe)
    // standard base64: 47DEQpj8HBSa+_TImW+5JCeuQeRkm5NMpJWZG3hSuFU=
    // Actual: 47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=
    expect(sha256Base64("")).toBe("47DEQpj8HBSa+/TImW+5JCeuQeRkm5NMpJWZG3hSuFU=");
  });
  it("'abc' has known digest", () => {
    // SHA-256("abc") = ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad (hex)
    // base64: ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=
    expect(sha256Base64("abc")).toBe("ungWv48Bz+pBQUDeXa4iI7ADYaOWF3qctBD/YfIAFa0=");
  });
  it("deterministic — same input → same output", () => {
    expect(sha256Base64("hello world")).toBe(sha256Base64("hello world"));
  });
  it("different inputs → different hashes", () => {
    expect(sha256Base64("a")).not.toBe(sha256Base64("b"));
  });
  it("output is 44 chars (SHA-256 base64 with padding)", () => {
    expect(sha256Base64("any string").length).toBe(44);
  });
});

describe("utf8ByteLength", () => {
  it("ASCII chars are 1 byte each", () => {
    expect(utf8ByteLength("hello")).toBe(5);
  });
  it("empty string is 0 bytes", () => {
    expect(utf8ByteLength("")).toBe(0);
  });
  it("multi-byte UTF-8 (é = 2 bytes)", () => {
    expect(utf8ByteLength("é")).toBe(2);
  });
  it("emoji (4 bytes)", () => {
    expect(utf8ByteLength("🎉")).toBe(4);
  });
  it("mixed", () => {
    expect(utf8ByteLength("hi 🎉")).toBe(3 + 4); // h + i + space + emoji
  });
});
