import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  hchacha20Subkey,
  xchacha20Encrypt,
  xchacha20Poly1305Decrypt,
  xchacha20Poly1305Encrypt,
} from "../src/index.js";

interface HChaCha20Case {
  id: string;
  key_hex: string;
  nonce_hex: string;
  subkey_hex: string;
}

interface XChaCha20Case {
  id: string;
  counter: number;
  key_hex: string;
  nonce_hex: string;
  input_hex: string;
  output_hex: string;
}

interface AeadCase {
  id: string;
  key_hex: string;
  nonce_hex: string;
  aad_hex: string;
  plaintext_hex: string;
  ciphertext_hex: string;
  tag_hex: string;
}

type MutationTarget = "ciphertext" | "key" | "nonce" | "aad" | "tag";

interface Mutation {
  source_case: string;
  target: MutationTarget;
  byte_indices: number[];
  xor_hex: string;
}

interface Fixture {
  schema_version: number;
  profile: string;
  authentication_failure: string;
  hchacha20_cases: HChaCha20Case[];
  xchacha20_cases: XChaCha20Case[];
  aead_cases: AeadCase[];
  mutations: Mutation[];
}

const fixture = JSON.parse(
  readFileSync(
    new URL(
      "../../../../specs/fixtures/se04-xchacha20-poly1305-v1/cases.json",
      import.meta.url,
    ),
    "utf8",
  ),
) as Fixture;

function fromHex(value: string): Uint8Array {
  if (value.length % 2 !== 0) {
    throw new Error("fixture hex must contain whole bytes");
  }
  return Uint8Array.from(
    value.match(/.{2}/g)?.map((byte) => Number.parseInt(byte, 16)) ?? [],
  );
}

describe("SE04 shared fixture", () => {
  it("has closed v1 metadata", () => {
    expect(fixture.schema_version).toBe(1);
    expect(fixture.profile).toBe("se04-xchacha20-poly1305-v1");
    expect(fixture.authentication_failure).toBe("authentication_failed");
    expect(fixture.hchacha20_cases).toHaveLength(1);
    expect(fixture.xchacha20_cases).toHaveLength(2);
    expect(fixture.aead_cases).toHaveLength(3);
    expect(fixture.mutations).toHaveLength(5);
  });

  it("reproduces every HChaCha20 case", () => {
    for (const testCase of fixture.hchacha20_cases) {
      expect(
        hchacha20Subkey(
          fromHex(testCase.key_hex),
          fromHex(testCase.nonce_hex),
        ),
        testCase.id,
      ).toEqual(fromHex(testCase.subkey_hex));
    }
  });

  it("reproduces and reverses every raw XChaCha20 case", () => {
    for (const testCase of fixture.xchacha20_cases) {
      const input = fromHex(testCase.input_hex);
      const key = fromHex(testCase.key_hex);
      const nonce = fromHex(testCase.nonce_hex);
      const output = xchacha20Encrypt(
        input,
        key,
        nonce,
        testCase.counter,
      );
      expect(output, testCase.id).toEqual(fromHex(testCase.output_hex));
      expect(
        xchacha20Encrypt(output, key, nonce, testCase.counter),
        testCase.id,
      ).toEqual(input);
    }
  });

  it("encrypts and decrypts every AEAD case byte-identically", () => {
    for (const testCase of fixture.aead_cases) {
      const key = fromHex(testCase.key_hex);
      const nonce = fromHex(testCase.nonce_hex);
      const aad = fromHex(testCase.aad_hex);
      const plaintext = fromHex(testCase.plaintext_hex);
      const ciphertext = fromHex(testCase.ciphertext_hex);
      const tag = fromHex(testCase.tag_hex);

      expect(
        xchacha20Poly1305Encrypt(plaintext, key, nonce, aad),
        testCase.id,
      ).toEqual([ciphertext, tag]);
      expect(
        xchacha20Poly1305Decrypt(ciphertext, key, nonce, aad, tag),
        testCase.id,
      ).toEqual(plaintext);
    }
  });

  it("maps every mutation to one authentication failure", () => {
    const cases = new Map(fixture.aead_cases.map((testCase) => [testCase.id, testCase]));

    for (const mutation of fixture.mutations) {
      const source = cases.get(mutation.source_case);
      if (source === undefined) {
        throw new Error(`unknown fixture source ${mutation.source_case}`);
      }
      const originals: Record<MutationTarget, Uint8Array> = {
        ciphertext: fromHex(source.ciphertext_hex),
        key: fromHex(source.key_hex),
        nonce: fromHex(source.nonce_hex),
        aad: fromHex(source.aad_hex),
        tag: fromHex(source.tag_hex),
      };

      for (const byteIndex of mutation.byte_indices) {
        const changed: Record<MutationTarget, Uint8Array> = {
          ciphertext: new Uint8Array(originals.ciphertext),
          key: new Uint8Array(originals.key),
          nonce: new Uint8Array(originals.nonce),
          aad: new Uint8Array(originals.aad),
          tag: new Uint8Array(originals.tag),
        };
        const target = changed[mutation.target];
        target[byteIndex] =
          (target[byteIndex] ?? 0) ^ Number.parseInt(mutation.xor_hex, 16);

        expect(() =>
          xchacha20Poly1305Decrypt(
            changed.ciphertext,
            changed.key,
            changed.nonce,
            changed.aad,
            changed.tag,
          ),
        ).toThrow("Authentication failed");
      }
    }
  });
});
