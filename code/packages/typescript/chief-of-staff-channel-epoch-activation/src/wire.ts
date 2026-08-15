/** Exact D18S v2 state and D18T v1 activation-plan codecs. */

import {
  MAX_PENDING_HEADER_BYTES,
  MAX_U64,
  MessageHeader,
  bytesEqual,
  messageHeaderDeserialize,
  messageHeaderSerialize,
  validateUuidV7,
} from "@coding-adventures/chief-of-staff-channel-store";

export const EPOCH_STATE_CONTENT_TYPE =
  "application/vnd.coding-adventures.chief-channel-state-v2";
export const ACTIVATION_PLAN_CONTENT_TYPE =
  "application/vnd.coding-adventures.chief-channel-epoch-activation-v1";
export const MAX_PLAN_RECEIVERS = 1024;

export class EpochWireError extends Error {
  readonly code = "corrupt_record" as const;
  constructor() { super("corrupt_record"); this.name = "EpochWireError"; }
}

export interface EpochStateParts {
  readonly activeEpoch: bigint;
  readonly nextSequence: bigint;
  readonly pendingHeader?: MessageHeader;
}

export class EpochState {
  readonly activeEpoch: bigint;
  readonly nextSequence: bigint;
  readonly #pendingHeader?: MessageHeader;

  constructor(channelId: Uint8Array, parts: EpochStateParts) {
    requireU64(parts.activeEpoch);
    requireU64(parts.nextSequence);
    if (parts.pendingHeader !== undefined) {
      const header = parts.pendingHeader;
      if (
        !bytesEqual(header.channelId, channelId) ||
        header.sequence === MAX_U64 ||
        header.sequence + 1n !== parts.nextSequence ||
        header.keyEpoch !== parts.activeEpoch
      ) fail();
    }
    this.activeEpoch = parts.activeEpoch;
    this.nextSequence = parts.nextSequence;
    this.#pendingHeader = parts.pendingHeader;
    Object.freeze(this);
  }

  get pendingHeader(): MessageHeader | undefined { return this.#pendingHeader; }

  withActiveEpoch(channelId: Uint8Array, activeEpoch: bigint): EpochState {
    return new EpochState(channelId, {
      activeEpoch, nextSequence: this.nextSequence, pendingHeader: this.#pendingHeader,
    });
  }

  withPending(
    channelId: Uint8Array,
    nextSequence: bigint,
    pendingHeader?: MessageHeader,
  ): EpochState {
    return new EpochState(channelId, { activeEpoch: this.activeEpoch, nextSequence, pendingHeader });
  }

  equals(other: EpochState): boolean {
    return this.activeEpoch === other.activeEpoch &&
      this.nextSequence === other.nextSequence &&
      ((this.#pendingHeader === undefined && other.#pendingHeader === undefined) ||
        (this.#pendingHeader !== undefined && other.#pendingHeader !== undefined &&
          this.#pendingHeader.equals(other.#pendingHeader)));
  }
}

export class ActivationPlanEntry {
  readonly #receiverIdHash: Uint8Array;
  readonly #grantHash: Uint8Array;

  constructor(receiverIdHash: Uint8Array, grantHash: Uint8Array) {
    requireLength(receiverIdHash, 32);
    requireLength(grantHash, 32);
    this.#receiverIdHash = receiverIdHash.slice();
    this.#grantHash = grantHash.slice();
    Object.freeze(this);
  }

  get receiverIdHash(): Uint8Array { return this.#receiverIdHash.slice(); }
  get grantHash(): Uint8Array { return this.#grantHash.slice(); }
}

export class ActivationPlan {
  readonly #channelId: Uint8Array;
  readonly baseEpoch: bigint;
  readonly newEpoch: bigint;
  readonly #receivers: readonly ActivationPlanEntry[];

  constructor(
    channelId: Uint8Array,
    baseEpoch: bigint,
    newEpoch: bigint,
    receivers: readonly ActivationPlanEntry[],
  ) {
    try { validateUuidV7(channelId); } catch { fail(); }
    requireU64(baseEpoch);
    requireU64(newEpoch);
    if (baseEpoch === MAX_U64 || newEpoch !== baseEpoch + 1n) fail();
    if (receivers.length < 1 || receivers.length > MAX_PLAN_RECEIVERS) fail();
    const ordered = [...receivers].sort((left, right) => compare(left.receiverIdHash, right.receiverIdHash));
    for (let index = 1; index < ordered.length; index += 1) {
      const prior = ordered[index - 1]!;
      const current = ordered[index]!;
      if (bytesEqual(prior.receiverIdHash, current.receiverIdHash) ||
          bytesEqual(prior.grantHash, current.grantHash)) fail();
    }
    for (let left = 0; left < ordered.length; left += 1) {
      for (let right = left + 1; right < ordered.length; right += 1) {
        if (bytesEqual(ordered[left]!.grantHash, ordered[right]!.grantHash)) fail();
      }
    }
    this.#channelId = channelId.slice();
    this.baseEpoch = baseEpoch;
    this.newEpoch = newEpoch;
    this.#receivers = Object.freeze(ordered);
    Object.freeze(this);
  }

  get channelId(): Uint8Array { return this.#channelId.slice(); }
  get receivers(): readonly ActivationPlanEntry[] { return this.#receivers; }

  equals(other: ActivationPlan): boolean {
    return bytesEqual(activationPlanSerialize(this), activationPlanSerialize(other));
  }
}

export function epochStateSerialize(state: EpochState): Uint8Array {
  const writer = new Writer();
  writer.ascii("D18S").u8(2).u64(state.activeEpoch).u64(state.nextSequence);
  if (state.pendingHeader === undefined) return writer.u8(0).finish();
  const header = messageHeaderSerialize(state.pendingHeader);
  if (header.length > MAX_PENDING_HEADER_BYTES) fail();
  return writer.u8(1).u32(header.length).bytes(header).finish();
}

export function epochStateDeserialize(bytes: Uint8Array, channelId: Uint8Array): EpochState {
  try {
    const reader = new Reader(bytes);
    reader.magic("D18S");
    if (reader.u8() !== 2) fail();
    const activeEpoch = reader.u64();
    const nextSequence = reader.u64();
    const flag = reader.u8();
    let pendingHeader: MessageHeader | undefined;
    if (flag === 1) {
      const length = reader.u32();
      if (length > MAX_PENDING_HEADER_BYTES) fail();
      try { pendingHeader = messageHeaderDeserialize(reader.bytes(length)); } catch { fail(); }
    } else if (flag !== 0) fail();
    reader.finish();
    return new EpochState(channelId, { activeEpoch, nextSequence, pendingHeader });
  } catch (error) {
    if (error instanceof EpochWireError) throw error;
    fail();
  }
}

export function activationPlanSerialize(plan: ActivationPlan): Uint8Array {
  const writer = new Writer();
  writer.ascii("D18T").u8(1).bytes(plan.channelId)
    .u64(plan.baseEpoch).u64(plan.newEpoch).u32(plan.receivers.length);
  for (const receiver of plan.receivers) {
    writer.bytes(receiver.receiverIdHash).bytes(receiver.grantHash);
  }
  return writer.finish();
}

export function activationPlanDeserialize(bytes: Uint8Array): ActivationPlan {
  try {
    const reader = new Reader(bytes);
    reader.magic("D18T");
    if (reader.u8() !== 1) fail();
    const channelId = reader.bytes(16);
    const baseEpoch = reader.u64();
    const newEpoch = reader.u64();
    const count = reader.u32();
    if (count < 1 || count > MAX_PLAN_RECEIVERS) fail();
    const receivers: ActivationPlanEntry[] = [];
    for (let index = 0; index < count; index += 1) {
      receivers.push(new ActivationPlanEntry(reader.bytes(32), reader.bytes(32)));
    }
    reader.finish();
    for (let index = 1; index < receivers.length; index += 1) {
      if (compare(receivers[index - 1]!.receiverIdHash, receivers[index]!.receiverIdHash) >= 0) fail();
    }
    const plan = new ActivationPlan(channelId, baseEpoch, newEpoch, receivers);
    return plan;
  } catch (error) {
    if (error instanceof EpochWireError) throw error;
    fail();
  }
}

export function activationPlanRecordKey(channelId: Uint8Array, newEpoch: bigint): string {
  requireLength(channelId, 16);
  requireU64(newEpoch);
  return `${hex(channelId)}/epochs/${newEpoch.toString().padStart(20, "0")}/activation`;
}

class Writer {
  readonly #parts: Uint8Array[] = [];
  ascii(value: string): this { return this.bytes(new TextEncoder().encode(value)); }
  u8(value: number): this { this.#parts.push(Uint8Array.of(value)); return this; }
  u32(value: number): this {
    if (!Number.isSafeInteger(value) || value < 0 || value > 0xffff_ffff) fail();
    const bytes = new Uint8Array(4); new DataView(bytes.buffer).setUint32(0, value); return this.bytes(bytes);
  }
  u64(value: bigint): this {
    requireU64(value); const bytes = new Uint8Array(8); new DataView(bytes.buffer).setBigUint64(0, value); return this.bytes(bytes);
  }
  bytes(value: Uint8Array): this { this.#parts.push(value.slice()); return this; }
  finish(): Uint8Array {
    const length = this.#parts.reduce((sum, part) => sum + part.length, 0);
    const output = new Uint8Array(length); let offset = 0;
    for (const part of this.#parts) { output.set(part, offset); offset += part.length; }
    return output;
  }
}

class Reader {
  #offset = 0;
  constructor(readonly source: Uint8Array) {}
  bytes(length: number): Uint8Array {
    if (length < 0 || this.#offset + length > this.source.length) fail();
    const result = this.source.slice(this.#offset, this.#offset + length); this.#offset += length; return result;
  }
  magic(value: string): void { if (!bytesEqual(this.bytes(4), new TextEncoder().encode(value))) fail(); }
  u8(): number { return this.bytes(1)[0]!; }
  u32(): number { const bytes = this.bytes(4); return new DataView(bytes.buffer, bytes.byteOffset, 4).getUint32(0); }
  u64(): bigint { const bytes = this.bytes(8); return new DataView(bytes.buffer, bytes.byteOffset, 8).getBigUint64(0); }
  finish(): void { if (this.#offset !== this.source.length) fail(); }
}

function requireLength(value: Uint8Array, length: number): void {
  if (!(value instanceof Uint8Array) || value.length !== length) fail();
}
function requireU64(value: bigint): void { if (value < 0n || value > MAX_U64) fail(); }
function compare(left: Uint8Array, right: Uint8Array): number {
  for (let index = 0; index < Math.min(left.length, right.length); index += 1) {
    if (left[index] !== right[index]) return left[index]! - right[index]!;
  }
  return left.length - right.length;
}
function hex(bytes: Uint8Array): string {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, "0")).join("");
}
function fail(): never { throw new EpochWireError(); }
