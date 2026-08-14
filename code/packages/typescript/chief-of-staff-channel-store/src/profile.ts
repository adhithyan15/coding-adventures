/** Portable D18P durable-channel values, codecs, keys, and stable failures. */

import { sha256 } from "@coding-adventures/sha256";

export const CHANNEL_STORAGE_NAMESPACE = "chief-channels";
export const CHANNEL_DEFINITION_CONTENT_TYPE =
  "application/vnd.coding-adventures.chief-channel-definition-v1";
export const CHANNEL_STATE_CONTENT_TYPE =
  "application/vnd.coding-adventures.chief-channel-state-v1";
export const CHANNEL_MESSAGE_CONTENT_TYPE =
  "application/vnd.coding-adventures.chief-channel-message-v1";
export const CHANNEL_GRANT_CONTENT_TYPE =
  "application/vnd.coding-adventures.chief-channel-key-grant-v1";
export const CHANNEL_ACK_CONTENT_TYPE =
  "application/vnd.coding-adventures.chief-channel-ack-v1";
export const MAX_IDENTITY_BYTES = 4 * 1024;
export const MAX_CONTENT_TYPE_BYTES = 1024;
export const MAX_CHANNEL_RECEIVERS = 1024;
export const MAX_PENDING_HEADER_BYTES = 16 * 1024;
export const MAX_CHANNEL_CAS_ATTEMPTS = 16;
export const MAX_DEFINITION_CAS_ATTEMPTS = 16;
export const MAX_U64 = (1n << 64n) - 1n;

export const CHANNEL_ERROR_CODES = [
  "invalid_definition",
  "invalid_message_id",
  "definition_not_found",
  "conflicting_definition",
  "corrupt_definition",
  "definition_changed",
  "channel_destroyed",
  "unauthorized_originator",
  "unauthorized_receiver",
  "public_key_mismatch",
  "missing_key_grant",
  "unknown_message_id",
  "unauthorized_message",
  "not_initialized",
  "corrupt_record",
  "pending_append",
  "no_pending_append",
  "pending_header_mismatch",
  "conflicting_record",
  "concurrent_update",
  "invalid_receiver_id",
  "invalid_page_size",
  "acknowledgement_regression",
  "acknowledgement_ahead",
  "acknowledgement_pending",
  "sequence_exhausted",
  "storage_error",
  "wire_error",
  "crypto_error",
  "metadata_error",
] as const;

export type ChannelErrorCode = (typeof CHANNEL_ERROR_CODES)[number];

/** One fail-closed D18P operation error. Messages never contain secret bytes. */
export class ChannelProfileError extends Error {
  readonly code: ChannelErrorCode;

  constructor(code: ChannelErrorCode) {
    super(code);
    this.name = "ChannelProfileError";
    this.code = code;
  }
}

export type ChannelLifecycle = "active" | "destroyed";

export interface OriginatorIdentity {
  readonly agentId: Uint8Array;
  readonly publicKey: Uint8Array;
}

export interface ReceiverIdentity {
  readonly agentId: Uint8Array;
  readonly publicKey: Uint8Array;
}

export interface ChannelDefinitionParts {
  readonly channelId: Uint8Array;
  readonly originator: OriginatorIdentity;
  readonly receivers: readonly ReceiverIdentity[];
  readonly createdAtNs: bigint;
  readonly keyEpoch: bigint;
  readonly lifecycle?: ChannelLifecycle;
}

/** Immutable canonical D18C channel membership. */
export class ChannelDefinition {
  readonly #channelId: Uint8Array;
  readonly #originator: OriginatorIdentity;
  readonly #receivers: readonly ReceiverIdentity[];
  readonly #createdAtNs: bigint;
  readonly #keyEpoch: bigint;
  readonly #lifecycle: ChannelLifecycle;

  constructor(parts: ChannelDefinitionParts) {
    validateUuidV7(parts.channelId, "invalid_definition");
    validateAgentId(parts.originator.agentId, "invalid_definition");
    requireLength(parts.originator.publicKey, 32, "invalid_definition");
    if (parts.receivers.length < 1 || parts.receivers.length > MAX_CHANNEL_RECEIVERS) {
      fail("invalid_definition");
    }
    requireU64(parts.createdAtNs, "invalid_definition");
    requireU64(parts.keyEpoch, "invalid_definition");
    const originatorId = copy(parts.originator.agentId);
    const receivers = parts.receivers.map((receiver) => {
      validateAgentId(receiver.agentId, "invalid_definition");
      requireLength(receiver.publicKey, 32, "invalid_definition");
      if (bytesEqual(receiver.agentId, originatorId)) fail("invalid_definition");
      return Object.freeze({
        agentId: copy(receiver.agentId),
        publicKey: copy(receiver.publicKey),
      });
    });
    receivers.sort((left, right) => compareBytes(left.agentId, right.agentId));
    for (let index = 1; index < receivers.length; index += 1) {
      if (bytesEqual(receivers[index - 1]!.agentId, receivers[index]!.agentId)) {
        fail("invalid_definition");
      }
    }
    const lifecycle = parts.lifecycle ?? "active";
    if (lifecycle !== "active" && lifecycle !== "destroyed") fail("invalid_definition");
    this.#channelId = copy(parts.channelId);
    this.#originator = Object.freeze({
      agentId: originatorId,
      publicKey: copy(parts.originator.publicKey),
    });
    this.#receivers = Object.freeze(receivers);
    this.#createdAtNs = parts.createdAtNs;
    this.#keyEpoch = parts.keyEpoch;
    this.#lifecycle = lifecycle;
    Object.freeze(this);
  }

  get channelId(): Uint8Array { return copy(this.#channelId); }
  get originator(): OriginatorIdentity { return cloneOriginator(this.#originator); }
  get receivers(): readonly ReceiverIdentity[] {
    return this.#receivers.map((receiver) => cloneReceiver(receiver));
  }
  get createdAtNs(): bigint { return this.#createdAtNs; }
  get keyEpoch(): bigint { return this.#keyEpoch; }
  get lifecycle(): ChannelLifecycle { return this.#lifecycle; }

  receiver(agentId: Uint8Array): ReceiverIdentity | undefined {
    const found = this.#receivers.find((receiver) => bytesEqual(receiver.agentId, agentId));
    return found === undefined ? undefined : cloneReceiver(found);
  }

  withLifecycle(lifecycle: ChannelLifecycle): ChannelDefinition {
    return new ChannelDefinition({
      channelId: this.#channelId,
      originator: this.#originator,
      receivers: this.#receivers,
      createdAtNs: this.#createdAtNs,
      keyEpoch: this.#keyEpoch,
      lifecycle,
    });
  }

  equals(other: ChannelDefinition): boolean {
    return bytesEqual(channelDefinitionSerialize(this), channelDefinitionSerialize(other));
  }
}

export interface MessageHeaderParts {
  readonly messageId: Uint8Array;
  readonly timestampNs: bigint;
  readonly originatorId: Uint8Array;
  readonly channelId: Uint8Array;
  readonly sequence: bigint;
  readonly keyEpoch: bigint;
  readonly contentType: string;
  readonly plaintextHash: Uint8Array;
}

/** Exact D18H value consumed by reserve-before-encrypt recovery. */
export class MessageHeader {
  readonly #parts: MessageHeaderParts;

  constructor(parts: MessageHeaderParts) {
    requireLength(parts.messageId, 16, "wire_error");
    requireU64(parts.timestampNs, "wire_error");
    if (parts.originatorId.length > MAX_IDENTITY_BYTES) fail("wire_error");
    requireLength(parts.channelId, 16, "wire_error");
    requireU64(parts.sequence, "wire_error");
    requireU64(parts.keyEpoch, "wire_error");
    const contentType = encodeUtf8(parts.contentType, "wire_error");
    if (contentType.length > MAX_CONTENT_TYPE_BYTES) fail("wire_error");
    requireLength(parts.plaintextHash, 32, "wire_error");
    this.#parts = Object.freeze({
      messageId: copy(parts.messageId),
      timestampNs: parts.timestampNs,
      originatorId: copy(parts.originatorId),
      channelId: copy(parts.channelId),
      sequence: parts.sequence,
      keyEpoch: parts.keyEpoch,
      contentType: parts.contentType,
      plaintextHash: copy(parts.plaintextHash),
    });
    Object.freeze(this);
  }

  get messageId(): Uint8Array { return copy(this.#parts.messageId); }
  get timestampNs(): bigint { return this.#parts.timestampNs; }
  get originatorId(): Uint8Array { return copy(this.#parts.originatorId); }
  get channelId(): Uint8Array { return copy(this.#parts.channelId); }
  get sequence(): bigint { return this.#parts.sequence; }
  get keyEpoch(): bigint { return this.#parts.keyEpoch; }
  get contentType(): string { return this.#parts.contentType; }
  get plaintextHash(): Uint8Array { return copy(this.#parts.plaintextHash); }

  equals(other: MessageHeader): boolean {
    return bytesEqual(messageHeaderSerialize(this), messageHeaderSerialize(other));
  }
}

export interface ChannelState {
  readonly nextSequence: bigint;
  readonly pendingHeader?: MessageHeader;
}

export function channelDefinitionSerialize(definition: ChannelDefinition): Uint8Array {
  const writer = new ByteWriter();
  writer.bytes(ascii("D18C")).u8(1).bytes(definition.channelId);
  writer.sized32(definition.originator.agentId).bytes(definition.originator.publicKey);
  writer.u32(definition.receivers.length);
  for (const receiver of definition.receivers) {
    writer.sized32(receiver.agentId).bytes(receiver.publicKey);
  }
  writer.u64(definition.createdAtNs).u64(definition.keyEpoch);
  writer.u8(definition.lifecycle === "active" ? 0 : 1);
  return writer.finish();
}

export function channelDefinitionDeserialize(bytes: Uint8Array): ChannelDefinition {
  try {
    const reader = new ByteReader(bytes, "corrupt_definition");
    reader.magic("D18C").version();
    const channelId = reader.bytes(16);
    const originator = {
      agentId: reader.sized32(MAX_IDENTITY_BYTES),
      publicKey: reader.bytes(32),
    };
    const receiverCount = reader.u32();
    if (receiverCount < 1 || receiverCount > MAX_CHANNEL_RECEIVERS) fail("corrupt_definition");
    const receivers: ReceiverIdentity[] = [];
    for (let index = 0; index < receiverCount; index += 1) {
      receivers.push({
        agentId: reader.sized32(MAX_IDENTITY_BYTES),
        publicKey: reader.bytes(32),
      });
    }
    const createdAtNs = reader.u64();
    const keyEpoch = reader.u64();
    const lifecycleByte = reader.u8();
    if (lifecycleByte !== 0 && lifecycleByte !== 1) fail("corrupt_definition");
    reader.finish();
    try {
      return new ChannelDefinition({
        channelId,
        originator,
        receivers,
        createdAtNs,
        keyEpoch,
        lifecycle: lifecycleByte === 0 ? "active" : "destroyed",
      });
    } catch {
      fail("corrupt_definition");
    }
  } catch (error) {
    remap(error, "corrupt_definition");
  }
}

export function messageHeaderSerialize(header: MessageHeader): Uint8Array {
  const writer = new ByteWriter();
  writer.bytes(ascii("D18H")).u8(1).bytes(header.messageId).u64(header.timestampNs);
  writer.sized32(header.originatorId).bytes(header.channelId);
  writer.u64(header.sequence).u64(header.keyEpoch);
  writer.sized32(encodeUtf8(header.contentType, "wire_error")).bytes(header.plaintextHash);
  return writer.finish();
}

export function messageHeaderDeserialize(bytes: Uint8Array): MessageHeader {
  const reader = new ByteReader(bytes, "wire_error");
  reader.magic("D18H").version();
  const messageId = reader.bytes(16);
  const timestampNs = reader.u64();
  const originatorId = reader.sized32(MAX_IDENTITY_BYTES);
  const channelId = reader.bytes(16);
  const sequence = reader.u64();
  const keyEpoch = reader.u64();
  const contentType = decodeUtf8(reader.sized32(MAX_CONTENT_TYPE_BYTES), "wire_error");
  const plaintextHash = reader.bytes(32);
  reader.finish();
  return new MessageHeader({
    messageId,
    timestampNs,
    originatorId,
    channelId,
    sequence,
    keyEpoch,
    contentType,
    plaintextHash,
  });
}

export function channelStateSerialize(state: ChannelState): Uint8Array {
  requireU64(state.nextSequence, "corrupt_record");
  const writer = new ByteWriter();
  writer.bytes(ascii("D18S")).u8(1).u64(state.nextSequence);
  if (state.pendingHeader === undefined) {
    writer.u8(0);
  } else {
    const header = messageHeaderSerialize(state.pendingHeader);
    if (header.length > MAX_PENDING_HEADER_BYTES) fail("corrupt_record");
    writer.u8(1).u32(header.length).bytes(header);
  }
  return writer.finish();
}

export function channelStateDeserialize(bytes: Uint8Array, channelId: Uint8Array): ChannelState {
  try {
    const reader = new ByteReader(bytes, "corrupt_record");
    reader.magic("D18S").version();
    const nextSequence = reader.u64();
    const flag = reader.u8();
    let pendingHeader: MessageHeader | undefined;
    if (flag === 0) {
      reader.finish();
    } else if (flag === 1) {
      const length = reader.u32();
      if (length > MAX_PENDING_HEADER_BYTES) fail("corrupt_record");
      try {
        pendingHeader = messageHeaderDeserialize(reader.bytes(length));
      } catch {
        fail("corrupt_record");
      }
      reader.finish();
      if (
        !bytesEqual(pendingHeader.channelId, channelId) ||
        pendingHeader.sequence === MAX_U64 ||
        pendingHeader.sequence + 1n !== nextSequence
      ) fail("corrupt_record");
    } else {
      fail("corrupt_record");
    }
    return Object.freeze({ nextSequence, pendingHeader });
  } catch (error) {
    remap(error, "corrupt_record");
  }
}

export function receiverCursorSerialize(firstUnreadSequence: bigint): Uint8Array {
  requireU64(firstUnreadSequence, "corrupt_record");
  return new ByteWriter().bytes(ascii("D18A")).u8(1).u64(firstUnreadSequence).finish();
}

export function receiverCursorDeserialize(bytes: Uint8Array): bigint {
  try {
    const reader = new ByteReader(bytes, "corrupt_record");
    reader.magic("D18A").version();
    const cursor = reader.u64();
    reader.finish();
    return cursor;
  } catch (error) {
    remap(error, "corrupt_record");
  }
}

export function channelDefinitionRecordKey(channelId: Uint8Array): string {
  requireLength(channelId, 16, "invalid_definition");
  return `${hex(channelId)}/definition`;
}

export function sequenceStateRecordKey(channelId: Uint8Array): string {
  requireLength(channelId, 16, "invalid_definition");
  return `${hex(channelId)}/state/next-sequence`;
}

export function messageRecordPrefix(channelId: Uint8Array): string {
  requireLength(channelId, 16, "invalid_definition");
  return `${hex(channelId)}/messages/`;
}

export function messageRecordKey(channelId: Uint8Array, sequence: bigint): string {
  return `${messageRecordPrefix(channelId)}${decimal20(sequence)}`;
}

export function keyGrantRecordKey(
  channelId: Uint8Array,
  keyEpoch: bigint,
  receiverId: Uint8Array,
): string {
  validateAgentId(receiverId, "invalid_receiver_id");
  return `${hex(channelId)}/grants/${decimal20(keyEpoch)}/${hex(sha256(receiverId))}`;
}

export function receiverAckRecordKey(channelId: Uint8Array, receiverId: Uint8Array): string {
  validateAgentId(receiverId, "invalid_receiver_id");
  return `${hex(channelId)}/receivers/${hex(sha256(receiverId))}/ack`;
}

export function validateUuidV7(bytes: Uint8Array, code: ChannelErrorCode = "invalid_message_id"): void {
  requireLength(bytes, 16, code);
  if ((bytes[6]! >> 4) !== 7 || (bytes[8]! & 0xc0) !== 0x80) fail(code);
}

export function validateAgentId(bytes: Uint8Array, code: ChannelErrorCode): void {
  if (bytes.length === 0 || bytes.length > MAX_IDENTITY_BYTES) fail(code);
}

export function bytesEqual(left: Uint8Array, right: Uint8Array): boolean {
  if (left.length !== right.length) return false;
  let difference = 0;
  for (let index = 0; index < left.length; index += 1) {
    difference |= left[index]! ^ right[index]!;
  }
  return difference === 0;
}

export function copy(bytes: Uint8Array): Uint8Array { return bytes.slice(); }

export function fail(code: ChannelErrorCode): never { throw new ChannelProfileError(code); }

function cloneOriginator(identity: OriginatorIdentity): OriginatorIdentity {
  return { agentId: copy(identity.agentId), publicKey: copy(identity.publicKey) };
}

function cloneReceiver(identity: ReceiverIdentity): ReceiverIdentity {
  return { agentId: copy(identity.agentId), publicKey: copy(identity.publicKey) };
}

function requireLength(bytes: Uint8Array, length: number, code: ChannelErrorCode): void {
  if (bytes.length !== length) fail(code);
}

function requireU64(value: bigint, code: ChannelErrorCode): void {
  if (value < 0n || value > MAX_U64) fail(code);
}

function decimal20(value: bigint): string {
  requireU64(value, "corrupt_record");
  return value.toString(10).padStart(20, "0");
}

function compareBytes(left: Uint8Array, right: Uint8Array): number {
  const length = Math.min(left.length, right.length);
  for (let index = 0; index < length; index += 1) {
    if (left[index] !== right[index]) return left[index]! - right[index]!;
  }
  return left.length - right.length;
}

function ascii(value: string): Uint8Array { return new TextEncoder().encode(value); }
function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}

function encodeUtf8(value: string, code: ChannelErrorCode): Uint8Array {
  if (/\p{Surrogate}/u.test(value)) fail(code);
  return new TextEncoder().encode(value);
}

function decodeUtf8(bytes: Uint8Array, code: ChannelErrorCode): string {
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    fail(code);
  }
}

function remap(error: unknown, code: ChannelErrorCode): never {
  if (error instanceof ChannelProfileError && error.code === code) throw error;
  fail(code);
}

class ByteWriter {
  readonly #values: number[] = [];
  bytes(bytes: Uint8Array): this { this.#values.push(...bytes); return this; }
  u8(value: number): this { this.#values.push(value & 0xff); return this; }
  u32(value: number): this {
    if (!Number.isInteger(value) || value < 0 || value > 0xffff_ffff) fail("wire_error");
    this.#values.push((value >>> 24) & 0xff, (value >>> 16) & 0xff, (value >>> 8) & 0xff, value & 0xff);
    return this;
  }
  u64(value: bigint): this {
    requireU64(value, "wire_error");
    for (let shift = 56n; shift >= 0n; shift -= 8n) this.#values.push(Number((value >> shift) & 0xffn));
    return this;
  }
  sized32(bytes: Uint8Array): this { return this.u32(bytes.length).bytes(bytes); }
  finish(): Uint8Array { return Uint8Array.from(this.#values); }
}

class ByteReader {
  readonly #bytes: Uint8Array;
  readonly #code: ChannelErrorCode;
  #position = 0;
  constructor(bytes: Uint8Array, code: ChannelErrorCode) { this.#bytes = bytes; this.#code = code; }
  bytes(length: number): Uint8Array {
    if (!Number.isSafeInteger(length) || length < 0 || this.#position + length > this.#bytes.length) fail(this.#code);
    const value = this.#bytes.slice(this.#position, this.#position + length);
    this.#position += length;
    return value;
  }
  u8(): number { return this.bytes(1)[0]!; }
  u32(): number {
    const value = this.bytes(4);
    return value[0]! * 0x1000000 + value[1]! * 0x10000 + value[2]! * 0x100 + value[3]!;
  }
  u64(): bigint {
    let value = 0n;
    for (const byte of this.bytes(8)) value = (value << 8n) | BigInt(byte);
    return value;
  }
  sized32(maximum: number): Uint8Array {
    const length = this.u32();
    if (length > maximum) fail(this.#code);
    return this.bytes(length);
  }
  magic(expected: string): this { if (!bytesEqual(this.bytes(4), ascii(expected))) fail(this.#code); return this; }
  version(): this { if (this.u8() !== 1) fail(this.#code); return this; }
  finish(): void { if (this.#position !== this.#bytes.length) fail(this.#code); }
}
