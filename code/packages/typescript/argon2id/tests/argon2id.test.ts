import { describe, expect, it } from "vitest";
import { argon2id, argon2idHex } from "../src/index.js";

// Build a ``len``-byte Uint8Array filled with ``byte``.
function filled(len: number, byte: number): Uint8Array {
  return new Uint8Array(len).fill(byte);
}

function hexToBytes(hex: string): Uint8Array {
  const out = new Uint8Array(hex.length / 2);
  for (let i = 0; i < out.length; i++) {
    out[i] = parseInt(hex.slice(i * 2, i * 2 + 2), 16);
  }
  return out;
}

function bytesToHex(bytes: Uint8Array): string {
  let hex = "";
  for (const b of bytes) hex += b.toString(16).padStart(2, "0");
  return hex;
}

// --- RFC 9106 §5.3 canonical test vector ----------------------------------

const RFC_PASSWORD = filled(32, 0x01);
const RFC_SALT = filled(16, 0x02);
const RFC_KEY = filled(8, 0x03);
const RFC_AD = filled(12, 0x04);
const RFC_EXPECTED = hexToBytes(
  "0d640df58d78766c08c037a34a8b53c9d01ef0452d75b65eb52520e96b01e659",
);

describe("argon2id — RFC 9106 §5.3", () => {
  it("matches the canonical RFC vector", () => {
    const tag = argon2id(
      RFC_PASSWORD,
      RFC_SALT,
      3, 32, 4, 32,
      { key: RFC_KEY, associatedData: RFC_AD },
    );
    expect(bytesToHex(tag)).toBe(bytesToHex(RFC_EXPECTED));
  });

  it("hex form matches bytes form", () => {
    const tag = argon2id(RFC_PASSWORD, RFC_SALT, 3, 32, 4, 32);
    const hex = argon2idHex(RFC_PASSWORD, RFC_SALT, 3, 32, 4, 32);
    expect(hex).toBe(bytesToHex(tag));
  });
});

// --- Parameter validation -------------------------------------------------

describe("argon2id — parameter validation", () => {
  const pw = new Uint8Array([0x70, 0x77]);
  const salt8 = new TextEncoder().encode("saltsalt");

  it("rejects short salt", () => {
    expect(() => argon2id(pw, new TextEncoder().encode("short"), 1, 8, 1, 32))
      .toThrow(/salt/);
  });

  it("rejects tag_length < 4", () => {
    expect(() => argon2id(pw, salt8, 1, 8, 1, 3)).toThrow(/tagLength/);
  });

  it("rejects memory below 8*p", () => {
    expect(() => argon2id(pw, salt8, 1, 1, 1, 32)).toThrow(/memoryCost/);
  });

  it("rejects time_cost = 0", () => {
    expect(() => argon2id(pw, salt8, 0, 8, 1, 32)).toThrow(/timeCost/);
  });

  it("rejects parallelism = 0", () => {
    expect(() => argon2id(pw, salt8, 1, 8, 0, 32)).toThrow(/parallelism/);
  });

  it("rejects unsupported version", () => {
    expect(() => argon2id(pw, salt8, 1, 8, 1, 32, { version: 0x10 }))
      .toThrow(/v1\.3/);
  });

  it("rejects non-Uint8Array password", () => {
    expect(() => argon2id("not bytes" as unknown as Uint8Array, salt8, 1, 8, 1, 32))
      .toThrow(/password/);
  });
});

// --- Determinism and field-bind sanity checks -----------------------------

describe("argon2id — deterministic sanity", () => {
  const pw = new TextEncoder().encode("password");
  const salt = new TextEncoder().encode("somesalt");

  it("is deterministic for fixed inputs", () => {
    const a = argon2id(pw, salt, 1, 8, 1, 32);
    const b = argon2id(pw, salt, 1, 8, 1, 32);
    expect(bytesToHex(a)).toBe(bytesToHex(b));
    expect(a.length).toBe(32);
  });

  it("differs on password change", () => {
    const a = argon2id(new TextEncoder().encode("password1"), salt, 1, 8, 1, 32);
    const b = argon2id(new TextEncoder().encode("password2"), salt, 1, 8, 1, 32);
    expect(bytesToHex(a)).not.toBe(bytesToHex(b));
  });

  it("differs on salt change", () => {
    const a = argon2id(pw, new TextEncoder().encode("saltsalt"), 1, 8, 1, 32);
    const b = argon2id(pw, new TextEncoder().encode("saltsal2"), 1, 8, 1, 32);
    expect(bytesToHex(a)).not.toBe(bytesToHex(b));
  });

  it("is bound to the key", () => {
    const a = argon2id(pw, new TextEncoder().encode("saltsalt"), 1, 8, 1, 32);
    const b = argon2id(pw, new TextEncoder().encode("saltsalt"), 1, 8, 1, 32,
      { key: new TextEncoder().encode("secret!!") });
    expect(bytesToHex(a)).not.toBe(bytesToHex(b));
  });

  it("is bound to the associated data", () => {
    const a = argon2id(pw, new TextEncoder().encode("saltsalt"), 1, 8, 1, 32);
    const b = argon2id(pw, new TextEncoder().encode("saltsalt"), 1, 8, 1, 32,
      { associatedData: new TextEncoder().encode("ad") });
    expect(bytesToHex(a)).not.toBe(bytesToHex(b));
  });
});

// --- Tag-length variability -----------------------------------------------

describe("argon2id — tag length", () => {
  const pw = new TextEncoder().encode("password");
  const salt = new TextEncoder().encode("saltsalt");

  for (const T of [4, 16, 32, 64, 65, 128]) {
    it(`emits exactly ${T} bytes`, () => {
      const tag = argon2id(pw, salt, 1, 8, 1, T);
      expect(tag.length).toBe(T);
    });
  }
});

// --- Multi-lane / multiple passes -----------------------------------------

describe("argon2id — edges", () => {
  it("works with p=4, m=32", () => {
    const tag = argon2id(RFC_PASSWORD, RFC_SALT, 3, 32, 4, 32);
    expect(tag.length).toBe(32);
  });

  it("distinct tags across t=1,2,3", () => {
    const pw = new TextEncoder().encode("password");
    const salt = new TextEncoder().encode("saltsalt");
    const t1 = bytesToHex(argon2id(pw, salt, 1, 8, 1, 32));
    const t2 = bytesToHex(argon2id(pw, salt, 2, 8, 1, 32));
    const t3 = bytesToHex(argon2id(pw, salt, 3, 8, 1, 32));
    expect(new Set([t1, t2, t3]).size).toBe(3);
  });
});
