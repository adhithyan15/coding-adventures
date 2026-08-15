import { readFileSync } from "node:fs";
import { describe, expect, it } from "vitest";
import {
  ChannelMasterKey,
  OriginatorSigningKey,
  ReceiverKeyPair,
  RotationReceiver,
  planRotation,
} from "@coding-adventures/chief-of-staff-channel-crypto";
import {
  ChannelDefinition,
  channelStateDeserialize,
} from "@coding-adventures/chief-of-staff-channel-store";
import {
  EPOCH_ACTIVATION_ERROR_CODES,
  EPOCH_STATE_CONTENT_TYPE,
  ACTIVATION_PLAN_CONTENT_TYPE,
  activationPlanDeserialize,
  activationPlanRecordKey,
  epochStateDeserialize,
  epochStateSerialize,
  epochActivationSecretErasureCapability,
  prepareRotationCandidate,
} from "../src/index.js";

interface Manifest {
  fixture_format: string;
  spec: string;
  warning: string;
  constants: Record<string, string>;
  test_only_secrets: Record<string, string>;
  state_migrations: Array<{
    name: string; d18s_v1_b64: string; d18s_v2_b64: string;
    active_epoch: string; next_sequence: string;
  }>;
  activation_case: {
    plan_record_key: string; plan_content_type: string; plan_b64: string;
    grant_b64: string[]; receiver_a_new_grant: null;
    receiver_a_retains_epochs: string[]; receiver_b_retains_epochs: string[];
  };
  crash_replay_traces: Array<{ name: string }>;
  race_traces: Array<{ name: string }>;
  stable_error_codes: string[];
  negative_scenarios: Array<{ name: string }>;
  secret_erasure_capability: string;
}

const manifest = JSON.parse(readFileSync(
  new URL("../../../../fixtures/chief-of-staff-channel-epoch-activation/v1/manifest.json", import.meta.url),
  "utf8",
)) as Manifest;
const channelId = fromHex("018f47a09b6c7def923456789abcdef0");

describe("D18T canonical shared fixtures", () => {
  it("locks constants, scenarios, stable errors, and the secret boundary", () => {
    expect(manifest.fixture_format).toBe("D18T-durable-epoch-activation-fixtures-v1");
    expect(manifest.spec).toBe("code/specs/D18T-chief-of-staff-durable-epoch-activation-profile.md");
    expect(manifest.warning).toContain("Never log");
    expect(manifest.constants).toEqual({
      state_magic_ascii: "D18S", state_version: "2",
      plan_magic_ascii: "D18T", plan_version: "1",
      state_content_type: EPOCH_STATE_CONTENT_TYPE,
      plan_content_type: ACTIVATION_PLAN_CONTENT_TYPE,
      max_cas_attempts: "16",
    });
    expect([...EPOCH_ACTIVATION_ERROR_CODES]).toEqual(manifest.stable_error_codes);
    expect(manifest.crash_replay_traces.map(({ name }) => name)).toEqual([
      "after-custody-selection", "after-plan-write", "after-first-grant",
      "after-all-grants", "after-activation-cas",
    ]);
    expect(manifest.race_traces).toHaveLength(4);
    expect(manifest.negative_scenarios).toHaveLength(6);
    expect(manifest.secret_erasure_capability).toBe("guaranteed");
    expect(epochActivationSecretErasureCapability()).toBe("best_effort");
    const text = JSON.stringify(manifest);
    for (const secret of Object.values(manifest.test_only_secrets)) {
      expect(text.split(secret)).toHaveLength(2);
    }
  });

  it("migrates exact D18S v1 vectors by adding only the active epoch", () => {
    expect(manifest.state_migrations.map(({ name }) => name)).toEqual(["no-pending", "pending-d18h"]);
    for (const vector of manifest.state_migrations) {
      const v1 = channelStateDeserialize(fromBase64(vector.d18s_v1_b64), channelId);
      const bytes = fromBase64(vector.d18s_v2_b64);
      const v2 = epochStateDeserialize(bytes, channelId);
      expect(v2.activeEpoch).toBe(BigInt(vector.active_epoch));
      expect(v2.nextSequence).toBe(v1.nextSequence);
      expect(v2.nextSequence).toBe(BigInt(vector.next_sequence));
      if (v1.pendingHeader === undefined) expect(v2.pendingHeader).toBeUndefined();
      else expect(v2.pendingHeader?.equals(v1.pendingHeader)).toBe(true);
      expect(epochStateSerialize(v2)).toEqual(bytes);
    }
  });

  it("consumes the Rust-authored activation plan and revocation history directly", () => {
    const activation = manifest.activation_case;
    const bytes = fromBase64(activation.plan_b64);
    const plan = activationPlanDeserialize(bytes);
    expect(plan.channelId).toEqual(channelId);
    expect(plan.baseEpoch).toBe(0n);
    expect(plan.newEpoch).toBe(1n);
    expect(plan.receivers).toHaveLength(1);
    expect(activationPlanRecordKey(channelId, 1n)).toBe(activation.plan_record_key);
    expect(activation.plan_content_type).toBe(ACTIVATION_PLAN_CONTENT_TYPE);
    expect(activation.grant_b64).toHaveLength(1);
    expect(activation.receiver_a_new_grant).toBeNull();
    expect(activation.receiver_a_retains_epochs).toEqual(["0"]);
    expect(activation.receiver_b_retains_epochs).toEqual(["0", "1"]);
  });

  it("reproduces the Rust-authored D18T plan and D18G grant byte-for-byte", () => {
    const signer = OriginatorSigningKey.fromSeed(fromHex(manifest.test_only_secrets.originator_signing_seed_hex!));
    const receiverAKey = ReceiverKeyPair.fromPrivateKey(fromHex(manifest.test_only_secrets.receiver_a_private_key_hex!));
    const receiverBKey = ReceiverKeyPair.fromPrivateKey(fromHex(manifest.test_only_secrets.receiver_b_private_key_hex!));
    const receiverA = { agentId: utf8("receiver-a"), publicKey: receiverAKey.publicKey };
    const receiverB = { agentId: utf8("receiver-b"), publicKey: receiverBKey.publicKey };
    const definition = new ChannelDefinition({
      channelId,
      originator: { agentId: utf8("originator"), publicKey: signer.publicKey },
      receivers: [receiverA, receiverB],
      createdAtNs: 1_725_000_000_000_000_000n,
      keyEpoch: 0n,
    });
    const rotation = planRotation(
      utf8("originator"), channelId, 0n,
      ChannelMasterKey.fromBytes(fromHex(manifest.test_only_secrets.next_cmk_hex!)),
      [RotationReceiver.withMaterial(
        receiverB.agentId,
        receiverB.publicKey,
        fromHex(manifest.test_only_secrets.ephemeral_private_key_hex!),
        fromHex(manifest.test_only_secrets.wrapping_nonce_hex!),
      )],
      signer,
    );
    const prepared = prepareRotationCandidate(definition, 0n, [receiverB], rotation);
    expect(prepared.publicPreparation.planBytes).toEqual(fromBase64(manifest.activation_case.plan_b64));
    expect(prepared.publicPreparation.grants).toEqual(
      manifest.activation_case.grant_b64.map(fromBase64),
    );
    prepared.destroy();
    signer.destroy(); receiverAKey.destroy(); receiverBKey.destroy();
  });

  it("rejects malformed state and non-canonical plan records", () => {
    const state = fromBase64(manifest.state_migrations[0]!.d18s_v2_b64);
    expect(() => epochStateDeserialize(state.slice(0, -1), channelId)).toThrowError("corrupt_record");
    const badVersion = state.slice(); badVersion[4] = 3;
    expect(() => epochStateDeserialize(badVersion, channelId)).toThrowError("corrupt_record");

    const plan = fromBase64(manifest.activation_case.plan_b64);
    const trailing = new Uint8Array(plan.length + 1); trailing.set(plan);
    expect(() => activationPlanDeserialize(trailing)).toThrowError("corrupt_record");
    const wrongOrder = new Uint8Array(41 + 128);
    wrongOrder.set(plan.slice(0, 41));
    new DataView(wrongOrder.buffer).setUint32(37, 2);
    wrongOrder.set(new Uint8Array(32).fill(2), 41);
    wrongOrder.set(new Uint8Array(32).fill(4), 73);
    wrongOrder.set(new Uint8Array(32).fill(1), 105);
    wrongOrder.set(new Uint8Array(32).fill(3), 137);
    expect(() => activationPlanDeserialize(wrongOrder)).toThrowError("corrupt_record");
  });
});

function fromHex(value: string): Uint8Array {
  return Uint8Array.from(value.match(/../g) ?? [], (pair) => Number.parseInt(pair, 16));
}
function fromBase64(value: string): Uint8Array { return Uint8Array.from(Buffer.from(value, "base64")); }
function utf8(value: string): Uint8Array { return new TextEncoder().encode(value); }
