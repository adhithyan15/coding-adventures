import { readFileSync } from "node:fs";
import { generateKeypair } from "@coding-adventures/ed25519";
import { describe, expect, it } from "vitest";
import {
  D18Message,
  MAX_MESSAGE_JSON_BYTES,
  MessageProfileError,
  type MessageProfileErrorCode,
  MonotonicUuidV7Generator,
  messageAuthenticatedHeader,
  messageCreate,
  messageCreateWithSources,
  messageDeserialize,
  messageFromJson,
  messageSerialize,
  messageToJson,
  messageVerify,
  messageVerifyWithKeyResolver,
} from "../src/index.js";

interface PositiveCase {
  name: string;
  plaintext_b64: string;
  authenticated_header_b64: string;
  d18m_b64: string;
  canonical_json_b64: string;
}

interface NegativeBinaryCase {
  name: string;
  phase: "deserialize" | "verify";
  d18m_b64: string;
  expected_error: MessageProfileErrorCode;
}

interface NegativeJsonCase {
  name: string;
  json_b64: string;
  expected_error: MessageProfileErrorCode;
}

interface OversizeRecipe {
  field: "originator-id" | "content-type" | "ciphertext" | "json-input";
  declared_length: string;
  expected_error: MessageProfileErrorCode;
}

interface Fixture {
  fixture_format: string;
  generator_blob_sha1: string;
  warning: string;
  keys: {
    originator_signing_seed_hex: string;
    originator_public_key_hex: string;
    channel_master_keys: Array<{ key_epoch: string; key_hex: string }>;
  };
  positive_cases: PositiveCase[];
  binary_negative_cases: NegativeBinaryCase[];
  json_negative_cases: NegativeJsonCase[];
  oversize_recipes: OversizeRecipe[];
}

const fixture = JSON.parse(
  readFileSync(
    new URL(
      "../../../../fixtures/chief-of-staff-message/v1/manifest.json",
      import.meta.url,
    ),
    "utf8",
  ),
) as Fixture;

const signingSeed = fromHex(fixture.keys.originator_signing_seed_hex);
const { publicKey: derivedPublicKey, secretKey: signingSecretKey } =
  generateKeypair(signingSeed);
const publicKey = fromHex(fixture.keys.originator_public_key_hex);
const epochKeys = new Map(
  fixture.keys.channel_master_keys.map(({ key_epoch, key_hex }) => [
    BigInt(key_epoch),
    fromHex(key_hex),
  ]),
);

describe("D18F shared fixture", () => {
  it("locks fixture provenance and public test material", () => {
    expect(fixture.fixture_format).toBe("D18F-message-fixtures-v1");
    expect(fixture.generator_blob_sha1).toMatch(/^[0-9a-f]{40}$/);
    expect(fixture.warning).toContain("test-only");
    expect(fixture.positive_cases).toHaveLength(8);
    expect(fixture.binary_negative_cases).toHaveLength(20);
    expect(fixture.json_negative_cases).toHaveLength(11);
    expect(derivedPublicKey).toEqual(publicKey);
  });

  it("decodes, verifies, and reproduces every positive case byte-identically", () => {
    for (const testCase of fixture.positive_cases) {
      const binary = fromBase64(testCase.d18m_b64);
      const plaintext = fromBase64(testCase.plaintext_b64);
      const message = messageDeserialize(binary);
      const key = epochKeys.get(message.keyEpoch);
      expect(key, testCase.name).toBeDefined();

      expect(messageSerialize(message), testCase.name).toEqual(binary);
      expect(messageAuthenticatedHeader(message), testCase.name).toEqual(
        fromBase64(testCase.authenticated_header_b64),
      );
      expect(
        messageVerifyWithKeyResolver(message, publicKey, (epoch) => epochKeys.get(epoch)),
        testCase.name,
      ).toEqual(plaintext);
      expect(messageVerify(message, publicKey, key!), testCase.name).toEqual(plaintext);

      const canonicalJson = fromBase64(testCase.canonical_json_b64);
      expect(messageToJson(message), testCase.name).toEqual(canonicalJson);
      expect(messageSerialize(messageFromJson(canonicalJson)), testCase.name).toEqual(binary);

      const recreated = messageCreate(
        fieldsOf(message),
        plaintext,
        signingSecretKey,
        key!,
      );
      expect(messageSerialize(recreated), testCase.name).toEqual(binary);
    }
  });

  it("maps every binary mutation to its declared stable error", () => {
    for (const testCase of fixture.binary_negative_cases) {
      expectProfileError(() => {
        const message = messageDeserialize(fromBase64(testCase.d18m_b64));
        if (testCase.phase === "verify") {
          messageVerifyWithKeyResolver(message, publicKey, (epoch) => epochKeys.get(epoch));
        }
      }, testCase.expected_error, testCase.name);
    }
  });

  it("maps every JSON mutation to its declared stable error", () => {
    for (const testCase of fixture.json_negative_cases) {
      expectProfileError(
        () => messageFromJson(fromBase64(testCase.json_b64)),
        testCase.expected_error,
        testCase.name,
      );
    }
  });

  it("accepts arbitrary JSON field order and restores canonical order", () => {
    const canonical = new TextDecoder().decode(
      fromBase64(fixture.positive_cases[2].canonical_json_b64),
    );
    const reversed = JSON.stringify(
      Object.fromEntries(Object.entries(JSON.parse(canonical)).reverse()),
    );
    const restored = messageToJson(
      messageFromJson(new TextEncoder().encode(reversed)),
    );
    expect(new TextDecoder().decode(restored)).toBe(canonical);
  });

  it("rejects JSON strings that cannot represent lossless UTF-8", () => {
    const canonical = new TextDecoder().decode(
      fromBase64(fixture.positive_cases[0].canonical_json_b64),
    );
    const malformed = canonical.replace(
      '"content_type":"application/octet-stream"',
      '"content_type":"\\ud800"',
    );
    expectProfileError(
      () => messageFromJson(new TextEncoder().encode(malformed)),
      "invalid_field",
      "unpaired-surrogate",
    );
  });

  it("materializes compact oversize recipes without checked-in large blobs", () => {
    const baseline = fromBase64(fixture.positive_cases[0].d18m_b64);
    for (const recipe of fixture.oversize_recipes) {
      if (recipe.field === "json-input") {
        const logicalOversizeInput = { length: Number(recipe.declared_length) } as Uint8Array;
        expectProfileError(
          () => messageFromJson(logicalOversizeInput),
          recipe.expected_error,
          recipe.field,
        );
        continue;
      }

      const changed = baseline.slice();
      if (recipe.field === "originator-id") {
        setU32(changed, 29, Number(recipe.declared_length));
      } else if (recipe.field === "content-type") {
        setU32(changed, 83, Number(recipe.declared_length));
      } else {
        setU64(changed, 143, BigInt(recipe.declared_length));
      }
      expectProfileError(
        () => messageDeserialize(changed),
        recipe.expected_error,
        recipe.field,
      );
    }
    expect(Number(fixture.oversize_recipes[3].declared_length)).toBe(
      MAX_MESSAGE_JSON_BYTES + 1,
    );
  });
});

describe("TypeScript D18F API", () => {
  it("defensively copies constructor inputs and every byte accessor", () => {
    const source = messageDeserialize(fromBase64(fixture.positive_cases[1].d18m_b64));
    const parts = {
      ...fieldsOf(source),
      plaintextHash: source.plaintextHash,
      ciphertext: source.ciphertext,
      authenticationTag: source.authenticationTag,
      originatorSignature: source.originatorSignature,
    };
    const message = new D18Message(parts);
    const original = messageSerialize(message);

    parts.messageId.fill(0);
    parts.originatorId.fill(0);
    parts.channelId.fill(0);
    parts.plaintextHash.fill(0);
    parts.ciphertext.fill(0);
    parts.authenticationTag.fill(0);
    parts.originatorSignature.fill(0);
    message.messageId.fill(0);
    message.originatorId.fill(0);
    message.channelId.fill(0);
    message.plaintextHash.fill(0);
    message.ciphertext.fill(0);
    message.authenticationTag.fill(0);
    message.originatorSignature.fill(0);

    expect(Object.isFrozen(message)).toBe(true);
    expect(() => Object.assign(message, { sequence: 999n })).toThrow();
    expect(messageSerialize(message)).toEqual(original);
  });

  it("uses injected UUID-v7 and monotonic clock sources", () => {
    const source = messageDeserialize(fromBase64(fixture.positive_cases[0].d18m_b64));
    const key = epochKeys.get(source.keyEpoch)!;
    const message = messageCreateWithSources(
      {
        originatorId: source.originatorId,
        channelId: source.channelId,
        sequence: 123n,
        keyEpoch: source.keyEpoch,
        contentType: source.contentType,
      },
      new Uint8Array([1, 2, 3]),
      signingSecretKey,
      key,
      { next: () => source.messageId },
      { now: () => 456n },
    );
    expect(message.messageId).toEqual(source.messageId);
    expect(message.timestampNs).toBe(456n);
    expect(messageVerify(message, publicKey, key)).toEqual(new Uint8Array([1, 2, 3]));
  });

  it("generates 1,000 strictly ordered UUID-v7 values in one millisecond", () => {
    const generator = new MonotonicUuidV7Generator();
    let previous: Uint8Array | undefined;
    for (let index = 0; index < 1_000; index += 1) {
      const current = generator.next(1_725_000_000_000n, new Uint8Array(10).fill(0x55));
      expect(current[6] >> 4).toBe(7);
      expect(current[8] & 0xc0).toBe(0x80);
      if (previous !== undefined) expect(compareBytes(previous, current)).toBeLessThan(0);
      previous = current;
    }
  });
});

function fieldsOf(message: D18Message) {
  return {
    messageId: message.messageId,
    timestampNs: message.timestampNs,
    originatorId: message.originatorId,
    channelId: message.channelId,
    sequence: message.sequence,
    keyEpoch: message.keyEpoch,
    contentType: message.contentType,
  };
}

function expectProfileError(
  operation: () => unknown,
  code: MessageProfileErrorCode,
  label: string,
): void {
  try {
    operation();
    throw new Error(`${label}: expected ${code}`);
  } catch (error) {
    expect(error, label).toBeInstanceOf(MessageProfileError);
    expect((error as MessageProfileError).code, label).toBe(code);
  }
}

function fromHex(value: string): Uint8Array {
  return Uint8Array.from(value.match(/.{2}/g)?.map((byte) => Number.parseInt(byte, 16)) ?? []);
}

function fromBase64(value: string): Uint8Array {
  return new Uint8Array(Buffer.from(value, "base64"));
}

function setU32(bytes: Uint8Array, offset: number, value: number): void {
  new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).setUint32(offset, value, false);
}

function setU64(bytes: Uint8Array, offset: number, value: bigint): void {
  new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).setBigUint64(offset, value, false);
}

function compareBytes(left: Uint8Array, right: Uint8Array): number {
  for (let index = 0; index < left.length; index += 1) {
    if (left[index] !== right[index]) return left[index] - right[index];
  }
  return 0;
}
