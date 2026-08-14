/** Injected atomic storage and the authority-free D18P durable channel store. */

import {
  type D18Message,
  type MessageFields,
  MessageProfileError,
  messageCreate,
  messageDeserialize,
  messageSerialize,
  validateMessageFields,
} from "@coding-adventures/chief-of-staff-channel-crypto";
import { sha256 } from "@coding-adventures/sha256";
import {
  CHANNEL_ACK_CONTENT_TYPE,
  CHANNEL_GRANT_CONTENT_TYPE,
  CHANNEL_MESSAGE_CONTENT_TYPE,
  CHANNEL_STATE_CONTENT_TYPE,
  CHANNEL_STORAGE_NAMESPACE,
  ChannelProfileError,
  type ChannelState,
  MAX_CHANNEL_CAS_ATTEMPTS,
  MAX_U64,
  MessageHeader,
  bytesEqual,
  channelStateDeserialize,
  channelStateSerialize,
  copy,
  fail,
  keyGrantRecordKey,
  messageRecordKey,
  messageRecordPrefix,
  receiverAckRecordKey,
  receiverCursorDeserialize,
  receiverCursorSerialize,
  sequenceStateRecordKey,
  validateAgentId,
  validateUuidV7,
} from "./profile.js";

export interface StorageRecord {
  readonly namespace: string;
  readonly key: string;
  readonly contentType: string;
  readonly metadata: Readonly<Record<string, never>>;
  readonly body: Uint8Array;
  readonly revision: string;
}

export interface StoragePut {
  readonly namespace: string;
  readonly key: string;
  readonly contentType: string;
  readonly metadata: Readonly<Record<string, never>>;
  readonly body: Uint8Array;
  readonly ifAbsent?: true;
  readonly ifRevision?: string;
}

export interface StorageListOptions {
  readonly prefix: string;
  readonly recursive: true;
  readonly pageSize: number;
  readonly cursor?: string;
}

export interface StoragePage {
  readonly records: readonly StorageRecord[];
  readonly nextCursor?: string;
}

export interface ChannelStorageBackend {
  initialize(): Promise<void>;
  get(namespace: string, key: string): Promise<StorageRecord | undefined>;
  put(input: StoragePut): Promise<StorageRecord>;
  list(namespace: string, options: StorageListOptions): Promise<StoragePage>;
}

/** Expected failure of an atomic create or revision-CAS condition. */
export class StorageConflictError extends Error {
  constructor() { super("storage conflict"); this.name = "StorageConflictError"; }
}

/** Deterministic in-memory backend used by portable conformance tests. */
export class MemoryChannelStorage implements ChannelStorageBackend {
  readonly #records = new Map<string, StorageRecord>();
  #revision = 0;

  async initialize(): Promise<void> {}

  async get(namespace: string, key: string): Promise<StorageRecord | undefined> {
    const record = this.#records.get(storageMapKey(namespace, key));
    return record === undefined ? undefined : cloneRecord(record);
  }

  async put(input: StoragePut): Promise<StorageRecord> {
    if ((input.ifAbsent === true) === (input.ifRevision !== undefined)) {
      throw new Error("exactly one storage condition is required");
    }
    const mapKey = storageMapKey(input.namespace, input.key);
    const current = this.#records.get(mapKey);
    if (input.ifAbsent === true) {
      if (current !== undefined) throw new StorageConflictError();
    } else if (current === undefined || current.revision !== input.ifRevision) {
      throw new StorageConflictError();
    }
    this.#revision += 1;
    const record: StorageRecord = Object.freeze({
      namespace: input.namespace,
      key: input.key,
      contentType: input.contentType,
      metadata: Object.freeze({}),
      body: copy(input.body),
      revision: `r${this.#revision}`,
    });
    this.#records.set(mapKey, record);
    return cloneRecord(record);
  }

  async list(namespace: string, options: StorageListOptions): Promise<StoragePage> {
    if (options.pageSize <= 0) throw new Error("invalid backend page size");
    const records = [...this.#records.values()]
      .filter((record) => record.namespace === namespace)
      .filter((record) => record.key.startsWith(options.prefix))
      .filter((record) => options.cursor === undefined || record.key > options.cursor)
      .sort((left, right) => left.key < right.key ? -1 : left.key > right.key ? 1 : 0);
    const selected = records.slice(0, options.pageSize).map(cloneRecord);
    return {
      records: selected,
      nextCursor: records.length > selected.length ? selected.at(-1)?.key : undefined,
    };
  }

  /** Replace a record for negative-test corruption without weakening put(). */
  corrupt(record: StorageRecord): void {
    this.#records.set(storageMapKey(record.namespace, record.key), cloneRecord(record));
  }
}

export interface AppendRequest {
  readonly messageId: Uint8Array;
  readonly timestampNs: bigint;
  readonly originatorId: Uint8Array;
  readonly keyEpoch: bigint;
  readonly contentType: string;
}

export interface MessagePage {
  readonly messages: readonly D18Message[];
  readonly nextStart?: bigint;
}

export interface OpaqueKeyGrant {
  readonly channelId: Uint8Array;
  readonly keyEpoch: bigint;
  readonly receiverId: Uint8Array;
  readonly body: Uint8Array;
}

/** CAS-backed view of one encrypted channel. */
export class ChannelStore {
  readonly #backend: ChannelStorageBackend;
  readonly #channelId: Uint8Array;

  constructor(backend: ChannelStorageBackend, channelId: Uint8Array) {
    validateUuidV7(channelId, "corrupt_record");
    this.#backend = backend;
    this.#channelId = copy(channelId);
  }

  async initialize(): Promise<ChannelState> {
    await storage(() => this.#backend.initialize());
    const existing = await this.#stateRecord();
    if (existing !== undefined) return decodeStateRecord(existing, this.#channelId);
    const body = channelStateSerialize({ nextSequence: 0n });
    try {
      const record = await this.#backend.put(putInput(
        sequenceStateRecordKey(this.#channelId), CHANNEL_STATE_CONTENT_TYPE, body, { ifAbsent: true },
      ));
      return decodeStateRecord(record, this.#channelId);
    } catch (error) {
      if (error instanceof StorageConflictError) return this.state();
      throw storageError(error);
    }
  }

  async state(): Promise<ChannelState> {
    const record = await this.#stateRecord();
    if (record === undefined) fail("not_initialized");
    return decodeStateRecord(record, this.#channelId);
  }

  async reserveAppend(request: AppendRequest, plaintext: Uint8Array): Promise<MessageHeader> {
    validateUuidV7(request.messageId);
    const fields: MessageFields = {
      messageId: request.messageId,
      timestampNs: request.timestampNs,
      originatorId: request.originatorId,
      channelId: this.#channelId,
      sequence: 0n,
      keyEpoch: request.keyEpoch,
      contentType: request.contentType,
    };
    try { validateMessageFields(fields); } catch { fail("crypto_error"); }
    for (let attempt = 0; attempt < MAX_CHANNEL_CAS_ATTEMPTS; attempt += 1) {
      const record = await this.#stateRecord();
      if (record === undefined) fail("not_initialized");
      const current = decodeStateRecord(record, this.#channelId);
      if (current.pendingHeader !== undefined) fail("pending_append");
      if (current.nextSequence === MAX_U64) fail("sequence_exhausted");
      const header = new MessageHeader({
        ...request,
        channelId: this.#channelId,
        sequence: current.nextSequence,
        plaintextHash: sha256(plaintext),
      });
      const updated = channelStateSerialize({
        nextSequence: current.nextSequence + 1n,
        pendingHeader: header,
      });
      try {
        await this.#backend.put(putInput(
          sequenceStateRecordKey(this.#channelId), CHANNEL_STATE_CONTENT_TYPE, updated,
          { ifRevision: record.revision },
        ));
        return header;
      } catch (error) {
        if (error instanceof StorageConflictError) continue;
        throw storageError(error);
      }
    }
    fail("concurrent_update");
  }

  async commitReserved(
    header: MessageHeader,
    plaintext: Uint8Array,
    channelMasterKey: Uint8Array,
    signingSecretKey: Uint8Array,
  ): Promise<D18Message> {
    if (!bytesEqual(header.channelId, this.#channelId)) fail("pending_header_mismatch");
    const state = await this.state();
    if (state.pendingHeader === undefined) {
      const key = messageRecordKey(this.#channelId, header.sequence);
      const record = await storage(() => this.#backend.get(CHANNEL_STORAGE_NAMESPACE, key));
      if (record === undefined) fail("no_pending_append");
      requireContentType(record, CHANNEL_MESSAGE_CONTENT_TYPE);
      const stored = decodeMessage(record.body);
      if (!messageMatchesHeader(stored, header)) fail("conflicting_record");
      const expected = createMessage(header, plaintext, signingSecretKey, channelMasterKey);
      if (!bytesEqual(messageSerialize(expected), record.body)) fail("conflicting_record");
      return stored;
    }
    if (!state.pendingHeader.equals(header)) fail("pending_header_mismatch");
    const message = createMessage(header, plaintext, signingSecretKey, channelMasterKey);
    await this.#putIdempotent(
      messageRecordKey(this.#channelId, header.sequence),
      CHANNEL_MESSAGE_CONTENT_TYPE,
      messageSerialize(message),
    );
    await this.#clearPending(header);
    return message;
  }

  async append(
    request: AppendRequest,
    plaintext: Uint8Array,
    channelMasterKey: Uint8Array,
    signingSecretKey: Uint8Array,
  ): Promise<D18Message> {
    const header = await this.reserveAppend(request, plaintext);
    return this.commitReserved(header, plaintext, channelMasterKey, signingSecretKey);
  }

  async abandonPending(): Promise<MessageHeader | undefined> {
    for (let attempt = 0; attempt < MAX_CHANNEL_CAS_ATTEMPTS; attempt += 1) {
      const record = await this.#stateRecord();
      if (record === undefined) fail("not_initialized");
      const current = decodeStateRecord(record, this.#channelId);
      if (current.pendingHeader === undefined) return undefined;
      try {
        await this.#backend.put(putInput(
          sequenceStateRecordKey(this.#channelId), CHANNEL_STATE_CONTENT_TYPE,
          channelStateSerialize({ nextSequence: current.nextSequence }),
          { ifRevision: record.revision },
        ));
        return current.pendingHeader;
      } catch (error) {
        if (error instanceof StorageConflictError) continue;
        throw storageError(error);
      }
    }
    fail("concurrent_update");
  }

  async readMessages(start: bigint, pageSize: number): Promise<MessagePage> {
    if (!Number.isSafeInteger(pageSize) || pageSize <= 0) fail("invalid_page_size");
    const cursor = start > 0n ? messageRecordKey(this.#channelId, start - 1n) : undefined;
    const page = await storage(() => this.#backend.list(CHANNEL_STORAGE_NAMESPACE, {
      prefix: messageRecordPrefix(this.#channelId), recursive: true, pageSize, cursor,
    }));
    const messages: D18Message[] = [];
    for (const record of page.records) {
      requireContentType(record, CHANNEL_MESSAGE_CONTENT_TYPE);
      const message = decodeMessage(record.body);
      if (
        !bytesEqual(message.channelId, this.#channelId) ||
        message.sequence < start ||
        record.key !== messageRecordKey(this.#channelId, message.sequence) ||
        (messages.at(-1)?.sequence ?? -1n) >= message.sequence
      ) fail("corrupt_record");
      messages.push(message);
    }
    let nextStart: bigint | undefined;
    if (page.nextCursor !== undefined) {
      const last = messages.at(-1);
      if (last === undefined || last.sequence === MAX_U64) fail("corrupt_record");
      nextStart = last.sequence + 1n;
    }
    return Object.freeze({ messages: Object.freeze(messages), nextStart });
  }

  async readForReceiver(receiverId: Uint8Array, pageSize: number): Promise<MessagePage> {
    return this.readMessages(await this.receiverCursor(receiverId), pageSize);
  }

  async receiverCursor(receiverId: Uint8Array): Promise<bigint> {
    validateAgentId(receiverId, "invalid_receiver_id");
    const key = receiverAckRecordKey(this.#channelId, receiverId);
    const record = await storage(() => this.#backend.get(CHANNEL_STORAGE_NAMESPACE, key));
    if (record === undefined) return 0n;
    requireContentType(record, CHANNEL_ACK_CONTENT_TYPE);
    return receiverCursorDeserialize(record.body);
  }

  async acknowledge(receiverId: Uint8Array, acknowledged: bigint): Promise<bigint> {
    validateAgentId(receiverId, "invalid_receiver_id");
    const state = await this.state();
    if (acknowledged >= state.nextSequence) fail("acknowledgement_ahead");
    if (state.pendingHeader !== undefined && acknowledged >= state.pendingHeader.sequence) {
      fail("acknowledgement_pending");
    }
    if (acknowledged === MAX_U64) fail("sequence_exhausted");
    const desired = acknowledged + 1n;
    const key = receiverAckRecordKey(this.#channelId, receiverId);
    for (let attempt = 0; attempt < MAX_CHANNEL_CAS_ATTEMPTS; attempt += 1) {
      const record = await storage(() => this.#backend.get(CHANNEL_STORAGE_NAMESPACE, key));
      if (record === undefined) {
        try {
          await this.#backend.put(putInput(key, CHANNEL_ACK_CONTENT_TYPE, receiverCursorSerialize(desired), { ifAbsent: true }));
          return desired;
        } catch (error) {
          if (error instanceof StorageConflictError) continue;
          throw storageError(error);
        }
      }
      requireContentType(record, CHANNEL_ACK_CONTENT_TYPE);
      const current = receiverCursorDeserialize(record.body);
      if (desired < current) fail("acknowledgement_regression");
      if (desired === current) return current;
      try {
        await this.#backend.put(putInput(
          key, CHANNEL_ACK_CONTENT_TYPE, receiverCursorSerialize(desired),
          { ifRevision: record.revision },
        ));
        return desired;
      } catch (error) {
        if (error instanceof StorageConflictError) continue;
        throw storageError(error);
      }
    }
    fail("concurrent_update");
  }

  async saveKeyGrant(grant: OpaqueKeyGrant): Promise<void> {
    if (!bytesEqual(grant.channelId, this.#channelId)) fail("corrupt_record");
    validateAgentId(grant.receiverId, "invalid_receiver_id");
    await this.#putIdempotent(
      keyGrantRecordKey(this.#channelId, grant.keyEpoch, grant.receiverId),
      CHANNEL_GRANT_CONTENT_TYPE,
      grant.body,
    );
  }

  async keyGrant(keyEpoch: bigint, receiverId: Uint8Array): Promise<Uint8Array | undefined> {
    validateAgentId(receiverId, "invalid_receiver_id");
    const key = keyGrantRecordKey(this.#channelId, keyEpoch, receiverId);
    const record = await storage(() => this.#backend.get(CHANNEL_STORAGE_NAMESPACE, key));
    if (record === undefined) return undefined;
    requireContentType(record, CHANNEL_GRANT_CONTENT_TYPE);
    return copy(record.body);
  }

  async #stateRecord(): Promise<StorageRecord | undefined> {
    return storage(() => this.#backend.get(
      CHANNEL_STORAGE_NAMESPACE, sequenceStateRecordKey(this.#channelId),
    ));
  }

  async #putIdempotent(key: string, contentType: string, body: Uint8Array): Promise<void> {
    try {
      await this.#backend.put(putInput(key, contentType, body, { ifAbsent: true }));
    } catch (error) {
      if (!(error instanceof StorageConflictError)) throw storageError(error);
      const current = await storage(() => this.#backend.get(CHANNEL_STORAGE_NAMESPACE, key));
      if (current === undefined || current.contentType !== contentType || !bytesEqual(current.body, body)) {
        fail("conflicting_record");
      }
    }
  }

  async #clearPending(expected: MessageHeader): Promise<void> {
    for (let attempt = 0; attempt < MAX_CHANNEL_CAS_ATTEMPTS; attempt += 1) {
      const record = await this.#stateRecord();
      if (record === undefined) fail("not_initialized");
      const current = decodeStateRecord(record, this.#channelId);
      if (current.pendingHeader === undefined) return;
      if (!current.pendingHeader.equals(expected)) fail("pending_header_mismatch");
      try {
        await this.#backend.put(putInput(
          sequenceStateRecordKey(this.#channelId), CHANNEL_STATE_CONTENT_TYPE,
          channelStateSerialize({ nextSequence: current.nextSequence }),
          { ifRevision: record.revision },
        ));
        return;
      } catch (error) {
        if (error instanceof StorageConflictError) continue;
        throw storageError(error);
      }
    }
    fail("concurrent_update");
  }
}

function createMessage(
  header: MessageHeader,
  plaintext: Uint8Array,
  signingSecretKey: Uint8Array,
  channelMasterKey: Uint8Array,
): D18Message {
  if (!bytesEqual(sha256(plaintext), header.plaintextHash)) fail("crypto_error");
  try {
    return messageCreate(messageFields(header), plaintext, signingSecretKey, channelMasterKey);
  } catch { fail("crypto_error"); }
}

function decodeMessage(bytes: Uint8Array): D18Message {
  try { return messageDeserialize(bytes); }
  catch (error) {
    if (error instanceof MessageProfileError) fail("wire_error");
    throw error;
  }
}

function messageFields(header: MessageHeader): MessageFields {
  return {
    messageId: header.messageId,
    timestampNs: header.timestampNs,
    originatorId: header.originatorId,
    channelId: header.channelId,
    sequence: header.sequence,
    keyEpoch: header.keyEpoch,
    contentType: header.contentType,
  };
}

function messageMatchesHeader(message: D18Message, header: MessageHeader): boolean {
  return bytesEqual(message.messageId, header.messageId) &&
    message.timestampNs === header.timestampNs &&
    bytesEqual(message.originatorId, header.originatorId) &&
    bytesEqual(message.channelId, header.channelId) &&
    message.sequence === header.sequence &&
    message.keyEpoch === header.keyEpoch &&
    message.contentType === header.contentType &&
    bytesEqual(message.plaintextHash, header.plaintextHash);
}

function decodeStateRecord(record: StorageRecord, channelId: Uint8Array): ChannelState {
  requireContentType(record, CHANNEL_STATE_CONTENT_TYPE);
  return channelStateDeserialize(record.body, channelId);
}

function requireContentType(record: StorageRecord, expected: string): void {
  if (record.contentType !== expected) fail("corrupt_record");
}

function putInput(
  key: string,
  contentType: string,
  body: Uint8Array,
  condition: { readonly ifAbsent: true } | { readonly ifRevision: string },
): StoragePut {
  return { namespace: CHANNEL_STORAGE_NAMESPACE, key, contentType, metadata: {}, body: copy(body), ...condition };
}

async function storage<T>(operation: () => Promise<T>): Promise<T> {
  try { return await operation(); }
  catch (error) {
    if (error instanceof ChannelProfileError || error instanceof StorageConflictError) throw error;
    throw storageError(error);
  }
}

function storageError(error: unknown): ChannelProfileError {
  if (error instanceof ChannelProfileError) return error;
  return new ChannelProfileError("storage_error");
}

function storageMapKey(namespace: string, key: string): string { return `${namespace}\0${key}`; }
function cloneRecord(record: StorageRecord): StorageRecord {
  return Object.freeze({ ...record, metadata: Object.freeze({}), body: copy(record.body) });
}
