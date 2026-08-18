/** Portable D18T durable epoch-activation orchestration. */

import {
  ChannelMasterKey,
  type D18Message,
  OriginatorSigningKey,
  type PortableKeyGrant,
  type RotationPlan,
  grantDeserialize,
  grantSerialize,
  messageCreateWithSigner,
  messageDeserialize,
  messageSerialize,
  secretErasureCapability,
  verifyGrantSignature,
} from "@coding-adventures/chief-of-staff-channel-crypto";
import {
  CHANNEL_GRANT_CONTENT_TYPE,
  CHANNEL_MESSAGE_CONTENT_TYPE,
  CHANNEL_STATE_CONTENT_TYPE,
  CHANNEL_STORAGE_NAMESPACE,
  ChannelDefinition,
  ChannelDefinitionStore,
  ChannelProfileError,
  type ChannelState,
  type ChannelStorageBackend,
  MAX_U64,
  MessageHeader,
  type ReceiverIdentity,
  StorageConflictError,
  type StorageRecord,
  bytesEqual,
  channelStateDeserialize,
  keyGrantRecordKey,
  messageRecordKey,
  sequenceStateRecordKey,
} from "@coding-adventures/chief-of-staff-channel-store";
import { sha256 } from "@coding-adventures/sha256";
import {
  type CustodySelection,
  EpochKeyHandle,
  InMemoryKeyCustody,
  type OriginatorKeyCustody,
  PreparedEpoch,
  PublicPreparation,
} from "./custody.js";
import {
  ACTIVATION_PLAN_CONTENT_TYPE,
  ActivationPlan,
  ActivationPlanEntry,
  EPOCH_STATE_CONTENT_TYPE,
  EpochState,
  MAX_PLAN_RECEIVERS,
  activationPlanDeserialize,
  activationPlanRecordKey,
  activationPlanSerialize,
  epochStateDeserialize,
  epochStateSerialize,
} from "./wire.js";

export * from "./custody.js";
export * from "./wire.js";

export const MAX_EPOCH_CAS_ATTEMPTS = 16;
export const EPOCH_ACTIVATION_ERROR_CODES = [
  "not_initialized", "channel_destroyed", "invalid_plan", "corrupt_record",
  "pending_append", "unactivated_epoch", "active_key_missing",
  "conflicting_active_key", "preparation_missing", "conflicting_preparation",
  "conflicting_plan", "conflicting_grant", "unexpected_epoch", "decreasing_epoch",
  "epoch_exhausted", "concurrent_update", "storage_error", "custody_error", "crypto_error",
] as const;
export type EpochActivationErrorCode = (typeof EPOCH_ACTIVATION_ERROR_CODES)[number];

/** Stable D18T failure whose message never contains secret data. */
export class EpochActivationError extends Error {
  readonly code: EpochActivationErrorCode;
  constructor(code: EpochActivationErrorCode) {
    super(code); this.name = "EpochActivationError"; this.code = code;
  }
}

export type PreparationOutcome = "prepared" | "idempotent";
export type ActivationOutcome = "activated" | "idempotent";

/** Honest D18Q/TypeScript secret-erasure capability for this composition. */
export function epochActivationSecretErasureCapability() {
  return secretErasureCapability();
}

export interface ActiveEpochAppendRequest {
  readonly messageId: Uint8Array;
  readonly timestampNs: bigint;
  readonly originatorId: Uint8Array;
  readonly contentType: string;
  /** If supplied, it must equal the authoritative D18S v2 epoch. */
  readonly keyEpoch?: bigint;
}

/** Exact reserved D18H plus the redacted handle for its selected active CMK. */
export class EpochReservation {
  readonly header: MessageHeader;
  readonly keyHandle: EpochKeyHandle;
  constructor(header: MessageHeader, keyHandle: EpochKeyHandle) {
    this.header = header; this.keyHandle = keyHandle; Object.freeze(this);
  }
}

/** D18T coordinator over injected public storage and secret custody. */
export class EpochActivationStore {
  readonly #backend: ChannelStorageBackend;
  readonly #custody: OriginatorKeyCustody;
  readonly #channelId: Uint8Array;

  private constructor(
    backend: ChannelStorageBackend,
    custody: OriginatorKeyCustody,
    channelId: Uint8Array,
  ) {
    this.#backend = backend; this.#custody = custody; this.#channelId = channelId.slice();
  }

  static async open(
    backend: ChannelStorageBackend,
    custody: OriginatorKeyCustody,
    channelId: Uint8Array,
  ): Promise<EpochActivationStore> {
    if (!custody.durable) fail("custody_error");
    await backendCall(() => backend.initialize());
    return new EpochActivationStore(backend, custody, channelId);
  }

  static async openForTesting(
    backend: ChannelStorageBackend,
    custody: OriginatorKeyCustody,
    channelId: Uint8Array,
  ): Promise<EpochActivationStore> {
    await backendCall(() => backend.initialize());
    return new EpochActivationStore(backend, custody, channelId);
  }

  /** Custody-first creation of a D18T-aware channel and its initial v2 state. */
  async createEpochChannel(
    definition: ChannelDefinition,
    initialCmk: ChannelMasterKey,
  ): Promise<EpochState> {
    if (!bytesEqual(definition.channelId, this.#channelId) || definition.lifecycle !== "active") {
      initialCmk.destroy();
      fail("invalid_plan");
    }
    await this.#importInitialKey(definition.keyEpoch, initialCmk);
    const definitions = new ChannelDefinitionStore(this.#backend);
    try {
      const existing = await definitions.load(this.#channelId);
      if (existing === undefined) await definitions.create(definition);
      else if (existing.lifecycle === "destroyed") fail("channel_destroyed");
      else if (!existing.equals(definition)) fail("invalid_plan");
    }
    catch (error) {
      if (error instanceof EpochActivationError) throw error;
      if (error instanceof ChannelProfileError) {
        if (error.code === "channel_destroyed") fail("channel_destroyed");
        if (error.code === "conflicting_definition") fail("invalid_plan");
        fail("corrupt_record");
      }
      throw storageError(error);
    }
    return this.migrateEpochState(definition);
  }

  /** Upgrade absent or D18S v1 state only after the current CMK is resolvable. */
  async migrateEpochState(
    definition: ChannelDefinition,
    currentCmk?: ChannelMasterKey,
  ): Promise<EpochState> {
    await this.#requireDefinition(definition, false);
    for (let attempt = 0; attempt < MAX_EPOCH_CAS_ATTEMPTS; attempt += 1) {
      const record = await this.#stateRecord();
      if (record?.contentType === EPOCH_STATE_CONTENT_TYPE) {
        const state = this.#decodeV2StateRecord(record);
        if (await custodyCall(() => this.#custody.resolveHandle(this.#channelId, state.activeEpoch)) === undefined) {
          fail("active_key_missing");
        }
        return state;
      }

      await this.#ensureInitialKey(definition.keyEpoch, currentCmk);
      let state: EpochState;
      if (record === undefined) {
        state = new EpochState(this.#channelId, {
          activeEpoch: definition.keyEpoch, nextSequence: 0n,
        });
      } else {
        this.#requireEnvelope(record, sequenceStateRecordKey(this.#channelId), CHANNEL_STATE_CONTENT_TYPE);
        let prior: ChannelState;
        try { prior = channelStateDeserialize(record.body, this.#channelId); }
        catch { fail("corrupt_record"); }
        if (prior.pendingHeader !== undefined && prior.pendingHeader.keyEpoch !== definition.keyEpoch) {
          fail("corrupt_record");
        }
        state = new EpochState(this.#channelId, {
          activeEpoch: definition.keyEpoch,
          nextSequence: prior.nextSequence,
          pendingHeader: prior.pendingHeader,
        });
      }
      try {
        const stored = await this.#backend.put(publicPut(
          sequenceStateRecordKey(this.#channelId), EPOCH_STATE_CONTENT_TYPE,
          epochStateSerialize(state), record === undefined ? { ifAbsent: true } : { ifRevision: record.revision },
        ));
        return this.#decodeV2StateRecord(stored);
      } catch (error) {
        if (error instanceof StorageConflictError) continue;
        throw storageError(error);
      }
    }
    fail("concurrent_update");
  }

  async state(): Promise<EpochState> {
    const record = await this.#stateRecord();
    if (record === undefined) fail("not_initialized");
    return this.#decodeV2StateRecord(record);
  }

  async prepareRotation(
    definition: ChannelDefinition,
    targetRoster: readonly ReceiverIdentity[],
    rotation: RotationPlan,
  ): Promise<PreparationOutcome> {
    await this.#requireDefinition(definition, false);
    const state = await this.state();
    if (state.pendingHeader !== undefined) fail("pending_append");
    if (state.activeEpoch === MAX_U64) fail("epoch_exhausted");
    const expected = state.activeEpoch + 1n;
    if (rotation.newEpoch !== expected) fail("unexpected_epoch");
    const prepared = prepareRotationCandidate(definition, state.activeEpoch, targetRoster, rotation);
    let selection: CustodySelection;
    try { selection = await custodyCall(() => this.#custody.prepareIfAbsent(prepared)); }
    finally { prepared.destroy(); }
    if (selection === "conflict") fail("conflicting_preparation");
    await this.#replayPreparation(definition, expected);
    return selection === "selected" ? "prepared" : "idempotent";
  }

  async recoverPreparation(
    definition: ChannelDefinition,
    newEpoch: bigint,
  ): Promise<PreparationOutcome> {
    await this.#requireDefinition(definition, false);
    const active = (await this.state()).activeEpoch;
    if (newEpoch < active) fail("decreasing_epoch");
    if (newEpoch !== active) {
      if (active === MAX_U64) fail("epoch_exhausted");
      if (newEpoch !== active + 1n) fail("unexpected_epoch");
    }
    await this.#replayPreparation(definition, newEpoch);
    return "idempotent";
  }

  async activatePreparedEpoch(
    definition: ChannelDefinition,
    newEpoch: bigint,
  ): Promise<ActivationOutcome> {
    await this.#requireDefinition(definition, false);
    const prepared = await custodyCall(() => this.#custody.loadPreparation(this.#channelId, newEpoch));
    if (prepared === undefined) fail("preparation_missing");
    for (let attempt = 0; attempt < MAX_EPOCH_CAS_ATTEMPTS; attempt += 1) {
      await this.#requireDefinition(definition, false);
      const record = await this.#stateRecord();
      if (record === undefined) fail("not_initialized");
      const state = this.#decodeV2StateRecord(record);
      if (state.activeEpoch === newEpoch) {
        await this.#validateAndReplay(definition, prepared);
        await this.#requireHandle(newEpoch);
        return "idempotent";
      }
      if (state.activeEpoch > newEpoch) fail("decreasing_epoch");
      if (state.activeEpoch === MAX_U64) fail("epoch_exhausted");
      if (
        state.activeEpoch + 1n !== newEpoch || prepared.baseEpoch !== state.activeEpoch ||
        prepared.newEpoch !== newEpoch
      ) fail("unexpected_epoch");
      await this.#validateAndReplay(definition, prepared);
      await this.#requireHandle(newEpoch);
      if (state.pendingHeader !== undefined) fail("pending_append");
      const updated = state.withActiveEpoch(this.#channelId, newEpoch);
      try {
        const stored = await this.#backend.put(publicPut(
          sequenceStateRecordKey(this.#channelId), EPOCH_STATE_CONTENT_TYPE,
          epochStateSerialize(updated), { ifRevision: record.revision },
        ));
        if (!this.#decodeV2StateRecord(stored).equals(updated)) fail("corrupt_record");
        return "activated";
      } catch (error) {
        if (error instanceof StorageConflictError) continue;
        if (error instanceof EpochActivationError) throw error;
        throw storageError(error);
      }
    }
    fail("concurrent_update");
  }

  /** Reserve a publish using only the epoch authoritative in D18S v2. */
  async reservePublishUsingActiveEpoch(
    definition: ChannelDefinition,
    request: ActiveEpochAppendRequest,
    plaintext: Uint8Array,
  ): Promise<EpochReservation> {
    await this.#requireDefinition(definition, false);
    if (!bytesEqual(request.originatorId, definition.originator.agentId)) fail("invalid_plan");
    for (let attempt = 0; attempt < MAX_EPOCH_CAS_ATTEMPTS; attempt += 1) {
      const record = await this.#stateRecord();
      if (record === undefined) fail("not_initialized");
      const state = this.#decodeV2StateRecord(record);
      if (request.keyEpoch !== undefined && request.keyEpoch !== state.activeEpoch) fail("unactivated_epoch");
      const handle = await this.#requireHandle(state.activeEpoch);
      if (state.pendingHeader !== undefined) fail("pending_append");
      if (state.nextSequence === MAX_U64) fail("crypto_error");
      let header: MessageHeader;
      try {
        header = new MessageHeader({
          messageId: request.messageId,
          timestampNs: request.timestampNs,
          originatorId: request.originatorId,
          channelId: this.#channelId,
          sequence: state.nextSequence,
          keyEpoch: state.activeEpoch,
          contentType: request.contentType,
          plaintextHash: sha256(plaintext),
        });
      } catch { fail("crypto_error"); }
      const updated = state.withPending(this.#channelId, state.nextSequence + 1n, header);
      try {
        await this.#backend.put(publicPut(
          sequenceStateRecordKey(this.#channelId), EPOCH_STATE_CONTENT_TYPE,
          epochStateSerialize(updated), { ifRevision: record.revision },
        ));
        return new EpochReservation(header, handle);
      } catch (error) {
        if (error instanceof StorageConflictError) continue;
        throw storageError(error);
      }
    }
    fail("concurrent_update");
  }

  /** Encrypt, idempotently persist, and clear one exact reservation. */
  async commitReserved(
    definition: ChannelDefinition,
    reservation: EpochReservation,
    plaintext: Uint8Array,
    signingKey: OriginatorSigningKey,
  ): Promise<D18Message> {
    await this.#requireDefinition(definition, false);
    const header = reservation.header;
    if (
      !bytesEqual(header.channelId, this.#channelId) ||
      header.keyEpoch !== reservation.keyHandle.epoch ||
      !bytesEqual(reservation.keyHandle.channelId, this.#channelId) ||
      !bytesEqual(signingKey.publicKey, definition.originator.publicKey) ||
      !bytesEqual(sha256(plaintext), header.plaintextHash)
    ) fail("invalid_plan");
    const state = await this.state();
    if (state.pendingHeader === undefined) {
      const key = messageRecordKey(this.#channelId, header.sequence);
      const stored = await backendCall(() => this.#backend.get(CHANNEL_STORAGE_NAMESPACE, key));
      if (stored === undefined) fail("corrupt_record");
      this.#requireEnvelope(stored, key, CHANNEL_MESSAGE_CONTENT_TYPE);
      let message: D18Message;
      try { message = messageDeserialize(stored.body); } catch { fail("corrupt_record"); }
      if (!messageMatchesHeader(message, header)) fail("corrupt_record");
      const expected = await this.#encryptWithHandle(reservation.keyHandle, header, plaintext, signingKey);
      if (!bytesEqual(messageSerialize(expected), stored.body)) fail("corrupt_record");
      return message;
    }
    if (!state.pendingHeader.equals(header)) fail("invalid_plan");
    const message = await this.#encryptWithHandle(reservation.keyHandle, header, plaintext, signingKey);
    await this.#putImmutable(
      messageRecordKey(this.#channelId, header.sequence), CHANNEL_MESSAGE_CONTENT_TYPE,
      messageSerialize(message), "corrupt_record",
    );
    await this.#clearPending(header);
    return message;
  }

  async abandonPending(): Promise<MessageHeader | undefined> {
    for (let attempt = 0; attempt < MAX_EPOCH_CAS_ATTEMPTS; attempt += 1) {
      const record = await this.#stateRecord();
      if (record === undefined) fail("not_initialized");
      const state = this.#decodeV2StateRecord(record);
      if (state.pendingHeader === undefined) return undefined;
      const updated = state.withPending(this.#channelId, state.nextSequence);
      try {
        await this.#backend.put(publicPut(
          sequenceStateRecordKey(this.#channelId), EPOCH_STATE_CONTENT_TYPE,
          epochStateSerialize(updated), { ifRevision: record.revision },
        ));
        return state.pendingHeader;
      } catch (error) {
        if (error instanceof StorageConflictError) continue;
        throw storageError(error);
      }
    }
    fail("concurrent_update");
  }

  async activationPlan(newEpoch: bigint): Promise<ActivationPlan | undefined> {
    const key = activationPlanRecordKey(this.#channelId, newEpoch);
    const record = await backendCall(() => this.#backend.get(CHANNEL_STORAGE_NAMESPACE, key));
    if (record === undefined) return undefined;
    this.#requireEnvelope(record, key, ACTIVATION_PLAN_CONTENT_TYPE);
    let plan: ActivationPlan;
    try { plan = activationPlanDeserialize(record.body); } catch { fail("corrupt_record"); }
    if (!bytesEqual(plan.channelId, this.#channelId) || plan.newEpoch !== newEpoch) fail("corrupt_record");
    return plan;
  }

  async applyDestruction(definition: ChannelDefinition): Promise<void> {
    await this.#requireDefinition(definition, true);
    await custodyCall(() => this.#custody.destroyChannel(this.#channelId));
  }

  async #ensureInitialKey(epoch: bigint, currentCmk?: ChannelMasterKey): Promise<void> {
    if (await custodyCall(() => this.#custody.resolveHandle(this.#channelId, epoch)) !== undefined) return;
    if (currentCmk === undefined) fail("active_key_missing");
    await this.#importInitialKey(epoch, currentCmk);
  }

  async #importInitialKey(epoch: bigint, currentCmk: ChannelMasterKey): Promise<void> {
    let selection: CustodySelection;
    try { selection = await custodyCall(() => this.#custody.importActiveIfAbsent(this.#channelId, epoch, currentCmk)); }
    finally { currentCmk.destroy(); }
    if (selection === "conflict") fail("conflicting_active_key");
  }

  async #replayPreparation(definition: ChannelDefinition, newEpoch: bigint): Promise<void> {
    const prepared = await custodyCall(() => this.#custody.loadPreparation(this.#channelId, newEpoch));
    if (prepared === undefined) fail("preparation_missing");
    await this.#validateAndReplay(definition, prepared);
  }

  async #validateAndReplay(definition: ChannelDefinition, prepared: PublicPreparation): Promise<void> {
    const plan = validatePublicPreparation(definition, prepared);
    await this.#putImmutable(
      activationPlanRecordKey(this.#channelId, plan.newEpoch), ACTIVATION_PLAN_CONTENT_TYPE,
      prepared.planBytes, "conflicting_plan",
    );
    for (const bytes of prepared.grants) {
      let grant: PortableKeyGrant;
      try { grant = grantDeserialize(bytes); } catch { fail("crypto_error"); }
      await this.#putImmutable(
        keyGrantRecordKey(this.#channelId, grant.keyEpoch, grant.receiverId),
        CHANNEL_GRANT_CONTENT_TYPE, bytes, "conflicting_grant",
      );
    }
    const stored = await this.activationPlan(plan.newEpoch);
    if (stored === undefined || !stored.equals(plan)) fail("corrupt_record");
    for (const bytes of prepared.grants) {
      const grant = cryptoCall(() => grantDeserialize(bytes));
      const key = keyGrantRecordKey(this.#channelId, grant.keyEpoch, grant.receiverId);
      const record = await backendCall(() => this.#backend.get(CHANNEL_STORAGE_NAMESPACE, key));
      if (record === undefined) fail("corrupt_record");
      this.#requireEnvelope(record, key, CHANNEL_GRANT_CONTENT_TYPE);
      if (!bytesEqual(record.body, bytes)) fail("corrupt_record");
    }
  }

  async #encryptWithHandle(
    handle: EpochKeyHandle,
    header: MessageHeader,
    plaintext: Uint8Array,
    signingKey: OriginatorSigningKey,
  ): Promise<D18Message> {
    return custodyCall(() => this.#custody.withKey(handle, (cmk) => {
      const bytes = cmk.bytes;
      try {
        return cryptoCall(() => messageCreateWithSigner(
          {
            messageId: header.messageId, timestampNs: header.timestampNs,
            originatorId: header.originatorId, channelId: header.channelId,
            sequence: header.sequence, keyEpoch: header.keyEpoch, contentType: header.contentType,
          },
          plaintext,
          (message) => signingKey.sign(message),
          bytes,
        ));
      } finally { bytes.fill(0); }
    }));
  }

  async #requireHandle(epoch: bigint): Promise<EpochKeyHandle> {
    const handle = await custodyCall(() => this.#custody.resolveHandle(this.#channelId, epoch));
    if (handle === undefined) fail("active_key_missing");
    return handle;
  }

  async #clearPending(expected: MessageHeader): Promise<void> {
    for (let attempt = 0; attempt < MAX_EPOCH_CAS_ATTEMPTS; attempt += 1) {
      const record = await this.#stateRecord();
      if (record === undefined) fail("not_initialized");
      const state = this.#decodeV2StateRecord(record);
      if (state.pendingHeader === undefined) return;
      if (!state.pendingHeader.equals(expected)) fail("invalid_plan");
      const updated = state.withPending(this.#channelId, state.nextSequence);
      try {
        await this.#backend.put(publicPut(
          sequenceStateRecordKey(this.#channelId), EPOCH_STATE_CONTENT_TYPE,
          epochStateSerialize(updated), { ifRevision: record.revision },
        ));
        return;
      } catch (error) {
        if (error instanceof StorageConflictError) continue;
        throw storageError(error);
      }
    }
    fail("concurrent_update");
  }

  async #requireDefinition(expected: ChannelDefinition, requireDestroyed: boolean): Promise<void> {
    if (!bytesEqual(expected.channelId, this.#channelId)) fail("invalid_plan");
    let actual: ChannelDefinition | undefined;
    try { actual = await new ChannelDefinitionStore(this.#backend).load(this.#channelId); }
    catch (error) {
      if (error instanceof ChannelProfileError && error.code === "channel_destroyed") fail("channel_destroyed");
      if (error instanceof ChannelProfileError) fail("corrupt_record");
      throw storageError(error);
    }
    if (actual === undefined) fail("not_initialized");
    if (!actual.equals(expected)) fail("invalid_plan");
    if (!requireDestroyed && actual.lifecycle === "destroyed") fail("channel_destroyed");
    if (requireDestroyed && actual.lifecycle !== "destroyed") fail("invalid_plan");
  }

  async #stateRecord(): Promise<StorageRecord | undefined> {
    return backendCall(() => this.#backend.get(
      CHANNEL_STORAGE_NAMESPACE, sequenceStateRecordKey(this.#channelId),
    ));
  }

  #decodeV2StateRecord(record: StorageRecord): EpochState {
    this.#requireEnvelope(record, sequenceStateRecordKey(this.#channelId), EPOCH_STATE_CONTENT_TYPE);
    try { return epochStateDeserialize(record.body, this.#channelId); }
    catch { fail("corrupt_record"); }
  }

  #requireEnvelope(record: StorageRecord, key: string, contentType: string): void {
    if (
      record.namespace !== CHANNEL_STORAGE_NAMESPACE || record.key !== key ||
      record.contentType !== contentType || Object.keys(record.metadata).length !== 0
    ) fail("corrupt_record");
  }

  async #putImmutable(
    key: string,
    contentType: string,
    body: Uint8Array,
    conflictCode: EpochActivationErrorCode,
  ): Promise<void> {
    try {
      const record = await this.#backend.put(publicPut(key, contentType, body, { ifAbsent: true }));
      this.#requireEnvelope(record, key, contentType);
      if (!bytesEqual(record.body, body)) fail("corrupt_record");
    } catch (error) {
      if (!(error instanceof StorageConflictError)) {
        if (error instanceof EpochActivationError) throw error;
        throw storageError(error);
      }
      const record = await backendCall(() => this.#backend.get(CHANNEL_STORAGE_NAMESPACE, key));
      if (record === undefined) fail("corrupt_record");
      this.#requireEnvelope(record, key, contentType);
      if (!bytesEqual(record.body, body)) fail(conflictCode);
    }
  }
}

/** Pure candidate construction used by fixture and crash-boundary tests. */
export function prepareRotationCandidate(
  definition: ChannelDefinition,
  baseEpoch: bigint,
  targetRoster: readonly ReceiverIdentity[],
  rotation: RotationPlan,
): PreparedEpoch {
  try {
    if (
      targetRoster.length < 1 || targetRoster.length > MAX_PLAN_RECEIVERS ||
      targetRoster.length !== rotation.grants.length
    ) fail("invalid_plan");
    const roster = targetRoster.map((receiver) => ({
      agentId: receiver.agentId.slice(), publicKey: receiver.publicKey.slice(),
    })).sort((left, right) => compareBytes(left.agentId, right.agentId));
    for (let index = 1; index < roster.length; index += 1) {
      if (bytesEqual(roster[index - 1]!.agentId, roster[index]!.agentId)) fail("invalid_plan");
    }
    for (let index = 0; index < roster.length; index += 1) {
      const receiver = roster[index]!;
      const grant = rotation.grants[index]!;
      if (!bytesEqual(receiver.agentId, grant.receiverId) || grant.keyEpoch !== rotation.newEpoch) {
        fail("invalid_plan");
      }
      cryptoCall(() => verifyGrantSignature(
        grant, definition.originator.agentId, receiver.agentId,
        definition.channelId, definition.originator.publicKey,
      ));
    }
    if (baseEpoch === MAX_U64) fail("epoch_exhausted");
    if (rotation.newEpoch !== baseEpoch + 1n) fail("unexpected_epoch");
    const grantBytes = rotation.grants.map((grant) => cryptoCall(() => grantSerialize(grant)));
    const entries = rotation.grants.map((grant, index) => new ActivationPlanEntry(
      sha256(grant.receiverId), sha256(grantBytes[index]!),
    ));
    const plan = new ActivationPlan(definition.channelId, baseEpoch, rotation.newEpoch, entries);
    const publicPreparation = new PublicPreparation(
      definition.channelId, baseEpoch, rotation.newEpoch,
      activationPlanSerialize(plan), grantBytes,
    );
    const cmk = rotation.newCmk;
    try { return new PreparedEpoch(publicPreparation, cmk); }
    finally { cmk.destroy(); }
  } finally {
    rotation.destroy();
  }
}

function validatePublicPreparation(
  definition: ChannelDefinition,
  prepared: PublicPreparation,
): ActivationPlan {
  if (!bytesEqual(prepared.channelId, definition.channelId)) fail("invalid_plan");
  // Exhaustion sits between the channel comparison and the successor
  // comparison, exactly where Rust's short-circuiting `||` chain evaluates it.
  // It must precede the successor check because baseEpoch + 1 is not a
  // meaningful question once baseEpoch is saturated; it must follow the channel
  // check so a bundle that is BOTH foreign and saturated still reports
  // invalid_plan, as it did before and as Rust still does.
  if (prepared.baseEpoch === MAX_U64) fail("epoch_exhausted");
  if (
    prepared.newEpoch !== prepared.baseEpoch + 1n || prepared.grants.length < 1 ||
    prepared.grants.length > MAX_PLAN_RECEIVERS
  ) fail("invalid_plan");
  let plan: ActivationPlan;
  try { plan = activationPlanDeserialize(prepared.planBytes); } catch { fail("corrupt_record"); }
  if (
    !bytesEqual(plan.channelId, prepared.channelId) || plan.baseEpoch !== prepared.baseEpoch ||
    plan.newEpoch !== prepared.newEpoch || plan.receivers.length !== prepared.grants.length
  ) fail("invalid_plan");
  let priorReceiver: Uint8Array | undefined;
  const entries: ActivationPlanEntry[] = [];
  for (const bytes of prepared.grants) {
    const grant = cryptoCall(() => grantDeserialize(bytes));
    if (
      !bytesEqual(grant.channelId, prepared.channelId) || grant.keyEpoch !== prepared.newEpoch ||
      (priorReceiver !== undefined && compareBytes(priorReceiver, grant.receiverId) >= 0)
    ) fail("invalid_plan");
    cryptoCall(() => verifyGrantSignature(
      grant, definition.originator.agentId, grant.receiverId,
      definition.channelId, definition.originator.publicKey,
    ));
    priorReceiver = grant.receiverId;
    entries.push(new ActivationPlanEntry(sha256(grant.receiverId), sha256(bytes)));
  }
  const expected = new ActivationPlan(
    prepared.channelId, prepared.baseEpoch, prepared.newEpoch, entries,
  );
  if (!plan.equals(expected)) fail("invalid_plan");
  return plan;
}

function publicPut(
  key: string,
  contentType: string,
  body: Uint8Array,
  condition: { readonly ifAbsent: true } | { readonly ifRevision: string },
) {
  return {
    namespace: CHANNEL_STORAGE_NAMESPACE, key, contentType,
    metadata: Object.freeze({}), body: body.slice(), ...condition,
  };
}

function messageMatchesHeader(message: D18Message, header: MessageHeader): boolean {
  return bytesEqual(message.messageId, header.messageId) && message.timestampNs === header.timestampNs &&
    bytesEqual(message.originatorId, header.originatorId) && bytesEqual(message.channelId, header.channelId) &&
    message.sequence === header.sequence && message.keyEpoch === header.keyEpoch &&
    message.contentType === header.contentType && bytesEqual(message.plaintextHash, header.plaintextHash);
}

async function backendCall<T>(operation: () => Promise<T>): Promise<T> {
  try { return await operation(); } catch (error) { throw storageError(error); }
}
function storageError(error: unknown): EpochActivationError {
  if (error instanceof EpochActivationError) return error;
  return new EpochActivationError("storage_error");
}
async function custodyCall<T>(operation: () => Promise<T>): Promise<T> {
  try { return await operation(); }
  catch (error) {
    if (error instanceof EpochActivationError) throw error;
    fail("custody_error");
  }
}
function cryptoCall<T>(operation: () => T): T {
  try { return operation(); }
  catch (error) {
    if (error instanceof EpochActivationError) throw error;
    fail("crypto_error");
  }
}
function compareBytes(left: Uint8Array, right: Uint8Array): number {
  for (let index = 0; index < Math.min(left.length, right.length); index += 1) {
    if (left[index] !== right[index]) return left[index]! - right[index]!;
  }
  return left.length - right.length;
}
function fail(code: EpochActivationErrorCode): never { throw new EpochActivationError(code); }

export { InMemoryKeyCustody };
