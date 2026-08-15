/** Injected atomic originator-key custody for D18T. */

import { ChannelMasterKey } from "@coding-adventures/chief-of-staff-channel-crypto";
import { bytesEqual } from "@coding-adventures/chief-of-staff-channel-store";

export type CustodySelection = "selected" | "idempotent" | "conflict";

export class CustodyError extends Error {
  readonly code = "custody_error" as const;
  constructor() { super("custody_error"); this.name = "CustodyError"; }
}

/** Opaque, redacted reference to one retained epoch key. */
export class EpochKeyHandle {
  readonly #channelId: Uint8Array;
  readonly epoch: bigint;
  constructor(channelId: Uint8Array, epoch: bigint) {
    this.#channelId = channelId.slice(); this.epoch = epoch; Object.freeze(this);
  }
  get channelId(): Uint8Array { return this.#channelId.slice(); }
  toString(): string { return "EpochKeyHandle([REDACTED])"; }
  toJSON(): string { return "EpochKeyHandle([REDACTED])"; }
}

/** Exact secret-free recovery bundle retained beside a prepared CMK. */
export class PublicPreparation {
  readonly #channelId: Uint8Array;
  readonly baseEpoch: bigint;
  readonly newEpoch: bigint;
  readonly #planBytes: Uint8Array;
  readonly #grants: readonly Uint8Array[];

  constructor(
    channelId: Uint8Array,
    baseEpoch: bigint,
    newEpoch: bigint,
    planBytes: Uint8Array,
    grants: readonly Uint8Array[],
  ) {
    this.#channelId = channelId.slice();
    this.baseEpoch = baseEpoch;
    this.newEpoch = newEpoch;
    this.#planBytes = planBytes.slice();
    this.#grants = Object.freeze(grants.map((grant) => grant.slice()));
    Object.freeze(this);
  }

  get channelId(): Uint8Array { return this.#channelId.slice(); }
  get planBytes(): Uint8Array { return this.#planBytes.slice(); }
  get grants(): readonly Uint8Array[] { return this.#grants.map((grant) => grant.slice()); }

  equals(other: PublicPreparation): boolean {
    return bytesEqual(this.#channelId, other.#channelId) &&
      this.baseEpoch === other.baseEpoch && this.newEpoch === other.newEpoch &&
      bytesEqual(this.#planBytes, other.#planBytes) &&
      this.#grants.length === other.#grants.length &&
      this.#grants.every((grant, index) => bytesEqual(grant, other.#grants[index]!));
  }

  clone(): PublicPreparation {
    return new PublicPreparation(this.#channelId, this.baseEpoch, this.newEpoch, this.#planBytes, this.#grants);
  }
}

/** One indivisible candidate offered to custody. */
export class PreparedEpoch {
  readonly publicPreparation: PublicPreparation;
  readonly #cmk: ChannelMasterKey;
  constructor(publicPreparation: PublicPreparation, cmk: ChannelMasterKey) {
    this.publicPreparation = publicPreparation.clone();
    this.#cmk = cmk.clone();
  }
  cloneCmk(): ChannelMasterKey { return this.#cmk.clone(); }
  destroy(): void { this.#cmk.destroy(); }
  toString(): string { return "PreparedEpoch([REDACTED])"; }
  toJSON(): string { return "PreparedEpoch([REDACTED])"; }
}

/** Atomic, restart-safe custody boundary. Production implementations must be durable. */
export interface OriginatorKeyCustody {
  readonly durable: boolean;
  importActiveIfAbsent(
    channelId: Uint8Array, epoch: bigint, cmk: ChannelMasterKey,
  ): Promise<CustodySelection>;
  resolveHandle(channelId: Uint8Array, epoch: bigint): Promise<EpochKeyHandle | undefined>;
  prepareIfAbsent(prepared: PreparedEpoch): Promise<CustodySelection>;
  loadPreparation(channelId: Uint8Array, newEpoch: bigint): Promise<PublicPreparation | undefined>;
  withKey<T>(
    handle: EpochKeyHandle,
    operation: (cmk: ChannelMasterKey) => T | Promise<T>,
  ): Promise<T>;
  destroyChannel(channelId: Uint8Array): Promise<void>;
}

/** Deterministic, explicitly non-durable custody for conformance tests only. */
export class InMemoryKeyCustody implements OriginatorKeyCustody {
  readonly durable = false;
  readonly #keys = new Map<string, ChannelMasterKey>();
  readonly #preparations = new Map<string, PublicPreparation>();

  async importActiveIfAbsent(
    channelId: Uint8Array, epoch: bigint, cmk: ChannelMasterKey,
  ): Promise<CustodySelection> {
    const key = slot(channelId, epoch);
    const current = this.#keys.get(key);
    if (current === undefined) { this.#keys.set(key, cmk.clone()); return "selected"; }
    return sameCmk(current, cmk) ? "idempotent" : "conflict";
  }

  async resolveHandle(channelId: Uint8Array, epoch: bigint): Promise<EpochKeyHandle | undefined> {
    return this.#keys.has(slot(channelId, epoch)) ? new EpochKeyHandle(channelId, epoch) : undefined;
  }

  async prepareIfAbsent(prepared: PreparedEpoch): Promise<CustodySelection> {
    const publicPreparation = prepared.publicPreparation;
    const key = slot(publicPreparation.channelId, publicPreparation.newEpoch);
    const currentPublic = this.#preparations.get(key);
    const currentCmk = this.#keys.get(key);
    if (currentPublic === undefined && currentCmk === undefined) {
      const cmk = prepared.cloneCmk();
      this.#preparations.set(key, publicPreparation.clone());
      this.#keys.set(key, cmk);
      return "selected";
    }
    if (currentPublic === undefined || currentCmk === undefined || !currentPublic.equals(publicPreparation)) {
      return "conflict";
    }
    const candidate = prepared.cloneCmk();
    try { return sameCmk(currentCmk, candidate) ? "idempotent" : "conflict"; }
    finally { candidate.destroy(); }
  }

  async loadPreparation(channelId: Uint8Array, newEpoch: bigint): Promise<PublicPreparation | undefined> {
    return this.#preparations.get(slot(channelId, newEpoch))?.clone();
  }

  async withKey<T>(
    handle: EpochKeyHandle,
    operation: (cmk: ChannelMasterKey) => T | Promise<T>,
  ): Promise<T> {
    const cmk = this.#keys.get(slot(handle.channelId, handle.epoch));
    if (cmk === undefined) throw new CustodyError();
    const transient = cmk.clone();
    try { return await operation(transient); } finally { transient.destroy(); }
  }

  async destroyChannel(channelId: Uint8Array): Promise<void> {
    const prefix = `${hex(channelId)}:`;
    for (const [key, cmk] of this.#keys) {
      if (key.startsWith(prefix)) { cmk.destroy(); this.#keys.delete(key); }
    }
    for (const key of this.#preparations.keys()) {
      if (key.startsWith(prefix)) this.#preparations.delete(key);
    }
  }

  get retainedKeyCount(): number { return this.#keys.size; }
}

function sameCmk(left: ChannelMasterKey, right: ChannelMasterKey): boolean {
  const leftBytes = left.bytes;
  const rightBytes = right.bytes;
  try { return bytesEqual(leftBytes, rightBytes); }
  finally { leftBytes.fill(0); rightBytes.fill(0); }
}
function slot(channelId: Uint8Array, epoch: bigint): string { return `${hex(channelId)}:${epoch}`; }
function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
