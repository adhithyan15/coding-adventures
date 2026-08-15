/** Portable D18F encrypted messages for Chief of Staff channels. */

export * from "./grant-profile.js";

import {
  xchacha20Poly1305Decrypt,
  xchacha20Poly1305Encrypt,
} from "@coding-adventures/chacha20-poly1305";
import { sign, verify } from "@coding-adventures/ed25519";
import { sha256 } from "@coding-adventures/sha256";
import { UUID } from "@coding-adventures/uuid";

const MESSAGE_CONTEXT = new TextEncoder().encode("chief-channel-message-v1");
const MESSAGE_MAGIC = new TextEncoder().encode("D18M");
const WIRE_VERSION = 1;
const MAX_IDENTITY_BYTES = 4 * 1024;
const MAX_CONTENT_TYPE_BYTES = 1024;
const MAX_CIPHERTEXT_BYTES = 64 * 1024 * 1024;
const MAX_U64 = (1n << 64n) - 1n;
const MAX_UUID_TIMESTAMP = (1n << 48n) - 1n;
const RANDOM_MASK = (1n << 74n) - 1n;
const BASE64 = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const JSON_FIELDS = [
  "record_type",
  "wire_version",
  "message_id",
  "timestamp_ns",
  "originator_id_b64",
  "channel_id",
  "sequence",
  "key_epoch",
  "content_type",
  "plaintext_hash_hex",
  "ciphertext_b64",
  "authentication_tag_b64",
  "originator_signature_b64",
] as const;

/** Maximum accepted UTF-8 bytes in one diagnostic JSON record. */
export const MAX_MESSAGE_JSON_BYTES = 90 * 1024 * 1024;

/** Stable D18F error codes shared by all portable implementations. */
export type MessageProfileErrorCode =
  | "invalid_magic"
  | "unsupported_version"
  | "truncated_record"
  | "trailing_bytes"
  | "length_limit_exceeded"
  | "invalid_utf8"
  | "invalid_field"
  | "invalid_json"
  | "missing_epoch_key"
  | "invalid_signature"
  | "authentication_failed"
  | "plaintext_hash_mismatch";

/** One fail-closed D18F operation error. */
export class MessageProfileError extends Error {
  readonly code: MessageProfileErrorCode;

  constructor(code: MessageProfileErrorCode) {
    super(code);
    this.name = "MessageProfileError";
    this.code = code;
  }
}

/** Fields supplied before a message is hashed, signed, and encrypted. */
export interface MessageFields {
  readonly messageId: Uint8Array;
  readonly timestampNs: bigint;
  readonly originatorId: Uint8Array;
  readonly channelId: Uint8Array;
  readonly sequence: bigint;
  readonly keyEpoch: bigint;
  readonly contentType: string;
}

/** Inputs whose identifier and timestamp may be obtained from injected sources. */
export type SourcedMessageFields = Omit<MessageFields, "messageId" | "timestampNs">;

/** Injected UUID-v7 source used by convenience creation. */
export interface UuidV7Source {
  next(): Uint8Array;
}

/** Injected monotonic nanosecond clock used by convenience creation. */
export interface MonotonicNanosecondSource {
  now(): bigint;
}

/** Complete immutable encrypted-message representation. */
export interface MessageParts extends MessageFields {
  readonly plaintextHash: Uint8Array;
  readonly ciphertext: Uint8Array;
  readonly authenticationTag: Uint8Array;
  readonly originatorSignature: Uint8Array;
}

/**
 * Immutable D18F message.
 *
 * Byte inputs are defensively copied into runtime-private slots. Every byte
 * accessor returns a new copy, and the public object is frozen.
 */
export class D18Message {
  readonly #messageId: Uint8Array;
  readonly #timestampNs: bigint;
  readonly #originatorId: Uint8Array;
  readonly #channelId: Uint8Array;
  readonly #sequence: bigint;
  readonly #keyEpoch: bigint;
  readonly #contentType: string;
  readonly #plaintextHash: Uint8Array;
  readonly #ciphertext: Uint8Array;
  readonly #authenticationTag: Uint8Array;
  readonly #originatorSignature: Uint8Array;

  constructor(parts: MessageParts) {
    requireLength(parts.messageId, 16);
    requireU64(parts.timestampNs);
    if (parts.originatorId.length > MAX_IDENTITY_BYTES) {
      fail("length_limit_exceeded");
    }
    requireLength(parts.channelId, 16);
    requireU64(parts.sequence);
    requireU64(parts.keyEpoch);
    if (!isWellFormedUnicode(parts.contentType)) fail("invalid_field");
    const contentTypeLength = utf8(parts.contentType).length;
    if (contentTypeLength > MAX_CONTENT_TYPE_BYTES) {
      fail("length_limit_exceeded");
    }
    requireLength(parts.plaintextHash, 32);
    if (parts.ciphertext.length > MAX_CIPHERTEXT_BYTES) {
      fail("length_limit_exceeded");
    }
    requireLength(parts.authenticationTag, 16);
    requireLength(parts.originatorSignature, 64);

    this.#messageId = parts.messageId.slice();
    this.#timestampNs = parts.timestampNs;
    this.#originatorId = parts.originatorId.slice();
    this.#channelId = parts.channelId.slice();
    this.#sequence = parts.sequence;
    this.#keyEpoch = parts.keyEpoch;
    this.#contentType = parts.contentType;
    this.#plaintextHash = parts.plaintextHash.slice();
    this.#ciphertext = parts.ciphertext.slice();
    this.#authenticationTag = parts.authenticationTag.slice();
    this.#originatorSignature = parts.originatorSignature.slice();
    Object.freeze(this);
  }

  get messageId(): Uint8Array { return this.#messageId.slice(); }
  get timestampNs(): bigint { return this.#timestampNs; }
  get originatorId(): Uint8Array { return this.#originatorId.slice(); }
  get channelId(): Uint8Array { return this.#channelId.slice(); }
  get sequence(): bigint { return this.#sequence; }
  get keyEpoch(): bigint { return this.#keyEpoch; }
  get contentType(): string { return this.#contentType; }
  get plaintextHash(): Uint8Array { return this.#plaintextHash.slice(); }
  get ciphertext(): Uint8Array { return this.#ciphertext.slice(); }
  get authenticationTag(): Uint8Array { return this.#authenticationTag.slice(); }
  get originatorSignature(): Uint8Array { return this.#originatorSignature.slice(); }
}

/** Stateful RFC 9562 UUID-v7 generator with same-millisecond ordering. */
export class MonotonicUuidV7Generator {
  #lastTimestampMs: bigint | undefined;
  #lastRandom = 0n;

  next(timestampMs: bigint, entropy: Uint8Array): Uint8Array {
    if (timestampMs < 0n || timestampMs > MAX_UUID_TIMESTAMP || entropy.length !== 10) {
      fail("invalid_field");
    }
    let suppliedRandom = 0n;
    for (const byte of entropy) suppliedRandom = (suppliedRandom << 8n) | BigInt(byte);
    suppliedRandom &= RANDOM_MASK;

    let effectiveTimestamp = timestampMs;
    let random = suppliedRandom;
    if (this.#lastTimestampMs !== undefined && timestampMs <= this.#lastTimestampMs) {
      effectiveTimestamp = this.#lastTimestampMs;
      if (this.#lastRandom < RANDOM_MASK) {
        random = this.#lastRandom + 1n;
      } else if (effectiveTimestamp < MAX_UUID_TIMESTAMP) {
        effectiveTimestamp += 1n;
        random = 0n;
      } else {
        fail("invalid_field");
      }
    }
    this.#lastTimestampMs = effectiveTimestamp;
    this.#lastRandom = random;

    const randomA = (random >> 62n) & 0xfffn;
    const randomB = random & ((1n << 62n) - 1n);
    const value =
      (effectiveTimestamp << 80n) |
      (7n << 76n) |
      (randomA << 64n) |
      (2n << 62n) |
      randomB;
    return bigintToBytes(value, 16);
  }
}

/** Validate the high-level D18F creation and delivery rules. */
export function validateMessageFields(fields: MessageFields): void {
  validateUuidV7(fields.messageId);
  validateUuidV7(fields.channelId);
  requireU64(fields.timestampNs);
  requireU64(fields.sequence);
  requireU64(fields.keyEpoch);
  if (fields.originatorId.length === 0) fail("invalid_field");
  if (fields.originatorId.length > MAX_IDENTITY_BYTES) fail("length_limit_exceeded");
  const contentTypeBytes = utf8(fields.contentType);
  if (contentTypeBytes.length > MAX_CONTENT_TYPE_BYTES) fail("length_limit_exceeded");
  validateMime(fields.contentType);
}

/** Validate, hash, sign, and encrypt one D18F message. */
export function messageCreate(
  fields: MessageFields,
  plaintext: Uint8Array,
  signingSecretKey: Uint8Array,
  channelMasterKey: Uint8Array,
): D18Message {
  validateMessageFields(fields);
  if (plaintext.length > MAX_CIPHERTEXT_BYTES) fail("length_limit_exceeded");
  requireLength(signingSecretKey, 64);
  requireLength(channelMasterKey, 32);

  const plaintextHash = sha256(plaintext);
  const header = authenticatedHeaderFromFields(fields, plaintextHash);
  const nonce = messageNonce(fields.channelId, fields.sequence);
  const [ciphertext, authenticationTag] = xchacha20Poly1305Encrypt(
    plaintext,
    channelMasterKey,
    nonce,
    header,
  );
  const originatorSignature = sign(header, signingSecretKey);
  return new D18Message({
    ...copyFields(fields),
    plaintextHash,
    ciphertext,
    authenticationTag,
    originatorSignature,
  });
}

/** Create a message using explicit, injected UUID-v7 and monotonic sources. */
export function messageCreateWithSources(
  fields: SourcedMessageFields,
  plaintext: Uint8Array,
  signingSecretKey: Uint8Array,
  channelMasterKey: Uint8Array,
  uuidSource: UuidV7Source,
  clock: MonotonicNanosecondSource,
): D18Message {
  return messageCreate(
    { ...fields, messageId: uuidSource.next(), timestampNs: clock.now() },
    plaintext,
    signingSecretKey,
    channelMasterKey,
  );
}

/** Verify and decrypt a message using an explicitly selected epoch key. */
export function messageVerify(
  message: D18Message,
  originatorPublicKey: Uint8Array,
  channelMasterKey: Uint8Array,
): Uint8Array {
  return verifyWithKey(message, originatorPublicKey, channelMasterKey);
}

/** Resolve the named key epoch before signature and AEAD verification. */
export function messageVerifyWithKeyResolver(
  message: D18Message,
  originatorPublicKey: Uint8Array,
  keyForEpoch: (epoch: bigint) => Uint8Array | undefined,
): Uint8Array {
  validateMessageFields(messageFields(message));
  const key = keyForEpoch(message.keyEpoch);
  if (key === undefined) fail("missing_epoch_key");
  requireLength(key, 32);
  return verifyCryptography(message, originatorPublicKey, key);
}

function verifyWithKey(
  message: D18Message,
  originatorPublicKey: Uint8Array,
  channelMasterKey: Uint8Array,
): Uint8Array {
  validateMessageFields(messageFields(message));
  requireLength(channelMasterKey, 32);
  return verifyCryptography(message, originatorPublicKey, channelMasterKey);
}

function verifyCryptography(
  message: D18Message,
  originatorPublicKey: Uint8Array,
  channelMasterKey: Uint8Array,
): Uint8Array {
  requireLength(originatorPublicKey, 32);
  const header = messageAuthenticatedHeader(message);
  let signatureValid = false;
  try {
    signatureValid = verify(header, message.originatorSignature, originatorPublicKey);
  } catch {
    // Invalid encoded points remain one portable signature failure.
  }
  if (!signatureValid) {
    fail("invalid_signature");
  }
  let plaintext: Uint8Array;
  try {
    plaintext = xchacha20Poly1305Decrypt(
      message.ciphertext,
      channelMasterKey,
      messageNonce(message.channelId, message.sequence),
      header,
      message.authenticationTag,
    );
  } catch {
    fail("authentication_failed");
  }
  if (!equalBytes(sha256(plaintext), message.plaintextHash)) {
    fail("plaintext_hash_mismatch");
  }
  return plaintext;
}

/** Return the exact D18F authenticated header. */
export function messageAuthenticatedHeader(message: D18Message): Uint8Array {
  return authenticatedHeaderFromFields(messageFields(message), message.plaintextHash);
}

/** Serialize one message as the unchanged D18M version 1 record. */
export function messageSerialize(message: D18Message): Uint8Array {
  const originatorId = message.originatorId;
  const contentType = utf8(message.contentType);
  const ciphertext = message.ciphertext;
  if (originatorId.length > MAX_IDENTITY_BYTES || contentType.length > MAX_CONTENT_TYPE_BYTES || ciphertext.length > MAX_CIPHERTEXT_BYTES) {
    fail("length_limit_exceeded");
  }
  return concat([
    MESSAGE_MAGIC,
    Uint8Array.of(WIRE_VERSION),
    message.messageId,
    u64be(message.timestampNs),
    u32be(originatorId.length),
    originatorId,
    message.channelId,
    u64be(message.sequence),
    u64be(message.keyEpoch),
    u32be(contentType.length),
    contentType,
    message.plaintextHash,
    u64be(BigInt(ciphertext.length)),
    ciphertext,
    message.authenticationTag,
    message.originatorSignature,
  ]);
}

/** Structurally decode one D18M version 1 binary record. */
export function messageDeserialize(bytes: Uint8Array): D18Message {
  const decoder = new Decoder(bytes);
  const magic = decoder.take(4);
  if (!equalBytes(magic, MESSAGE_MAGIC)) fail("invalid_magic");
  const version = decoder.take(1)[0];
  if (version !== WIRE_VERSION) fail("unsupported_version");
  const messageId = decoder.take(16);
  const timestampNs = decoder.readU64();
  const originatorId = decoder.readBoundedU32(MAX_IDENTITY_BYTES);
  const channelId = decoder.take(16);
  const sequence = decoder.readU64();
  const keyEpoch = decoder.readU64();
  const contentTypeBytes = decoder.readBoundedU32(MAX_CONTENT_TYPE_BYTES);
  let contentType: string;
  try {
    contentType = new TextDecoder("utf-8", { fatal: true }).decode(contentTypeBytes);
  } catch {
    fail("invalid_utf8");
  }
  const plaintextHash = decoder.take(32);
  const ciphertext = decoder.readBoundedU64(MAX_CIPHERTEXT_BYTES);
  const authenticationTag = decoder.take(16);
  const originatorSignature = decoder.take(64);
  decoder.finish();
  return new D18Message({
    messageId,
    timestampNs,
    originatorId,
    channelId,
    sequence,
    keyEpoch,
    contentType,
    plaintextHash,
    ciphertext,
    authenticationTag,
    originatorSignature,
  });
}

/** Encode one message as canonical, lossless D18F JSON bytes. */
export function messageToJson(message: D18Message): Uint8Array {
  const pieces = [
    `{\"record_type\":\"D18M\",\"wire_version\":1`,
    `,\"message_id\":${jsonString(uuidString(message.messageId))}`,
    `,\"timestamp_ns\":${jsonString(message.timestampNs.toString())}`,
    `,\"originator_id_b64\":${jsonString(encodeBase64(message.originatorId))}`,
    `,\"channel_id\":${jsonString(uuidString(message.channelId))}`,
    `,\"sequence\":${jsonString(message.sequence.toString())}`,
    `,\"key_epoch\":${jsonString(message.keyEpoch.toString())}`,
    `,\"content_type\":${jsonString(message.contentType)}`,
    `,\"plaintext_hash_hex\":${jsonString(encodeHex(message.plaintextHash))}`,
    `,\"ciphertext_b64\":${jsonString(encodeBase64(message.ciphertext))}`,
    `,\"authentication_tag_b64\":${jsonString(encodeBase64(message.authenticationTag))}`,
    `,\"originator_signature_b64\":${jsonString(encodeBase64(message.originatorSignature))}}`,
  ];
  const encoded = utf8(pieces.join(""));
  if (encoded.length > MAX_MESSAGE_JSON_BYTES) fail("length_limit_exceeded");
  return encoded;
}

/** Structurally decode lossless D18F JSON into an immutable message. */
export function messageFromJson(bytes: Uint8Array): D18Message {
  if (bytes.length > MAX_MESSAGE_JSON_BYTES) fail("length_limit_exceeded");
  let text: string;
  try {
    text = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    fail("invalid_json");
  }
  const { value, rawFields } = parseStrictJsonObject(text);
  const keys = Object.keys(value);
  if (keys.length !== JSON_FIELDS.length || JSON_FIELDS.some((key) => !Object.hasOwn(value, key))) {
    fail("invalid_json");
  }
  if (typeof value.record_type !== "string") fail("invalid_json");
  if (value.record_type !== "D18M") fail("invalid_magic");
  if (typeof value.wire_version !== "number") fail("invalid_json");
  if (rawFields.get("wire_version")?.trim() !== "1") fail("unsupported_version");

  const messageId = decodeUuidV7(stringField(value, "message_id"));
  const timestampNs = decodeDecimal(stringField(value, "timestamp_ns"));
  const originatorId = decodeBase64(stringField(value, "originator_id_b64"), MAX_IDENTITY_BYTES);
  const channelId = decodeUuidV7(stringField(value, "channel_id"));
  const sequence = decodeDecimal(stringField(value, "sequence"));
  const keyEpoch = decodeDecimal(stringField(value, "key_epoch"));
  const contentType = stringField(value, "content_type");
  if (utf8(contentType).length > MAX_CONTENT_TYPE_BYTES) fail("length_limit_exceeded");
  const plaintextHash = decodeHex(stringField(value, "plaintext_hash_hex"), 32);
  const ciphertext = decodeBase64(stringField(value, "ciphertext_b64"), MAX_CIPHERTEXT_BYTES);
  const authenticationTag = decodeBase64Exact(stringField(value, "authentication_tag_b64"), 16);
  const originatorSignature = decodeBase64Exact(stringField(value, "originator_signature_b64"), 64);
  return new D18Message({
    messageId,
    timestampNs,
    originatorId,
    channelId,
    sequence,
    keyEpoch,
    contentType,
    plaintextHash,
    ciphertext,
    authenticationTag,
    originatorSignature,
  });
}

function authenticatedHeaderFromFields(fields: MessageFields, plaintextHash: Uint8Array): Uint8Array {
  return frame([
    MESSAGE_CONTEXT,
    fields.messageId,
    u64be(fields.timestampNs),
    fields.originatorId,
    fields.channelId,
    u64be(fields.sequence),
    u64be(fields.keyEpoch),
    utf8(fields.contentType),
    plaintextHash,
  ]);
}

function messageNonce(channelId: Uint8Array, sequence: bigint): Uint8Array {
  requireLength(channelId, 16);
  return concat([channelId, u64be(sequence)]);
}

function frame(fields: readonly Uint8Array[]): Uint8Array {
  const framed: Uint8Array[] = [];
  for (const field of fields) framed.push(u64be(BigInt(field.length)), field);
  return concat(framed);
}

function copyFields(fields: MessageFields): MessageFields {
  return {
    messageId: fields.messageId.slice(),
    timestampNs: fields.timestampNs,
    originatorId: fields.originatorId.slice(),
    channelId: fields.channelId.slice(),
    sequence: fields.sequence,
    keyEpoch: fields.keyEpoch,
    contentType: fields.contentType,
  };
}

function messageFields(message: D18Message): MessageFields {
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

function validateUuidV7(bytes: Uint8Array): void {
  requireLength(bytes, 16);
  if ((bytes[6] >> 4) !== 7 || (bytes[8] & 0xc0) !== 0x80) fail("invalid_field");
}

function decodeUuidV7(value: string): Uint8Array {
  let uuid: UUID;
  try {
    uuid = new UUID(value);
  } catch {
    fail("invalid_field");
  }
  if (uuid.toString() !== value) fail("invalid_field");
  const bytes = uuid.bytes;
  validateUuidV7(bytes);
  return bytes;
}

function uuidString(bytes: Uint8Array): string {
  try {
    return new UUID(bytes).toString();
  } catch {
    fail("invalid_field");
  }
}

function decodeDecimal(value: string): bigint {
  if (!/^(0|[1-9][0-9]*)$/.test(value)) fail("invalid_field");
  const decoded = BigInt(value);
  if (decoded > MAX_U64) fail("invalid_field");
  return decoded;
}

function validateMime(value: string): void {
  const bytes = utf8(value);
  if (bytes.length === 0 || bytes.some((byte) => byte < 0x20 || byte > 0x7e)) fail("invalid_field");
  let index = 0;
  index = consumeToken(bytes, index);
  if (bytes[index] !== 0x2f) fail("invalid_field");
  index = consumeToken(bytes, index + 1);
  while (index < bytes.length) {
    index = consumeSpaces(bytes, index);
    if (bytes[index] !== 0x3b) fail("invalid_field");
    index = consumeSpaces(bytes, index + 1);
    index = consumeToken(bytes, index);
    index = consumeSpaces(bytes, index);
    if (bytes[index] !== 0x3d) fail("invalid_field");
    index = consumeSpaces(bytes, index + 1);
    if (bytes[index] === 0x22) {
      index += 1;
      for (;;) {
        if (index >= bytes.length) fail("invalid_field");
        if (bytes[index] === 0x22) { index += 1; break; }
        if (bytes[index] === 0x5c) {
          index += 1;
          if (index >= bytes.length) fail("invalid_field");
        }
        index += 1;
      }
    } else {
      index = consumeToken(bytes, index);
    }
  }
}

function consumeToken(bytes: Uint8Array, index: number): number {
  const start = index;
  while (index < bytes.length && isMimeToken(bytes[index])) index += 1;
  if (index === start) fail("invalid_field");
  return index;
}

function consumeSpaces(bytes: Uint8Array, index: number): number {
  while (bytes[index] === 0x20) index += 1;
  return index;
}

function isMimeToken(byte: number): boolean {
  return (byte >= 0x30 && byte <= 0x39) ||
    (byte >= 0x41 && byte <= 0x5a) ||
    (byte >= 0x61 && byte <= 0x7a) ||
    [0x21, 0x23, 0x24, 0x25, 0x26, 0x27, 0x2a, 0x2b, 0x2d, 0x2e, 0x5e, 0x5f, 0x60, 0x7c, 0x7e].includes(byte);
}

function encodeBase64(bytes: Uint8Array): string {
  let output = "";
  for (let index = 0; index < bytes.length; index += 3) {
    const remaining = bytes.length - index;
    const word = (bytes[index] << 16) | ((bytes[index + 1] ?? 0) << 8) | (bytes[index + 2] ?? 0);
    output += BASE64[(word >>> 18) & 63];
    output += BASE64[(word >>> 12) & 63];
    output += remaining > 1 ? BASE64[(word >>> 6) & 63] : "=";
    output += remaining > 2 ? BASE64[word & 63] : "=";
  }
  return output;
}

function decodeBase64(value: string, maximum: number): Uint8Array {
  if (value.length % 4 !== 0) fail("invalid_field");
  if ((value.length / 4) * 3 > maximum + 2) fail("length_limit_exceeded");
  const output = new Uint8Array((value.length / 4) * 3);
  let outputIndex = 0;
  for (let index = 0; index < value.length; index += 4) {
    const final = index + 4 === value.length;
    const chars = value.slice(index, index + 4);
    if ((!final && chars.includes("=")) || (chars[2] === "=" && chars[3] !== "=")) fail("invalid_field");
    const a = base64Digit(chars[0]);
    const b = base64Digit(chars[1]);
    const c = chars[2] === "=" ? 0 : base64Digit(chars[2]);
    const d = chars[3] === "=" ? 0 : base64Digit(chars[3]);
    const word = (a << 18) | (b << 12) | (c << 6) | d;
    output[outputIndex++] = (word >>> 16) & 0xff;
    if (chars[2] !== "=") output[outputIndex++] = (word >>> 8) & 0xff;
    if (chars[3] !== "=") output[outputIndex++] = word & 0xff;
  }
  const decoded = output.slice(0, outputIndex);
  if (decoded.length > maximum) fail("length_limit_exceeded");
  if (encodeBase64(decoded) !== value) fail("invalid_field");
  return decoded;
}

function decodeBase64Exact(value: string, length: number): Uint8Array {
  const decoded = decodeBase64(value, length);
  if (decoded.length !== length) fail("invalid_field");
  return decoded;
}

function base64Digit(value: string): number {
  const index = BASE64.indexOf(value);
  if (index < 0) fail("invalid_field");
  return index;
}

function encodeHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function decodeHex(value: string, length: number): Uint8Array {
  if (value.length !== length * 2 || !/^[0-9a-f]+$/.test(value)) fail("invalid_field");
  const output = new Uint8Array(length);
  for (let index = 0; index < length; index += 1) {
    output[index] = Number.parseInt(value.slice(index * 2, index * 2 + 2), 16);
  }
  return output;
}

function parseStrictJsonObject(text: string): {
  value: Record<string, unknown>;
  rawFields: Map<string, string>;
} {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    fail("invalid_json");
  }
  if (parsed === null || typeof parsed !== "object" || Array.isArray(parsed)) fail("invalid_json");

  const rawFields = new Map<string, string>();
  let index = skipWhitespace(text, 0);
  if (text[index] !== "{") fail("invalid_json");
  index = skipWhitespace(text, index + 1);
  if (text[index] === "}") return { value: parsed as Record<string, unknown>, rawFields };
  for (;;) {
    if (text[index] !== "\"") fail("invalid_json");
    const keyEnd = skipJsonString(text, index);
    let key: string;
    try {
      key = JSON.parse(text.slice(index, keyEnd)) as string;
    } catch {
      fail("invalid_json");
    }
    if (rawFields.has(key)) fail("invalid_json");
    index = skipWhitespace(text, keyEnd);
    if (text[index] !== ":") fail("invalid_json");
    const valueStart = skipWhitespace(text, index + 1);
    const valueEnd = skipTopLevelValue(text, valueStart);
    rawFields.set(key, text.slice(valueStart, valueEnd));
    index = skipWhitespace(text, valueEnd);
    if (text[index] === "}") {
      index = skipWhitespace(text, index + 1);
      if (index !== text.length) fail("invalid_json");
      break;
    }
    if (text[index] !== ",") fail("invalid_json");
    index = skipWhitespace(text, index + 1);
  }
  return { value: parsed as Record<string, unknown>, rawFields };
}

function skipJsonString(text: string, start: number): number {
  let escaped = false;
  for (let index = start + 1; index < text.length; index += 1) {
    const character = text[index];
    if (escaped) { escaped = false; continue; }
    if (character === "\\") { escaped = true; continue; }
    if (character === "\"") return index + 1;
  }
  fail("invalid_json");
}

function skipTopLevelValue(text: string, start: number): number {
  let objectDepth = 0;
  let arrayDepth = 0;
  let inString = false;
  let escaped = false;
  for (let index = start; index < text.length; index += 1) {
    const character = text[index];
    if (inString) {
      if (escaped) escaped = false;
      else if (character === "\\") escaped = true;
      else if (character === "\"") inString = false;
      continue;
    }
    if (character === "\"") inString = true;
    else if (character === "{") objectDepth += 1;
    else if (character === "[") arrayDepth += 1;
    else if (character === "}") {
      if (objectDepth === 0 && arrayDepth === 0) return index;
      objectDepth -= 1;
    } else if (character === "]") arrayDepth -= 1;
    else if (character === "," && objectDepth === 0 && arrayDepth === 0) return index;
  }
  fail("invalid_json");
}

function stringField(object: Record<string, unknown>, name: string): string {
  const value = object[name];
  if (typeof value !== "string") fail("invalid_json");
  return value;
}

function jsonString(value: string): string {
  return JSON.stringify(value);
}

function skipWhitespace(text: string, index: number): number {
  while (index < text.length && /[\t\n\r ]/.test(text[index])) index += 1;
  return index;
}

function utf8(value: string): Uint8Array {
  return new TextEncoder().encode(value);
}

function isWellFormedUnicode(value: string): boolean {
  for (let index = 0; index < value.length; index += 1) {
    const unit = value.charCodeAt(index);
    if (unit >= 0xd800 && unit <= 0xdbff) {
      if (index + 1 >= value.length) return false;
      const next = value.charCodeAt(index + 1);
      if (next < 0xdc00 || next > 0xdfff) return false;
      index += 1;
    } else if (unit >= 0xdc00 && unit <= 0xdfff) {
      return false;
    }
  }
  return true;
}

function concat(parts: readonly Uint8Array[]): Uint8Array {
  const length = parts.reduce((total, part) => total + part.length, 0);
  const output = new Uint8Array(length);
  let offset = 0;
  for (const part of parts) { output.set(part, offset); offset += part.length; }
  return output;
}

function u32be(value: number): Uint8Array {
  const output = new Uint8Array(4);
  new DataView(output.buffer).setUint32(0, value, false);
  return output;
}

function u64be(value: bigint): Uint8Array {
  requireU64(value);
  return bigintToBytes(value, 8);
}

function bigintToBytes(value: bigint, length: number): Uint8Array {
  const output = new Uint8Array(length);
  let remaining = value;
  for (let index = length - 1; index >= 0; index -= 1) {
    output[index] = Number(remaining & 0xffn);
    remaining >>= 8n;
  }
  return output;
}

function requireU64(value: bigint): void {
  if (typeof value !== "bigint" || value < 0n || value > MAX_U64) fail("invalid_field");
}

function requireLength(bytes: Uint8Array, length: number): void {
  if (!(bytes instanceof Uint8Array) || bytes.length !== length) fail("invalid_field");
}

function equalBytes(left: Uint8Array, right: Uint8Array): boolean {
  if (left.length !== right.length) return false;
  let difference = 0;
  for (let index = 0; index < left.length; index += 1) difference |= left[index] ^ right[index];
  return difference === 0;
}

function fail(code: MessageProfileErrorCode): never {
  throw new MessageProfileError(code);
}

class Decoder {
  readonly #bytes: Uint8Array;
  #position = 0;

  constructor(bytes: Uint8Array) {
    this.#bytes = bytes;
  }

  take(length: number): Uint8Array {
    if (length < 0 || this.#position + length > this.#bytes.length) fail("truncated_record");
    const output = this.#bytes.slice(this.#position, this.#position + length);
    this.#position += length;
    return output;
  }

  readU64(): bigint { return bytesToBigint(this.take(8)); }

  readBoundedU32(maximum: number): Uint8Array {
    const bytes = this.take(4);
    const length = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength).getUint32(0, false);
    return this.readBounded(BigInt(length), maximum);
  }

  readBoundedU64(maximum: number): Uint8Array {
    return this.readBounded(this.readU64(), maximum);
  }

  readBounded(length: bigint, maximum: number): Uint8Array {
    if (length > BigInt(maximum)) fail("length_limit_exceeded");
    return this.take(Number(length));
  }

  finish(): void {
    if (this.#position !== this.#bytes.length) fail("trailing_bytes");
  }
}

function bytesToBigint(bytes: Uint8Array): bigint {
  let value = 0n;
  for (const byte of bytes) value = (value << 8n) | BigInt(byte);
  return value;
}
