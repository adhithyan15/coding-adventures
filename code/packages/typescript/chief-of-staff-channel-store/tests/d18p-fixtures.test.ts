import { readFileSync } from "node:fs";
import { messageSerialize } from "@coding-adventures/chief-of-staff-channel-crypto";
import { describe, expect, it } from "vitest";
import {
  CHANNEL_ACK_CONTENT_TYPE,
  CHANNEL_DEFINITION_CONTENT_TYPE,
  CHANNEL_ERROR_CODES,
  CHANNEL_GRANT_CONTENT_TYPE,
  CHANNEL_MESSAGE_CONTENT_TYPE,
  CHANNEL_STATE_CONTENT_TYPE,
  CHANNEL_STORAGE_NAMESPACE,
  ChannelDefinition,
  ChannelDefinitionStore,
  ChannelProfileError,
  ChannelStore,
  DurableOriginator,
  DurableReceiver,
  MAX_CHANNEL_CAS_ATTEMPTS,
  MAX_CHANNEL_RECEIVERS,
  MAX_DEFINITION_CAS_ATTEMPTS,
  MAX_PENDING_HEADER_BYTES,
  MemoryChannelStorage,
  MessageHeader,
  type MessageMetadata,
  type ReceiverEpochKeyProvider,
  channelDefinitionDeserialize,
  channelDefinitionRecordKey,
  channelDefinitionSerialize,
  channelStateDeserialize,
  channelStateSerialize,
  keyGrantRecordKey,
  messageRecordKey,
  messageRecordPrefix,
  receiverAckRecordKey,
  receiverCursorDeserialize,
  receiverCursorSerialize,
  sequenceStateRecordKey,
} from "../src/index.js";

interface Fixture {
  fixture_format: string;
  generator_blob_sha1: string;
  constants: {
    storage_namespace: string;
    content_types: Record<string, string>;
    max_receivers: string;
    max_pending_header_bytes: string;
    max_store_cas_attempts: string;
    max_definition_cas_attempts: string;
  };
  test_keys: Record<string, string>;
  definition_cases: Array<{ name: string; lifecycle: "active" | "destroyed"; d18c_b64: string; canonical_receiver_ids_b64?: string[] }>;
  state_cases: Array<{ name: string; next_sequence: string; pending: boolean; d18h_b64?: string; d18s_b64: string }>;
  cursor_cases: Array<{ first_unread_sequence: string; d18a_b64: string }>;
  storage_key_cases: Array<{ name: string; expected_key: string }>;
  operation_cases: Array<Record<string, unknown> & { name: string }>;
  codec_negative_cases: Array<{ name: string; kind: "definition" | "state" | "cursor"; record_b64: string; expected_error: string }>;
  operation_negative_cases: Array<{ name: string; operation: string; expected_error: string }>;
  oversize_recipes: Array<{ field: string; declared_length: string; expected_error: string }>;
  stable_error_codes: string[];
}

const fixture = JSON.parse(readFileSync(
  new URL("../../../../fixtures/chief-of-staff-channel/v1/manifest.json", import.meta.url),
  "utf8",
)) as Fixture;
const activeBytes = fromBase64(fixture.definition_cases[0]!.d18c_b64);
const activeDefinition = channelDefinitionDeserialize(activeBytes);
const channelId = activeDefinition.channelId;
const originatorId = activeDefinition.originator.agentId;
const binaryReceiverId = fromBase64(fixture.definition_cases[0]!.canonical_receiver_ids_b64![0]!);
const textReceiverId = fromBase64(fixture.definition_cases[0]!.canonical_receiver_ids_b64![1]!);
const signingSecretKey = concat(
  fromHex(fixture.test_keys.originator_signing_seed_hex!),
  fromHex(fixture.test_keys.originator_public_key_hex!),
);
const channelMasterKey = fromHex(fixture.test_keys.channel_master_key_hex!);
const EXPECTED_OPERATION_ERRORS = {
  "conflicting-definition": "conflicting_definition",
  "session-delivery-enforcement": "unknown_message_id",
  "unauthorized-originator": "unauthorized_originator",
  "unauthorized-receiver": "unauthorized_receiver",
  "receiver-public-key-mismatch": "public_key_mismatch",
  "channel-destroyed": "channel_destroyed",
  "missing-key-grant": "missing_key_grant",
  "pending-append": "pending_append",
  "acknowledgement-pending": "acknowledgement_pending",
  "pending-header-mismatch": "pending_header_mismatch",
  "no-pending-append": "no_pending_append",
  "invalid-page-size": "invalid_page_size",
  "invalid-receiver-id": "invalid_receiver_id",
  "acknowledgement-ahead": "acknowledgement_ahead",
  "acknowledgement-regression": "acknowledgement_regression",
  "message-key-body-mismatch": "corrupt_record",
  "message-content-type-mismatch": "corrupt_record",
} as const;

describe("D18P shared codec fixture", () => {
  it("locks provenance, constants, and the closed error roster", () => {
    expect(fixture.fixture_format).toBe("D18P-durable-channel-fixtures-v1");
    expect(fixture.generator_blob_sha1).toMatch(/^[0-9a-f]{40}$/);
    expect(fixture.constants).toEqual({
      storage_namespace: CHANNEL_STORAGE_NAMESPACE,
      content_types: {
        definition: CHANNEL_DEFINITION_CONTENT_TYPE,
        state: CHANNEL_STATE_CONTENT_TYPE,
        message: CHANNEL_MESSAGE_CONTENT_TYPE,
        grant: CHANNEL_GRANT_CONTENT_TYPE,
        ack: CHANNEL_ACK_CONTENT_TYPE,
      },
      max_receivers: String(MAX_CHANNEL_RECEIVERS),
      max_pending_header_bytes: String(MAX_PENDING_HEADER_BYTES),
      max_store_cas_attempts: String(MAX_CHANNEL_CAS_ATTEMPTS),
      max_definition_cas_attempts: String(MAX_DEFINITION_CAS_ATTEMPTS),
    });
    expect([...CHANNEL_ERROR_CODES]).toEqual(fixture.stable_error_codes);
    expect(fixture.codec_negative_cases).toHaveLength(19);
    expect(Object.fromEntries(
      fixture.operation_negative_cases.map((testCase) => [testCase.name, testCase.expected_error]),
    )).toEqual(EXPECTED_OPERATION_ERRORS);
  });

  it("round-trips canonical active/destroyed definitions and owns inputs", () => {
    for (const testCase of fixture.definition_cases) {
      const bytes = fromBase64(testCase.d18c_b64);
      const definition = channelDefinitionDeserialize(bytes);
      expect(definition.lifecycle, testCase.name).toBe(testCase.lifecycle);
      expect(channelDefinitionSerialize(definition), testCase.name).toEqual(bytes);
    }
    expect(activeDefinition.receivers.map((receiver) => toBase64(receiver.agentId)))
      .toEqual(fixture.definition_cases[0]!.canonical_receiver_ids_b64);
    const mutable = activeDefinition.channelId;
    mutable.fill(0);
    expect(channelDefinitionSerialize(activeDefinition)).toEqual(activeBytes);
    expect(Object.isFrozen(activeDefinition)).toBe(true);
  });

  it("round-trips initial/pending state and every cursor byte-identically", () => {
    for (const testCase of fixture.state_cases) {
      const bytes = fromBase64(testCase.d18s_b64);
      const state = channelStateDeserialize(bytes, channelId);
      expect(state.nextSequence, testCase.name).toBe(BigInt(testCase.next_sequence));
      expect(state.pendingHeader !== undefined, testCase.name).toBe(testCase.pending);
      expect(channelStateSerialize(state), testCase.name).toEqual(bytes);
      if (testCase.d18h_b64 !== undefined) {
        expect(state.pendingHeader).toBeDefined();
        expect(channelStateSerialize(state).slice(18)).toEqual(fromBase64(testCase.d18h_b64));
      }
    }
    for (const testCase of fixture.cursor_cases) {
      const bytes = fromBase64(testCase.d18a_b64);
      const cursor = receiverCursorDeserialize(bytes);
      expect(cursor).toBe(BigInt(testCase.first_unread_sequence));
      expect(receiverCursorSerialize(cursor)).toEqual(bytes);
    }
  });

  it("reproduces every deterministic key including binary receiver hashes", () => {
    const actual: Record<string, string> = {
      definition: channelDefinitionRecordKey(channelId),
      state: sequenceStateRecordKey(channelId),
      "message-zero": messageRecordKey(channelId, 0n),
      "message-max": messageRecordKey(channelId, (1n << 64n) - 1n),
      "message-prefix": messageRecordPrefix(channelId),
      grant: keyGrantRecordKey(channelId, 7n, binaryReceiverId),
      "ack-binary-receiver": receiverAckRecordKey(channelId, binaryReceiverId),
    };
    for (const testCase of fixture.storage_key_cases) {
      expect(actual[testCase.name], testCase.name).toBe(testCase.expected_key);
    }
  });

  it("maps every malformed codec record to its declared stable error", () => {
    for (const testCase of fixture.codec_negative_cases) {
      expectCode(() => {
        const bytes = fromBase64(testCase.record_b64);
        if (testCase.kind === "definition") channelDefinitionDeserialize(bytes);
        else if (testCase.kind === "state") channelStateDeserialize(bytes, channelId);
        else receiverCursorDeserialize(bytes);
      }, testCase.expected_error, testCase.name);
    }
  });

  it("materializes all compact oversize recipes", () => {
    const tooLargeId = new Uint8Array(Number(fixture.oversize_recipes[0]!.declared_length));
    expectCode(() => new ChannelDefinition({
      channelId,
      originator: { agentId: tooLargeId, publicKey: activeDefinition.originator.publicKey },
      receivers: activeDefinition.receivers,
      createdAtNs: 0n,
      keyEpoch: 0n,
    }), "invalid_definition", "agent-id");
    expectCode(() => new ChannelDefinition({
      channelId,
      originator: activeDefinition.originator,
      receivers: Array.from({ length: Number(fixture.oversize_recipes[1]!.declared_length) }, (_, index) => ({
        agentId: Uint8Array.of(index >>> 8, index & 0xff),
        publicKey: new Uint8Array(32),
      })),
      createdAtNs: 0n,
      keyEpoch: 0n,
    }), "invalid_definition", "receiver-count");
    const oversizedState = Uint8Array.of(68, 49, 56, 83, 1, 0, 0, 0, 0, 0, 0, 0, 1, 1, 0, 0, 64, 1);
    expectCode(() => channelStateDeserialize(oversizedState, channelId), "corrupt_record", "pending-header");
  });
});

describe("D18P durable transitions", () => {
  it("creates definitions idempotently and rejects conflicting content", async () => {
    const expected = operation("definition-create-idempotent");
    const backend = new MemoryChannelStorage();
    const definitions = new ChannelDefinitionStore(backend);
    const first = await definitions.create(activeDefinition);
    const second = await definitions.create(activeDefinition);
    expect(first.equals(second)).toBe(expected.definitions_equal);
    expect(String((await new ChannelStore(backend, channelId).state()).nextSequence))
      .toBe(expected.initial_next_sequence);
    const conflict = new ChannelDefinition({
      channelId,
      originator: activeDefinition.originator,
      receivers: activeDefinition.receivers,
      createdAtNs: activeDefinition.createdAtNs + 1n,
      keyEpoch: activeDefinition.keyEpoch,
    });
    await expectCodeAsync(() => definitions.create(conflict), "conflicting_definition");
  });

  it("replays recovery, exact D18M retry, abandonment gaps, paging, and random access", async () => {
    const backend = new MemoryChannelStorage();
    const store = new ChannelStore(backend, channelId);
    await store.initialize();
    const header = await store.reserveAppend(request(20, 20_000_000_020n), utf8("recoverable"));
    const recovered = new ChannelStore(backend, channelId);
    const expected = operation("reserve-recover-complete-retry-abandon-gap");
    expect((await recovered.initialize()).pendingHeader?.equals(header)).toBe(expected.recovered_pending_equal);
    await expectCodeAsync(
      () => store.reserveAppend(request(21, 20_000_000_021n), utf8("pending")),
      "pending_append",
    );
    await expectCodeAsync(() => store.acknowledge(binaryReceiverId, 0n), "acknowledgement_pending");
    const mismatch = new MessageHeader({ ...request(22, 20_000_000_022n), channelId, sequence: 0n, plaintextHash: header.plaintextHash });
    await expectCodeAsync(
      () => recovered.commitReserved(mismatch, utf8("recoverable"), channelMasterKey, signingSecretKey),
      "pending_header_mismatch",
    );
    const first = await recovered.commitReserved(header, utf8("recoverable"), channelMasterKey, signingSecretKey);
    const retry = await recovered.commitReserved(header, utf8("recoverable"), channelMasterKey, signingSecretKey);
    expect(messageSerialize(first)).toEqual(fromBase64(expected.first_d18m_b64 as string));
    expect(bytesAreEqual(messageSerialize(retry), messageSerialize(first))).toBe(expected.commit_retry_equal);

    const abandoned = await recovered.reserveAppend(request(23, 20_000_000_023n), utf8("abandoned"));
    expect(String((await recovered.abandonPending())?.sequence)).toBe(expected.abandoned_sequence);
    await expectCodeAsync(
      () => recovered.commitReserved(abandoned, utf8("abandoned"), channelMasterKey, signingSecretKey),
      "no_pending_append",
    );
    const afterGap = await recovered.append(
      request(24, 20_000_000_024n), utf8("after gap"), channelMasterKey, signingSecretKey,
    );
    expect(String(afterGap.sequence)).toBe(expected.after_gap_sequence);
    expect((await recovered.readMessages(0n, 10)).messages.map((message) => String(message.sequence)))
      .toEqual(expected.read_sequences);
    const firstPage = await recovered.readMessages(0n, 1);
    expect(firstPage.messages.map((message) => String(message.sequence))).toEqual(expected.first_page_sequences);
    expect(String(firstPage.nextStart)).toBe(expected.first_page_next_start);
    expect((await recovered.readMessages(firstPage.nextStart!, 1)).messages.map((message) => String(message.sequence)))
      .toEqual(expected.second_page_sequences);
    expect((await recovered.readMessages(2n, 10)).messages.map((message) => String(message.sequence)))
      .toEqual(expected.random_access_sequences);
    expect((await recovered.readMessages(3n, 10)).messages.length === 0).toBe(expected.empty_continuation);
    await expectCodeAsync(() => recovered.readMessages(0n, 0), "invalid_page_size");
    await expectCodeAsync(() => recovered.acknowledge(binaryReceiverId, 3n), "acknowledgement_ahead");
    expect(await recovered.acknowledge(binaryReceiverId, 0n)).toBe(1n);
    expect(await recovered.acknowledge(binaryReceiverId, 2n)).toBe(3n);
    await expectCodeAsync(() => recovered.acknowledge(binaryReceiverId, 0n), "acknowledgement_regression");
    await expectCodeAsync(() => recovered.receiverCursor(new Uint8Array()), "invalid_receiver_id");
  });

  it("round-trips encrypted endpoints with independent cursors and session-bound acks", async () => {
    const expected = operation("encrypted-endpoint-round-trip-independent-cursors");
    const backend = new MemoryChannelStorage();
    const definitions = new ChannelDefinitionStore(backend);
    await definitions.create(activeDefinition);
    const metadata = metadataSource([
      { messageId: uuidV7(1), timestampNs: 10_000_000_001n },
      { messageId: uuidV7(2), timestampNs: 10_000_000_002n },
      { messageId: uuidV7(3), timestampNs: 10_000_000_003n },
    ]);
    const originator = await DurableOriginator.open(
      backend, channelId, originatorId, signingSecretKey, channelMasterKey, metadata,
    );
    await originator.saveReceiverGrant(binaryReceiverId, Uint8Array.of(1));
    await originator.saveReceiverGrant(textReceiverId, Uint8Array.of(2));
    const first = await originator.publish(utf8("message zero"), "text/plain");
    const second = await originator.publish(utf8("message one"), "application/octet-stream");
    expect([String(first.sequence), String(second.sequence)])
      .toEqual(expected.published_sequences);

    const binary = await DurableReceiver.open(
      backend, channelId, binaryReceiverId, providerFor(binaryReceiverId),
    );
    const binaryZero = await binary.receive(1);
    expect(binaryZero.map((message) => String(message.sequence))).toEqual(
      (expected.binary_receiver_delivered_sequences as string[]).slice(0, 1),
    );
    expect(String(await binary.acknowledge(binaryZero[0]!.messageId)))
      .toBe(expected.binary_first_unread_after_zero);
    const binaryOne = await binary.receive(10);
    expect([...binaryZero, ...binaryOne].map((message) => String(message.sequence)))
      .toEqual(expected.binary_receiver_delivered_sequences);
    expect(String(await binary.acknowledge(binaryOne[0]!.messageId)))
      .toBe(expected.binary_first_unread_after_one);
    expect(String(await binary.acknowledge(binaryOne[0]!.messageId)))
      .toBe(expected.binary_first_unread_after_retry);
    expect((await binary.receive(10)).length === 0).toBe(expected.binary_empty_continuation);

    const text = await DurableReceiver.open(backend, channelId, textReceiverId, providerFor(textReceiverId));
    const textMessages = await text.receive(10);
    expect(textMessages.map((message) => String(message.sequence)))
      .toEqual(expected.text_receiver_delivered_sequences);
    expect(String(await text.acknowledge(textMessages[0]!.messageId)))
      .toBe(expected.text_first_unread_after_zero);
    expect(String(await new ChannelStore(backend, channelId).receiverCursor(binaryReceiverId)))
      .toBe(expected.binary_first_unread_after_retry);
    expect(String(await new ChannelStore(backend, channelId).receiverCursor(textReceiverId)))
      .toBe(expected.text_first_unread_after_zero);

    const failingKeyProvider = await DurableReceiver.open(backend, channelId, textReceiverId, {
      publicKey: activeDefinition.receiver(textReceiverId)!.publicKey,
      openGrant: () => { throw new Error("provider details must not escape"); },
    });
    await expectCodeAsync(() => failingKeyProvider.receive(1), "crypto_error");

    const fresh = await DurableReceiver.open(backend, channelId, binaryReceiverId, providerFor(binaryReceiverId));
    await expectCodeAsync(() => fresh.acknowledge(first.messageId), "unknown_message_id");
    await expectCodeAsync(
      () => DurableOriginator.open(backend, channelId, utf8("intruder"), signingSecretKey, channelMasterKey, metadata),
      "unauthorized_originator",
    );
    await expectCodeAsync(
      () => DurableReceiver.open(backend, channelId, utf8("intruder"), providerFor(binaryReceiverId)),
      "unauthorized_receiver",
    );
    await expectCodeAsync(
      () => DurableReceiver.open(backend, channelId, binaryReceiverId, { publicKey: new Uint8Array(32), openGrant: () => channelMasterKey }),
      "public_key_mismatch",
    );

    const firstDestroyed = await definitions.destroy(channelId);
    const retriedDestroyed = await definitions.destroy(channelId);
    const destroyed = operation("destroy-idempotent-history-preserved");
    expect(firstDestroyed.equals(retriedDestroyed)).toBe(destroyed.definitions_equal);
    expect(String((await new ChannelStore(backend, channelId).readMessages(0n, 10)).messages.length))
      .toBe(destroyed.history_count);
    await expectCodeAsync(() => originator.publish(utf8("denied"), "text/plain"), "channel_destroyed");
  });

  it("fails closed on missing grants and corrupt message envelopes", async () => {
    const missingBackend = new MemoryChannelStorage();
    await new ChannelDefinitionStore(missingBackend).create(activeDefinition);
    const originator = await DurableOriginator.open(
      missingBackend, channelId, originatorId, signingSecretKey, channelMasterKey,
      metadataSource([{ messageId: uuidV7(9), timestampNs: 10_000_000_009n }]),
    );
    await originator.publish(utf8("no grant"), "text/plain");
    const receiver = await DurableReceiver.open(
      missingBackend, channelId, binaryReceiverId, providerFor(binaryReceiverId),
    );
    await expectCodeAsync(() => receiver.receive(1), "missing_key_grant");

    const keyMismatchBackend = await backendWithOneMessage();
    const original = await keyMismatchBackend.get(CHANNEL_STORAGE_NAMESPACE, messageRecordKey(channelId, 0n));
    keyMismatchBackend.corrupt({ ...original!, key: messageRecordKey(channelId, 1n) });
    await expectCodeAsync(
      () => new ChannelStore(keyMismatchBackend, channelId).readMessages(0n, 10),
      "corrupt_record",
    );

    const typeBackend = await backendWithOneMessage();
    const typed = await typeBackend.get(CHANNEL_STORAGE_NAMESPACE, messageRecordKey(channelId, 0n));
    typeBackend.corrupt({ ...typed!, contentType: "application/octet-stream" });
    await expectCodeAsync(
      () => new ChannelStore(typeBackend, channelId).readMessages(0n, 10),
      "corrupt_record",
    );
  });
});

async function backendWithOneMessage(): Promise<MemoryChannelStorage> {
  const backend = new MemoryChannelStorage();
  const store = new ChannelStore(backend, channelId);
  await store.initialize();
  await store.append(request(30, 30n), utf8("record"), channelMasterKey, signingSecretKey);
  return backend;
}

function request(byte: number, timestampNs: bigint) {
  return {
    messageId: uuidV7(byte), timestampNs, originatorId, keyEpoch: 0n, contentType: "text/plain",
  };
}

function uuidV7(byte: number): Uint8Array {
  const value = new Uint8Array(16).fill(byte);
  value[6] = 0x70 | (byte & 0x0f);
  value[8] = 0x80 | (byte & 0x3f);
  return value;
}

function metadataSource(values: MessageMetadata[]) {
  const queue = [...values];
  return { next: () => {
    const value = queue.shift();
    if (value === undefined) throw new Error("metadata exhausted");
    return value;
  } };
}

function providerFor(receiverId: Uint8Array): ReceiverEpochKeyProvider {
  const receiver = activeDefinition.receiver(receiverId)!;
  return {
    publicKey: receiver.publicKey,
    openGrant: (_epoch, _grant) => channelMasterKey,
  };
}

function operation(name: string): Record<string, unknown> {
  return fixture.operation_cases.find((testCase) => testCase.name === name)!;
}

function expectCode(operation: () => unknown, code: string, name?: string): void {
  try { operation(); }
  catch (error) {
    expect(error, name).toBeInstanceOf(ChannelProfileError);
    expect((error as ChannelProfileError).code, name).toBe(code);
    return;
  }
  throw new Error(`${name ?? code}: expected ${code}`);
}

async function expectCodeAsync(operation: () => Promise<unknown>, code: string): Promise<void> {
  try { await operation(); }
  catch (error) {
    expect(error).toBeInstanceOf(ChannelProfileError);
    expect((error as ChannelProfileError).code).toBe(code);
    return;
  }
  throw new Error(`expected ${code}`);
}

function fromBase64(value: string): Uint8Array { return new Uint8Array(Buffer.from(value, "base64")); }
function toBase64(value: Uint8Array): string { return Buffer.from(value).toString("base64"); }
function fromHex(value: string): Uint8Array { return new Uint8Array(Buffer.from(value, "hex")); }
function concat(...values: Uint8Array[]): Uint8Array {
  const output = new Uint8Array(values.reduce((sum, value) => sum + value.length, 0));
  let offset = 0;
  for (const value of values) { output.set(value, offset); offset += value.length; }
  return output;
}
function bytesAreEqual(left: Uint8Array, right: Uint8Array): boolean {
  return left.length === right.length && left.every((byte, index) => byte === right[index]);
}
function utf8(value: string): Uint8Array { return new TextEncoder().encode(value); }
