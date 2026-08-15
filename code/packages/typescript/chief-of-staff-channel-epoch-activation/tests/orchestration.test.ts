import { describe, expect, it } from "vitest";
import {
  ChannelMasterKey,
  OriginatorSigningKey,
  ReceiverKeyPair,
  RotationReceiver,
  grantDeserialize,
  messageVerify,
  planRotation,
} from "@coding-adventures/chief-of-staff-channel-crypto";
import {
  CHANNEL_GRANT_CONTENT_TYPE,
  CHANNEL_STORAGE_NAMESPACE,
  ChannelDefinition,
  ChannelDefinitionStore,
  ChannelStore,
  MemoryChannelStorage,
  StorageConflictError,
  type ChannelStorageBackend,
  type ReceiverIdentity,
  type StorageListOptions,
  type StoragePage,
  type StoragePut,
  type StorageRecord,
  keyGrantRecordKey,
  sequenceStateRecordKey,
} from "@coding-adventures/chief-of-staff-channel-store";
import {
  ACTIVATION_PLAN_CONTENT_TYPE,
  EPOCH_STATE_CONTENT_TYPE,
  EpochActivationError,
  EpochActivationStore,
  InMemoryKeyCustody,
  activationPlanRecordKey,
  prepareRotationCandidate,
} from "../src/index.js";

const CURRENT_CMK = new Uint8Array(32).fill(0x21);
const NEXT_CMK = new Uint8Array(32).fill(0x31);

class Fixture {
  readonly backend = new MemoryChannelStorage();
  readonly custody = new InMemoryKeyCustody();
  readonly signer = OriginatorSigningKey.fromSeed(new Uint8Array(32).fill(0x11));
  readonly receiverAKey = ReceiverKeyPair.fromPrivateKey(new Uint8Array(32).fill(0x41));
  readonly receiverBKey = ReceiverKeyPair.fromPrivateKey(new Uint8Array(32).fill(0x42));
  readonly receiverA: ReceiverIdentity = {
    agentId: utf8("receiver-a"), publicKey: this.receiverAKey.publicKey,
  };
  readonly receiverB: ReceiverIdentity = {
    agentId: utf8("receiver-b"), publicKey: this.receiverBKey.publicKey,
  };
  readonly definition = new ChannelDefinition({
    channelId: channelId(),
    originator: { agentId: utf8("originator"), publicKey: this.signer.publicKey },
    receivers: [this.receiverA, this.receiverB],
    createdAtNs: 1_725_000_000_000_000_000n,
    keyEpoch: 0n,
  });

  async initialize(): Promise<this> {
    await new ChannelDefinitionStore(this.backend).create(this.definition);
    return this;
  }
  async store(): Promise<EpochActivationStore> {
    return EpochActivationStore.openForTesting(this.backend, this.custody, channelId());
  }
  rotation(cmk = NEXT_CMK, material = 0x51) {
    return planRotation(
      utf8("originator"), channelId(), 0n, ChannelMasterKey.fromBytes(cmk),
      [RotationReceiver.withMaterial(
        this.receiverB.agentId, this.receiverB.publicKey,
        new Uint8Array(32).fill(material), new Uint8Array(24).fill(material + 0x10),
      )],
      this.signer,
    );
  }
  twoReceiverRotation() {
    return planRotation(
      utf8("originator"), channelId(), 0n, ChannelMasterKey.fromBytes(NEXT_CMK),
      [
        RotationReceiver.withMaterial(this.receiverB.agentId, this.receiverB.publicKey, new Uint8Array(32).fill(0x52), new Uint8Array(24).fill(0x62)),
        RotationReceiver.withMaterial(this.receiverA.agentId, this.receiverA.publicKey, new Uint8Array(32).fill(0x51), new Uint8Array(24).fill(0x61)),
      ],
      this.signer,
    );
  }
}

describe("D18T orchestration", () => {
  it("creates a new D18T-aware channel only after custody owns its initial CMK", async () => {
    const fixture = new Fixture();
    const store = await fixture.store();
    const state = await store.createEpochChannel(
      fixture.definition, ChannelMasterKey.fromBytes(CURRENT_CMK),
    );
    expect(state.activeEpoch).toBe(0n);
    expect(state.nextSequence).toBe(0n);
    expect(state.pendingHeader).toBeUndefined();
    expect(fixture.custody.retainedKeyCount).toBe(1);
    const record = await fixture.backend.get(
      CHANNEL_STORAGE_NAMESPACE, sequenceStateRecordKey(channelId()),
    );
    expect(record?.contentType).toBe(EPOCH_STATE_CONTENT_TYPE);
    expect((await store.createEpochChannel(
      fixture.definition, ChannelMasterKey.fromBytes(CURRENT_CMK),
    )).activeEpoch).toBe(0n);
    await expect(store.createEpochChannel(
      fixture.definition, ChannelMasterKey.fromBytes(new Uint8Array(32).fill(0x99)),
    )).rejects.toMatchObject({ code: "conflicting_active_key" });
  });

  it("migrates a legacy pending state without changing its D18H", async () => {
    const fixture = await new Fixture().initialize();
    const legacy = new ChannelStore(fixture.backend, channelId());
    const pending = await legacy.reserveAppend({
      messageId: messageId(1), timestampNs: 11n, originatorId: utf8("originator"),
      keyEpoch: 0n, contentType: "text/plain",
    }, utf8("already reserved"));
    const store = await fixture.store();
    const state = await store.migrateEpochState(
      fixture.definition, ChannelMasterKey.fromBytes(CURRENT_CMK),
    );
    expect(state.activeEpoch).toBe(0n);
    expect(state.nextSequence).toBe(1n);
    expect(state.pendingHeader?.equals(pending)).toBe(true);
    expect((await store.migrateEpochState(fixture.definition)).equals(state)).toBe(true);
  });

  it("selects, replays, activates, and publishes with the authoritative epoch idempotently", async () => {
    const fixture = await new Fixture().initialize();
    const store = await fixture.store();
    await store.migrateEpochState(fixture.definition, ChannelMasterKey.fromBytes(CURRENT_CMK));
    expect(await store.prepareRotation(fixture.definition, [fixture.receiverB], fixture.rotation())).toBe("prepared");
    expect(await store.prepareRotation(fixture.definition, [fixture.receiverB], fixture.rotation())).toBe("idempotent");
    expect(await store.activatePreparedEpoch(fixture.definition, 1n)).toBe("activated");
    expect(await store.activatePreparedEpoch(fixture.definition, 1n)).toBe("idempotent");
    const reservation = await store.reservePublishUsingActiveEpoch(fixture.definition, {
      messageId: messageId(2), timestampNs: 12n, originatorId: utf8("originator"),
      contentType: "text/plain", keyEpoch: 1n,
    }, utf8("epoch one"));
    expect(reservation.header.keyEpoch).toBe(1n);
    expect(reservation.keyHandle.toString()).toBe("EpochKeyHandle([REDACTED])");
    const message = await store.commitReserved(
      fixture.definition, reservation, utf8("epoch one"), fixture.signer,
    );
    expect(messageVerify(message, fixture.signer.publicKey, NEXT_CMK)).toEqual(utf8("epoch one"));
    expect((await store.commitReserved(fixture.definition, reservation, utf8("epoch one"), fixture.signer)).sequence).toBe(0n);
    expect((await store.state()).pendingHeader).toBeUndefined();
    await expect(store.reservePublishUsingActiveEpoch(fixture.definition, {
      messageId: messageId(3), timestampNs: 13n, originatorId: utf8("originator"),
      contentType: "text/plain", keyEpoch: 0n,
    }, utf8("old"))).rejects.toMatchObject({ code: "unactivated_epoch" });
  });

  it("serializes publication and activation through one shared state CAS", async () => {
    const fixture = await new Fixture().initialize();
    const store = await fixture.store();
    await store.migrateEpochState(fixture.definition, ChannelMasterKey.fromBytes(CURRENT_CMK));
    await store.prepareRotation(fixture.definition, [fixture.receiverB], fixture.rotation());
    const reservation = await store.reservePublishUsingActiveEpoch(fixture.definition, {
      messageId: messageId(4), timestampNs: 14n, originatorId: utf8("originator"), contentType: "text/plain",
    }, utf8("epoch zero first"));
    expect(reservation.header.keyEpoch).toBe(0n);
    await expect(store.activatePreparedEpoch(fixture.definition, 1n)).rejects.toMatchObject({ code: "pending_append" });
    await store.commitReserved(fixture.definition, reservation, utf8("epoch zero first"), fixture.signer);
    expect(await store.activatePreparedEpoch(fixture.definition, 1n)).toBe("activated");

    const abandoned = await store.reservePublishUsingActiveEpoch(fixture.definition, {
      messageId: messageId(5), timestampNs: 15n, originatorId: utf8("originator"), contentType: "text/plain",
    }, utf8("abandon"));
    expect((await store.abandonPending())?.equals(abandoned.header)).toBe(true);
    expect(await store.abandonPending()).toBeUndefined();
  });

  it("replays every public-record crash boundary from custody's selected bundle", async () => {
    for (let recordsWritten = 0; recordsWritten <= 3; recordsWritten += 1) {
      const fixture = await new Fixture().initialize();
      const store = await fixture.store();
      await store.migrateEpochState(fixture.definition, ChannelMasterKey.fromBytes(CURRENT_CMK));
      const prepared = prepareRotationCandidate(
        fixture.definition, 0n, [fixture.receiverA, fixture.receiverB], fixture.twoReceiverRotation(),
      );
      const publicPreparation = prepared.publicPreparation.clone();
      expect(await fixture.custody.prepareIfAbsent(prepared)).toBe("selected");
      prepared.destroy();
      if (recordsWritten >= 1) {
        await fixture.backend.put({
          namespace: CHANNEL_STORAGE_NAMESPACE,
          key: activationPlanRecordKey(channelId(), 1n),
          contentType: ACTIVATION_PLAN_CONTENT_TYPE, metadata: {},
          body: publicPreparation.planBytes, ifAbsent: true,
        });
      }
      for (const bytes of publicPreparation.grants.slice(0, Math.max(0, recordsWritten - 1))) {
        const grant = grantDeserialize(bytes);
        await fixture.backend.put({
          namespace: CHANNEL_STORAGE_NAMESPACE,
          key: keyGrantRecordKey(channelId(), grant.keyEpoch, grant.receiverId),
          contentType: CHANNEL_GRANT_CONTENT_TYPE, metadata: {}, body: bytes, ifAbsent: true,
        });
      }
      expect(await (await fixture.store()).recoverPreparation(fixture.definition, 1n)).toBe("idempotent");
      expect(await (await fixture.store()).activatePreparedEpoch(fixture.definition, 1n)).toBe("activated");
    }
  });

  it("selects exactly one candidate and rejects invalid rosters before custody", async () => {
    const fixture = await new Fixture().initialize();
    const store = await fixture.store();
    await store.migrateEpochState(fixture.definition, ChannelMasterKey.fromBytes(CURRENT_CMK));
    await store.prepareRotation(fixture.definition, [fixture.receiverB], fixture.rotation());
    await expect(store.prepareRotation(
      fixture.definition, [fixture.receiverB], fixture.rotation(new Uint8Array(32).fill(0x32), 0x52),
    )).rejects.toMatchObject({ code: "conflicting_preparation" });

    const invalid = await new Fixture().initialize();
    const invalidStore = await invalid.store();
    await invalidStore.migrateEpochState(invalid.definition, ChannelMasterKey.fromBytes(CURRENT_CMK));
    await expect(invalidStore.prepareRotation(invalid.definition, [invalid.receiverA], invalid.rotation()))
      .rejects.toMatchObject({ code: "invalid_plan" });
  });

  it("rejects non-durable production custody and erases secrets on logical destruction", async () => {
    const fixture = await new Fixture().initialize();
    await expect(EpochActivationStore.open(fixture.backend, fixture.custody, channelId()))
      .rejects.toMatchObject({ code: "custody_error" });
    const store = await fixture.store();
    await store.migrateEpochState(fixture.definition, ChannelMasterKey.fromBytes(CURRENT_CMK));
    await store.prepareRotation(fixture.definition, [fixture.receiverB], fixture.rotation());
    expect(fixture.custody.retainedKeyCount).toBe(2);
    const destroyed = await new ChannelDefinitionStore(fixture.backend).destroy(channelId());
    await store.applyDestruction(destroyed);
    expect(fixture.custody.retainedKeyCount).toBe(0);
    expect((await store.state()).activeEpoch).toBe(0n);
    await expect(store.activatePreparedEpoch(destroyed, 1n)).rejects.toMatchObject({ code: "channel_destroyed" });
  });

  it("fails closed for missing custody, corrupt public records, and epoch order", async () => {
    const missing = await new Fixture().initialize();
    const missingStore = await missing.store();
    await missingStore.migrateEpochState(missing.definition, ChannelMasterKey.fromBytes(CURRENT_CMK));
    await expect(missingStore.activatePreparedEpoch(missing.definition, 1n))
      .rejects.toMatchObject({ code: "preparation_missing" });
    await expect(missingStore.migrateEpochState(missing.definition))
      .resolves.toMatchObject({ activeEpoch: 0n });

    const corrupt = await new Fixture().initialize();
    const corruptStore = await corrupt.store();
    await corruptStore.migrateEpochState(corrupt.definition, ChannelMasterKey.fromBytes(CURRENT_CMK));
    const prepared = prepareRotationCandidate(corrupt.definition, 0n, [corrupt.receiverB], corrupt.rotation());
    const publicPreparation = prepared.publicPreparation.clone();
    await corrupt.custody.prepareIfAbsent(prepared); prepared.destroy();
    await corrupt.backend.put({
      namespace: CHANNEL_STORAGE_NAMESPACE, key: activationPlanRecordKey(channelId(), 1n),
      contentType: "application/octet-stream", metadata: {}, body: publicPreparation.planBytes,
      ifAbsent: true,
    });
    await expect(corruptStore.recoverPreparation(corrupt.definition, 1n))
      .rejects.toMatchObject({ code: "corrupt_record" });

    const advanced = await new Fixture().initialize();
    const advancedStore = await advanced.store();
    await advancedStore.migrateEpochState(advanced.definition, ChannelMasterKey.fromBytes(CURRENT_CMK));
    await advancedStore.prepareRotation(advanced.definition, [advanced.receiverB], advanced.rotation());
    await advancedStore.activatePreparedEpoch(advanced.definition, 1n);
    await expect(advancedStore.recoverPreparation(advanced.definition, 0n))
      .rejects.toMatchObject({ code: "decreasing_epoch" });
  });

  it("returns concurrent_update after exactly sixteen activation CAS conflicts", async () => {
    const backend = new StateCasConflictBackend();
    const fixture = new Fixture();
    await new ChannelDefinitionStore(backend).create(fixture.definition);
    const custody = new InMemoryKeyCustody();
    const store = await EpochActivationStore.openForTesting(backend, custody, channelId());
    await store.migrateEpochState(fixture.definition, ChannelMasterKey.fromBytes(CURRENT_CMK));
    await store.prepareRotation(fixture.definition, [fixture.receiverB], fixture.rotation());
    backend.rejectStateCas = true;
    await expect(store.activatePreparedEpoch(fixture.definition, 1n))
      .rejects.toMatchObject({ code: "concurrent_update" });
    expect(backend.rejected).toBe(16);
    expect((await store.state()).activeEpoch).toBe(0n);
  });

  it("reports epoch exhaustion before accepting an irrelevant candidate", async () => {
    const fixture = new Fixture();
    const definition = new ChannelDefinition({
      channelId: channelId(), originator: fixture.definition.originator,
      receivers: [fixture.receiverB], createdAtNs: 1n, keyEpoch: (1n << 64n) - 1n,
    });
    await new ChannelDefinitionStore(fixture.backend).create(definition);
    const store = await fixture.store();
    await store.migrateEpochState(definition, ChannelMasterKey.fromBytes(CURRENT_CMK));
    await expect(store.prepareRotation(definition, [fixture.receiverB], fixture.rotation()))
      .rejects.toMatchObject({ code: "epoch_exhausted" });
  });
});

class StateCasConflictBackend implements ChannelStorageBackend {
  readonly inner = new MemoryChannelStorage();
  rejectStateCas = false;
  rejected = 0;
  initialize(): Promise<void> { return this.inner.initialize(); }
  get(namespace: string, key: string): Promise<StorageRecord | undefined> { return this.inner.get(namespace, key); }
  async put(input: StoragePut): Promise<StorageRecord> {
    if (this.rejectStateCas && input.key === sequenceStateRecordKey(channelId()) && input.ifRevision !== undefined) {
      this.rejected += 1; throw new StorageConflictError();
    }
    return this.inner.put(input);
  }
  list(namespace: string, options: StorageListOptions): Promise<StoragePage> { return this.inner.list(namespace, options); }
}

function channelId(): Uint8Array { return fromHex("018f47a09b6c7def923456789abcdef0"); }
function messageId(byte: number): Uint8Array {
  const bytes = new Uint8Array(16).fill(byte); bytes[6] = 0x70 | (byte & 0x0f); bytes[8] = 0x80 | (byte & 0x3f); return bytes;
}
function utf8(value: string): Uint8Array { return new TextEncoder().encode(value); }
function fromHex(value: string): Uint8Array {
  return Uint8Array.from(value.match(/../g) ?? [], (pair) => Number.parseInt(pair, 16));
}

void EpochActivationError;
