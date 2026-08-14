/** Durable D18P membership plus structurally separate originator/receiver roles. */

import { messageVerify } from "@coding-adventures/chief-of-staff-channel-crypto";
import {
  CHANNEL_DEFINITION_CONTENT_TYPE,
  CHANNEL_STORAGE_NAMESPACE,
  ChannelDefinition,
  ChannelProfileError,
  MAX_DEFINITION_CAS_ATTEMPTS,
  bytesEqual,
  channelDefinitionDeserialize,
  channelDefinitionRecordKey,
  channelDefinitionSerialize,
  copy,
  fail,
  validateAgentId,
  validateUuidV7,
} from "./profile.js";
import {
  type ChannelStorageBackend,
  ChannelStore,
  type StorageRecord,
  StorageConflictError,
} from "./storage.js";

export interface MessageMetadata {
  readonly messageId: Uint8Array;
  readonly timestampNs: bigint;
}

export interface MessageMetadataSource {
  next(): MessageMetadata | Promise<MessageMetadata>;
}

export interface PublishedMessage {
  readonly messageId: Uint8Array;
  readonly sequence: bigint;
  readonly timestampNs: bigint;
}

export interface ReceivedMessage extends PublishedMessage {
  readonly contentType: string;
  readonly payload: Uint8Array;
}

/** #141-owned key custody boundary used after a persisted grant is found. */
export interface ReceiverEpochKeyProvider {
  readonly publicKey: Uint8Array;
  openGrant(keyEpoch: bigint, grantBody: Uint8Array): Uint8Array | undefined;
}

/** Atomic creation, loading, and irreversible retirement of D18C records. */
export class ChannelDefinitionStore {
  readonly #backend: ChannelStorageBackend;

  constructor(backend: ChannelStorageBackend) { this.#backend = backend; }

  async create(definition: ChannelDefinition): Promise<ChannelDefinition> {
    if (definition.lifecycle !== "active") fail("invalid_definition");
    await backendCall(() => this.#backend.initialize());
    const key = channelDefinitionRecordKey(definition.channelId);
    const body = channelDefinitionSerialize(definition);
    let persisted: ChannelDefinition;
    try {
      const record = await this.#backend.put({
        namespace: CHANNEL_STORAGE_NAMESPACE,
        key,
        contentType: CHANNEL_DEFINITION_CONTENT_TYPE,
        metadata: {},
        body,
        ifAbsent: true,
      });
      persisted = requireDefinitionRecord(record, definition.channelId);
    } catch (error) {
      if (!(error instanceof StorageConflictError)) throw backendError(error);
      const record = await backendCall(() => this.#backend.get(CHANNEL_STORAGE_NAMESPACE, key));
      if (record === undefined) fail("definition_not_found");
      if (record.contentType !== CHANNEL_DEFINITION_CONTENT_TYPE) fail("corrupt_definition");
      if (!bytesEqual(record.body, body)) fail("conflicting_definition");
      persisted = requireDefinitionRecord(record, definition.channelId);
    }
    if (!persisted.equals(definition)) fail("conflicting_definition");
    await new ChannelStore(this.#backend, definition.channelId).initialize();
    return this.requireCurrent(definition);
  }

  async load(channelId: Uint8Array): Promise<ChannelDefinition | undefined> {
    await backendCall(() => this.#backend.initialize());
    return (await this.#loadRecord(channelId))?.definition;
  }

  async destroy(channelId: Uint8Array): Promise<ChannelDefinition> {
    await backendCall(() => this.#backend.initialize());
    for (let attempt = 0; attempt < MAX_DEFINITION_CAS_ATTEMPTS; attempt += 1) {
      const loaded = await this.#loadRecord(channelId);
      if (loaded === undefined) fail("definition_not_found");
      if (loaded.definition.lifecycle === "destroyed") return loaded.definition;
      const destroyed = loaded.definition.withLifecycle("destroyed");
      try {
        const record = await this.#backend.put({
          namespace: CHANNEL_STORAGE_NAMESPACE,
          key: channelDefinitionRecordKey(channelId),
          contentType: CHANNEL_DEFINITION_CONTENT_TYPE,
          metadata: {},
          body: channelDefinitionSerialize(destroyed),
          ifRevision: loaded.revision,
        });
        return requireDefinitionRecord(record, channelId);
      } catch (error) {
        if (error instanceof StorageConflictError) continue;
        throw backendError(error);
      }
    }
    fail("concurrent_update");
  }

  async requireCurrent(expected: ChannelDefinition): Promise<ChannelDefinition> {
    const actual = await this.load(expected.channelId);
    if (actual === undefined) fail("definition_not_found");
    if (actual.lifecycle === "destroyed") fail("channel_destroyed");
    if (!actual.equals(expected)) fail("definition_changed");
    return actual;
  }

  async #loadRecord(channelId: Uint8Array): Promise<LoadedDefinition | undefined> {
    const key = channelDefinitionRecordKey(channelId);
    const record = await backendCall(() => this.#backend.get(CHANNEL_STORAGE_NAMESPACE, key));
    if (record === undefined) return undefined;
    return { definition: requireDefinitionRecord(record, channelId), revision: record.revision };
  }
}

interface LoadedDefinition { readonly definition: ChannelDefinition; readonly revision: string; }

/** The only role with a publish operation. */
export class DurableOriginator {
  readonly #backend: ChannelStorageBackend;
  readonly #definition: ChannelDefinition;
  readonly #signingSecretKey: Uint8Array;
  readonly #channelMasterKey: Uint8Array;
  readonly #metadataSource: MessageMetadataSource;

  private constructor(
    backend: ChannelStorageBackend,
    definition: ChannelDefinition,
    signingSecretKey: Uint8Array,
    channelMasterKey: Uint8Array,
    metadataSource: MessageMetadataSource,
  ) {
    this.#backend = backend;
    this.#definition = definition;
    this.#signingSecretKey = copy(signingSecretKey);
    this.#channelMasterKey = copy(channelMasterKey);
    this.#metadataSource = metadataSource;
  }

  static async open(
    backend: ChannelStorageBackend,
    channelId: Uint8Array,
    agentId: Uint8Array,
    signingSecretKey: Uint8Array,
    channelMasterKey: Uint8Array,
    metadataSource: MessageMetadataSource,
  ): Promise<DurableOriginator> {
    const definition = await activeDefinition(backend, channelId);
    if (!bytesEqual(definition.originator.agentId, agentId)) fail("unauthorized_originator");
    if (
      signingSecretKey.length !== 64 ||
      !bytesEqual(definition.originator.publicKey, signingSecretKey.slice(32))
    ) fail("public_key_mismatch");
    if (channelMasterKey.length !== 32) fail("crypto_error");
    await new ChannelStore(backend, channelId).initialize();
    return new DurableOriginator(
      backend, definition, signingSecretKey, channelMasterKey, metadataSource,
    );
  }

  get id(): Uint8Array { return this.#definition.originator.agentId; }
  get channelId(): Uint8Array { return this.#definition.channelId; }
  get publicKey(): Uint8Array { return this.#definition.originator.publicKey; }

  async publish(payload: Uint8Array, contentType: string): Promise<PublishedMessage> {
    let metadata: MessageMetadata;
    try { metadata = await this.#metadataSource.next(); }
    catch { fail("metadata_error"); }
    return this.publishWithMetadata(metadata, payload, contentType);
  }

  async publishWithMetadata(
    metadata: MessageMetadata,
    payload: Uint8Array,
    contentType: string,
  ): Promise<PublishedMessage> {
    validateUuidV7(metadata.messageId);
    await new ChannelDefinitionStore(this.#backend).requireCurrent(this.#definition);
    const message = await new ChannelStore(this.#backend, this.#definition.channelId).append(
      {
        messageId: metadata.messageId,
        timestampNs: metadata.timestampNs,
        originatorId: this.#definition.originator.agentId,
        keyEpoch: this.#definition.keyEpoch,
        contentType,
      },
      payload,
      this.#channelMasterKey,
      this.#signingSecretKey,
    );
    return Object.freeze({
      messageId: copy(metadata.messageId),
      sequence: message.sequence,
      timestampNs: metadata.timestampNs,
    });
  }

  /** Persist a #141-produced opaque grant after enforcing current membership. */
  async saveReceiverGrant(receiverId: Uint8Array, grantBody: Uint8Array): Promise<void> {
    const definition = await new ChannelDefinitionStore(this.#backend).requireCurrent(this.#definition);
    if (definition.receiver(receiverId) === undefined) fail("unauthorized_receiver");
    await new ChannelStore(this.#backend, definition.channelId).saveKeyGrant({
      channelId: definition.channelId,
      keyEpoch: definition.keyEpoch,
      receiverId,
      body: grantBody,
    });
  }
}

/** Receiver role: ordered verified delivery and session-bound acknowledgement only. */
export class DurableReceiver {
  readonly #backend: ChannelStorageBackend;
  readonly #definition: ChannelDefinition;
  readonly #receiverId: Uint8Array;
  readonly #keyProvider: ReceiverEpochKeyProvider;
  readonly #delivered = new Map<string, bigint>();

  private constructor(
    backend: ChannelStorageBackend,
    definition: ChannelDefinition,
    receiverId: Uint8Array,
    keyProvider: ReceiverEpochKeyProvider,
  ) {
    this.#backend = backend;
    this.#definition = definition;
    this.#receiverId = copy(receiverId);
    this.#keyProvider = keyProvider;
  }

  static async open(
    backend: ChannelStorageBackend,
    channelId: Uint8Array,
    receiverId: Uint8Array,
    keyProvider: ReceiverEpochKeyProvider,
  ): Promise<DurableReceiver> {
    validateAgentId(receiverId, "invalid_receiver_id");
    const definition = await activeDefinition(backend, channelId);
    const receiver = definition.receiver(receiverId);
    if (receiver === undefined) fail("unauthorized_receiver");
    if (!bytesEqual(receiver.publicKey, keyProvider.publicKey)) fail("public_key_mismatch");
    await new ChannelStore(backend, channelId).initialize();
    return new DurableReceiver(backend, definition, receiverId, keyProvider);
  }

  get id(): Uint8Array { return copy(this.#receiverId); }
  get channelId(): Uint8Array { return this.#definition.channelId; }
  get publicKey(): Uint8Array { return copy(this.#keyProvider.publicKey); }

  async receive(limit: number): Promise<readonly ReceivedMessage[]> {
    await new ChannelDefinitionStore(this.#backend).requireCurrent(this.#definition);
    const store = new ChannelStore(this.#backend, this.#definition.channelId);
    const page = await store.readForReceiver(this.#receiverId, limit);
    const delivered: ReceivedMessage[] = [];
    for (const message of page.messages) {
      if (
        !bytesEqual(message.channelId, this.#definition.channelId) ||
        !bytesEqual(message.originatorId, this.#definition.originator.agentId) ||
        message.keyEpoch > this.#definition.keyEpoch
      ) fail("unauthorized_message");
      const grant = await store.keyGrant(message.keyEpoch, this.#receiverId);
      if (grant === undefined) fail("missing_key_grant");
      let channelKey: Uint8Array | undefined;
      try { channelKey = this.#keyProvider.openGrant(message.keyEpoch, grant); }
      catch { fail("crypto_error"); }
      if (channelKey === undefined) fail("missing_key_grant");
      let payload: Uint8Array;
      try { payload = messageVerify(message, this.#definition.originator.publicKey, channelKey); }
      catch { fail("crypto_error"); }
      validateUuidV7(message.messageId);
      const identity = bytesHex(message.messageId);
      const previous = this.#delivered.get(identity);
      if (previous !== undefined && previous !== message.sequence) fail("unauthorized_message");
      this.#delivered.set(identity, message.sequence);
      delivered.push(Object.freeze({
        messageId: message.messageId,
        sequence: message.sequence,
        timestampNs: message.timestampNs,
        contentType: message.contentType,
        payload: copy(payload),
      }));
    }
    return Object.freeze(delivered);
  }

  async acknowledge(messageId: Uint8Array): Promise<bigint> {
    validateUuidV7(messageId);
    await new ChannelDefinitionStore(this.#backend).requireCurrent(this.#definition);
    const sequence = this.#delivered.get(bytesHex(messageId));
    if (sequence === undefined) fail("unknown_message_id");
    return new ChannelStore(this.#backend, this.#definition.channelId)
      .acknowledge(this.#receiverId, sequence);
  }
}

async function activeDefinition(
  backend: ChannelStorageBackend,
  channelId: Uint8Array,
): Promise<ChannelDefinition> {
  const definition = await new ChannelDefinitionStore(backend).load(channelId);
  if (definition === undefined) fail("definition_not_found");
  if (definition.lifecycle === "destroyed") fail("channel_destroyed");
  return definition;
}

function requireDefinitionRecord(record: StorageRecord, channelId: Uint8Array): ChannelDefinition {
  if (record.contentType !== CHANNEL_DEFINITION_CONTENT_TYPE) fail("corrupt_definition");
  const definition = channelDefinitionDeserialize(record.body);
  if (
    !bytesEqual(definition.channelId, channelId) ||
    record.key !== channelDefinitionRecordKey(channelId)
  ) fail("corrupt_definition");
  return definition;
}

async function backendCall<T>(operation: () => Promise<T>): Promise<T> {
  try { return await operation(); }
  catch (error) {
    if (error instanceof ChannelProfileError || error instanceof StorageConflictError) throw error;
    throw backendError(error);
  }
}

function backendError(error: unknown): ChannelProfileError {
  if (error instanceof ChannelProfileError) return error;
  return new ChannelProfileError("storage_error");
}

function bytesHex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
